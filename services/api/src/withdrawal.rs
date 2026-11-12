//! 개념: 철회 목록과 세대 비교.
//! 목적: 철회한 글이 다시 공개되지 않게 하고, 오래된 작업의 성공 기록이 최신 세대를 덮지 않게 한다.
//! 사용처: `POST /api/admin/entries/{id}/unpublish`, 발행 확인, 공개 파일.

use crate::admin::{audit, authorize, conflict, invalid, json_response, not_found, unavailable};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// 개념: 발행 성공을 받아도 되는 조건.
/// 목적: 철회된 글과, 이미 더 나중에 확인된 세대가 있는 작업을 성공으로 닫지 않는다.
/// 사용처: `commit_success`.
pub fn activation_decision(newer_generation_already_active: bool, entry_denied: bool) -> &'static str {
    if entry_denied {
        "withheld"
    } else if newer_generation_already_active {
        "stale"
    } else {
        "activate"
    }
}

/// 개념: 과거 세대로 돌아갈 수 있는 조건.
/// 목적: 철회한 글이 들어 있는 예전 릴리스를 다시 현재 포인터로 만들지 않는다.
/// 사용처: 릴리스 복구.
pub fn rollback_decision(entry_denied: bool) -> &'static str {
    if entry_denied { "refuse" } else { "restore" }
}

/// 개념: 공개 응답에 글을 넣어도 되는지.
/// 목적: 검색 색인을 다시 만들기 전에도 철회 목록을 먼저 적용한다.
/// 사용처: 공개 파일과 이후 검색.
pub fn public_visible(published: bool, denied: bool) -> bool {
    published && !denied
}

/// 개념: 철회 작업의 화면 상태.
/// 목적: API 차단과 외부 캐시 회수를 한 성공으로 합치지 않는다.
/// 사용처: 철회 목록.
pub fn withdrawal_memo(state: &str) -> &'static str {
    if state == "purge_failed" {
        "회수 표시를 쓰지 못했다. 공개 API 차단은 유지된다."
    } else {
        "공개 API에서는 막혔다. 외부 캐시 삭제는 보장하지 않는다."
    }
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/admin/entries/{id}/unpublish", post(unpublish_entry))
        .route("/api/admin/withdrawals", get(list_withdrawals))
        .route("/api/admin/withdrawals/{id}/retry", post(retry_withdrawal))
        .route("/api/admin/site-releases/{id}/restore", post(restore_release))
}

#[derive(Deserialize)]
struct UnpublishBody {
    #[serde(rename = "schemaVersion")]
    schema_version: i32,
    #[serde(rename = "expectedVersion")]
    expected_version: i32,
}

/// 개념: 글을 철회 목록에 넣고 공개 포인터를 비운다.
/// 목적: 202는 접수와 API 차단이다. 외부 캐시가 지워졌다는 뜻이 아니다.
/// 사용처: 편집 화면의 철회.
async fn unpublish_entry(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(entry_id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<UnpublishBody>(&body) else {
        return invalid();
    };
    if input.schema_version != 1 {
        return invalid_field("schemaVersion");
    }
    if input.expected_version < 1 {
        return invalid_field("expectedVersion");
    }
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let entry = sqlx::query_as::<_, (i32, Option<Uuid>)>("SELECT version, published_revision_id FROM entry WHERE id = $1 FOR UPDATE")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await;
    let (version, published_revision) = match entry {
        Ok(Some(row)) => row,
        Ok(None) => {
            let _ = tx.rollback().await;
            return not_found();
        }
        Err(_) => {
            let _ = tx.rollback().await;
            return unavailable();
        }
    };
    if version != input.expected_version {
        let _ = tx.rollback().await;
        return conflict();
    }
    let existing: Option<Uuid> = match sqlx::query_scalar("SELECT id FROM withdrawal WHERE entry_id = $1")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    if sqlx::query("INSERT INTO entry_denylist (entry_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(entry_id)
        .execute(tx.as_mut())
        .await
        .is_err()
    {
        let _ = tx.rollback().await;
        return unavailable();
    }
    if sqlx::query("UPDATE entry SET published_revision_id = NULL, updated_at = now() WHERE id = $1")
        .bind(entry_id)
        .execute(tx.as_mut())
        .await
        .is_err()
    {
        let _ = tx.rollback().await;
        return unavailable();
    }
    let withdrawal_id = if let Some(id) = existing {
        id
    } else {
        let id = Uuid::new_v4();
        if sqlx::query("INSERT INTO withdrawal (id, entry_id, revision_id, state, cache_state) VALUES ($1, $2, $3, 'blocked', 'local_only')")
            .bind(id)
            .bind(entry_id)
            .bind(published_revision)
            .execute(tx.as_mut())
            .await
            .is_err()
        {
            let _ = tx.rollback().await;
            return unavailable();
        }
        id
    };
    if tx.commit().await.is_err() {
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "entry_withdrawn").await;
    // 목록이 커밋된 뒤에 로컬 표시를 쓴다. 쓰기에 실패해도 철회 목록은 남긴다.
    if mark_local_withdrawal(entry_id).is_err() {
        let _ = sqlx::query("UPDATE withdrawal SET state = 'purge_failed', cache_state = 'purge_failed', updated_at = now() WHERE id = $1")
            .bind(withdrawal_id)
            .execute(&pool)
            .await;
    }
    accepted(withdrawal_id)
}

/// 개념: 철회 작업 목록.
/// 목적: API 차단과 회수 실패를 나눠 보여 준다.
/// 사용처: `/admin/publications`.
async fn list_withdrawals(State(pool): State<PgPool>, headers: HeaderMap) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String)>("SELECT id, entry_id, state, cache_state FROM withdrawal ORDER BY created_at DESC LIMIT 8")
        .fetch_all(&pool)
        .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let withdrawals = rows
        .into_iter()
        .map(|(id, entry_id, state, cache_state)| {
            json!({
                "id": id.to_string(),
                "entryId": entry_id.to_string(),
                "state": state,
                "cacheState": cache_state,
                "retryable": state == "purge_failed",
                "memo": withdrawal_memo(&state),
            })
        })
        .collect::<Vec<_>>();
    json_response(StatusCode::OK, json!({"schemaVersion": 1, "withdrawals": withdrawals}).to_string(), None)
}

/// 개념: 로컬 회수 표시만 다시 쓴다.
/// 목적: 실패한 회수를 성공한 철회로 바꾸지 않고, 철회 목록은 유지한다.
/// 사용처: 철회 목록의 재시도.
async fn retry_withdrawal(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (Uuid, String)>("SELECT entry_id, state FROM withdrawal WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await;
    let Ok(Some((entry_id, state))) = row else {
        return not_found();
    };
    if state != "purge_failed" {
        return conflict();
    }
    if mark_local_withdrawal(entry_id).is_err() {
        return json_response(StatusCode::OK, json!({"schemaVersion": 1, "state": "purge_failed", "succeeded": false}).to_string(), None);
    }
    let updated = sqlx::query("UPDATE withdrawal SET state = 'blocked', cache_state = 'local_only', updated_at = now() WHERE id = $1 AND state = 'purge_failed'")
        .bind(id)
        .execute(&pool)
        .await;
    if updated.is_err() {
        return unavailable();
    }
    json_response(StatusCode::OK, json!({"schemaVersion": 1, "state": "blocked", "succeeded": false}).to_string(), None)
}

/// 개념: 과거 릴리스를 현재 포인터로 되돌린다.
/// 목적: 그 글이 철회 목록에 있으면 복구하지 않는다.
/// 사용처: `POST /api/admin/site-releases/{id}/restore`.
async fn restore_release(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(release_id) = parse_id(&id) else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (Uuid, Uuid, Option<i64>)>(
        "SELECT entry_id, revision_id, release_sequence FROM site_release WHERE id = $1",
    )
    .bind(release_id)
    .fetch_optional(&pool)
    .await;
    let Ok(Some((entry_id, revision_id, _sequence))) = row else {
        return not_found();
    };
    let denied = match entry_is_denied(&pool, entry_id).await {
        Ok(denied) => denied,
        Err(()) => return unavailable(),
    };
    if rollback_decision(denied) == "refuse" {
        return conflict();
    }
    let root = release_root();
    let marker = fs::read_to_string(root.join(release_id.to_string()).join("release.txt")).ok();
    if marker.as_deref().map(str::trim) != Some(release_id.to_string().as_str()) {
        return conflict();
    }
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let sequence: Result<i64, _> = sqlx::query_scalar("UPDATE site_generation SET sequence = sequence + 1 WHERE id = 1 RETURNING sequence")
        .fetch_one(tx.as_mut())
        .await;
    let Ok(sequence) = sequence else {
        let _ = tx.rollback().await;
        return unavailable();
    };
    if sqlx::query("UPDATE site_release SET phase = 'superseded' WHERE phase = 'active' AND id <> $1")
        .bind(release_id)
        .execute(tx.as_mut())
        .await
        .is_err()
    {
        let _ = tx.rollback().await;
        return unavailable();
    }
    if sqlx::query("UPDATE site_release SET phase = 'active', release_sequence = $2, confirmed_at = now() WHERE id = $1")
        .bind(release_id)
        .bind(sequence)
        .execute(tx.as_mut())
        .await
        .is_err()
    {
        let _ = tx.rollback().await;
        return unavailable();
    }
    let published = sqlx::query("UPDATE entry SET published_revision_id = $2, updated_at = now() WHERE id = $1 AND NOT EXISTS (SELECT 1 FROM entry_denylist WHERE entry_denylist.entry_id = entry.id)")
        .bind(entry_id)
        .bind(revision_id)
        .execute(tx.as_mut())
        .await;
    let Ok(published) = published else {
        let _ = tx.rollback().await;
        return unavailable();
    };
    if published.rows_affected() != 1 {
        let _ = tx.rollback().await;
        return conflict();
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    let previous = read_pointer_id(&root);
    if write_pointer(&root, release_id, previous).is_err() {
        return json_response(StatusCode::OK, json!({"schemaVersion": 1, "succeeded": false, "failureCode": "pointer_failed"}).to_string(), None);
    }
    json_response(StatusCode::OK, json!({"schemaVersion": 1, "succeeded": true, "releaseId": release_id.to_string()}).to_string(), None)
}

/// 개념: 글이 철회 목록에 있는지.
/// 목적: 발행 성공과 과거 세대 복구가 같은 목록을 보게 한다.
/// 사용처: `commit_success`, `restore_release`.
pub async fn entry_is_denied(pool: &PgPool, entry_id: Uuid) -> Result<bool, ()> {
    let row: Option<i32> = sqlx::query_scalar("SELECT 1 FROM entry_denylist WHERE entry_id = $1")
        .bind(entry_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ())?;
    Ok(row.is_some())
}

/// 개념: 공개 파일이 철회된 글에 연결되어 있는지.
/// 목적: 첨부 직접 주소로 철회한 글을 다시 열지 못하게 한다.
/// 사용처: `GET /api/media/{id}`.
pub async fn media_is_withdrawn(pool: &PgPool, media_id: Uuid) -> Result<bool, ()> {
    let row: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM revision_media
         JOIN revision ON revision.id = revision_media.revision_id
         JOIN entry_denylist ON entry_denylist.entry_id = revision.entry_id
         WHERE revision_media.media_id = $1",
    )
    .bind(media_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ())?;
    Ok(row.is_some())
}

/// 개념: 현재와 이전 릴리스 폴더에 철회 표시를 남긴다.
/// 목적: 디스크에 남은 HTML을 공개 성공으로 보지 않게 한다. 외부 캐시를 지웠다는 뜻은 아니다.
/// 사용처: 철회와 회수 재시도.
fn mark_local_withdrawal(entry_id: Uuid) -> Result<(), ()> {
    let root = release_root();
    let text = match fs::read_to_string(root.join("current")) {
        Ok(text) => text,
        Err(_) => return Ok(()),
    };
    let value: Value = serde_json::from_str(&text).map_err(|_| ())?;
    let mut ids = Vec::new();
    if let Some(id) = value.get("releaseId").and_then(Value::as_str) {
        ids.push(id.to_string());
    }
    if let Some(id) = value.get("previousReleaseId").and_then(Value::as_str) {
        ids.push(id.to_string());
    }
    for id in ids {
        let dir = root.join(id).join("withdrawn");
        fs::create_dir_all(&dir).map_err(|_| ())?;
        fs::write(dir.join(entry_id.to_string()), "withdrawn\n").map_err(|_| ())?;
    }
    Ok(())
}

fn write_pointer(root: &std::path::Path, release_id: Uuid, previous: Option<Uuid>) -> Result<(), ()> {
    fs::create_dir_all(root).map_err(|_| ())?;
    let body = json!({
        "releaseId": release_id.to_string(),
        "previousReleaseId": previous.map(|value| value.to_string()),
    });
    let tmp = root.join("current.next");
    fs::write(&tmp, body.to_string()).map_err(|_| ())?;
    let current = root.join("current");
    if current.exists() {
        let backup = root.join("current.bak");
        let _ = fs::remove_file(&backup);
        fs::rename(&current, &backup).map_err(|_| ())?;
        if fs::rename(&tmp, &current).is_err() {
            let _ = fs::rename(&backup, &current);
            return Err(());
        }
        let _ = fs::remove_file(&backup);
        return Ok(());
    }
    fs::rename(&tmp, &current).map_err(|_| ())
}

fn read_pointer_id(root: &std::path::Path) -> Option<Uuid> {
    let text = fs::read_to_string(root.join("current")).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value.get("releaseId").and_then(Value::as_str).and_then(parse_id)
}

fn release_root() -> PathBuf {
    PathBuf::from(std::env::var("RELEASE_ROOT").unwrap_or_else(|_| "var/releases".to_string()))
}

fn accepted(id: Uuid) -> Response {
    json_response(StatusCode::ACCEPTED, json!({"schemaVersion": 1, "jobId": id.to_string()}).to_string(), None)
}

fn invalid_field(path: &str) -> Response {
    json_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({"error": "invalid", "fieldErrors": [{"path": path, "code": "invalid"}]}).to_string(),
        None,
    )
}

fn parse_id(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

#[cfg(test)]
mod tests {
    use super::{activation_decision, public_visible, rollback_decision, withdrawal_memo};

    /// 개념: 세대와 철회가 공개를 막는지.
    /// 목적: 오래된 성공, 철회된 글, 과거 세대 복구가 공개로 이어지지 않는지 확인한다.
    /// 사용처: `cargo test`.
    #[test]
    fn old_success_and_withdrawn_entries_stay_hidden() {
        assert_eq!(activation_decision(true, false), "stale");
        assert_eq!(activation_decision(false, true), "withheld");
        assert_eq!(activation_decision(false, false), "activate");
        assert!(!public_visible(true, true));
        assert!(public_visible(true, false));
        assert!(!public_visible(false, false));
        assert_eq!(rollback_decision(true), "refuse");
        assert_eq!(rollback_decision(false), "restore");
        assert!(withdrawal_memo("blocked").contains("외부 캐시"));
        assert!(withdrawal_memo("purge_failed").contains("공개 API 차단"));
    }
}
