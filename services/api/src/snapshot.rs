//! 개념: 발행 스냅샷, 후보 폴더, 포인터 확인.
//! 목적: 빌드 입력을 고정하고, 릴리스 표시가 맞기 전에는 공개 성공으로 기록하지 않는다.
//! 사용처: `POST /api/admin/publication-jobs/{id}/release`, 빌드 토큰의 스냅샷 조회.

use crate::admin::{authorize, conflict, json_response, not_found, unavailable};
use crate::withdrawal::{activation_decision, entry_is_denied, rollback_decision};
use crate::auth::{random_token, sha256_hex};
use crate::publication::JobState;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::fs;
use std::path::{Path as FsPath, PathBuf};
use uuid::Uuid;

/// 개념: 확인 뒤의 다음 상태.
/// 목적: 검증 실패나 표시 불일치는 성공으로 닫지 않는다.
/// 사용처: 릴리스 확인과 단위 시험.
pub fn next_after_check(validation_ok: bool, marker_ok: bool) -> &'static str {
    if !validation_ok {
        "failed"
    } else if !marker_ok {
        "deploying"
    } else {
        "succeeded"
    }
}

/// 개념: 포인터와 DB가 어긋났을 때의 다음 동작.
/// 목적: 파일 포인터와 DB 성공을 한 트랜잭션으로 묶지 않고, 다시 읽어 맞춘다.
/// 사용처: `reconcile_releases`.
#[derive(Debug, PartialEq, Eq)]
pub enum Reconcile {
    InSync,
    RecordSuccess,
    RestorePrevious,
}

pub fn plan_reconcile(pointer: Option<Uuid>, marker: Option<Uuid>, db_active: Option<Uuid>) -> Reconcile {
    match (pointer, marker, db_active) {
        (Some(pointer_id), Some(marker_id), db) if pointer_id == marker_id && db != Some(pointer_id) => Reconcile::RecordSuccess,
        (Some(pointer_id), marker, _) if marker != Some(pointer_id) => Reconcile::RestorePrevious,
        _ => Reconcile::InSync,
    }
}

/// 개념: 스냅샷에 넣을 본문.
/// 목적: 제목, 수정본, 미디어 해시, 릴리스 ID를 한 객체로 고정한다.
/// 사용처: 릴리스 행과 후보 폴더.
pub fn snapshot_document(release_id: Uuid, entry_id: Uuid, revision_id: Uuid, slug: &str, kind: &str, title: &str, blocks: &Value, media: &[Value]) -> Value {
    json!({
        "schemaVersion": 1,
        "releaseId": release_id.to_string(),
        "entryId": entry_id.to_string(),
        "revisionId": revision_id.to_string(),
        "slug": slug,
        "kind": kind,
        "title": title,
        "blocks": blocks,
        "media": media,
    })
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/admin/publication-jobs/{id}/release", post(release_job))
        .route("/api/admin/publication-jobs/reconcile", post(reconcile_releases))
        .route("/api/publish/snapshots/{id}", get(read_snapshot))
}

/// 개념: 후보를 만들고 표시를 확인한 뒤에만 성공으로 기록한다.
/// 목적: 배포 대기 작업을 공개 완료로 바로 바꾸지 않는다.
/// 사용처: 발행 작업 화면의 확인 버튼.
async fn release_job(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(job_id) = parse_id(&id) else {
        return not_found();
    };
    let root = release_root();
    let prepared = match prepare_snapshot(&pool, job_id, &root).await {
        Ok(prepared) => prepared,
        Err(ReleaseError::Conflict) => return conflict(),
        Err(ReleaseError::Missing) => return not_found(),
        Err(ReleaseError::Unavailable) => return unavailable(),
    };
    // 스냅샷 행이 커밋된 뒤에 파일을 쓴다. 이후 초안 수정은 이 폴더를 바꾸지 않는다.
    if let Some(token) = &prepared.token {
        let _ = fs::create_dir_all(&prepared.dir);
        let _ = fs::write(prepared.dir.join("build-token.txt"), token);
    }
    let manifest_hash = match materialize(&prepared.dir, prepared.release_id, &prepared.snapshot) {
        Ok(hash) => hash,
        Err(code) => {
            let _ = mark_failed(&pool, job_id, prepared.release_id, code).await;
            return release_body("failed", false, Some(code));
        }
    };
    let _ = sqlx::query("UPDATE site_release SET manifest_hash = $2, phase = 'candidate' WHERE id = $1 AND phase = 'snapshot'")
        .bind(prepared.release_id)
        .bind(&manifest_hash)
        .execute(&pool)
        .await;
    if let Err(code) = validate_candidate(&prepared.dir, prepared.release_id) {
        let _ = mark_failed(&pool, job_id, prepared.release_id, code).await;
        return release_body("failed", false, Some(code));
    }
    let previous = read_pointer(&root).ok().flatten();
    if swap_pointer(&root, prepared.release_id, previous).is_err() {
        let _ = mark_failed(&pool, job_id, prepared.release_id, "pointer_failed").await;
        return release_body("failed", false, Some("pointer_failed"));
    }
    let marker_ok = read_marker(&root).ok().flatten() == Some(prepared.release_id);
    // 표시가 다르면 실패로 닫지 않는다. 이전 포인터만 되돌리고 작업은 배포 대기로 남긴다.
    if next_after_check(true, marker_ok) != "succeeded" {
        restore_pointer(&root, previous);
        return release_body("deploying", false, Some("marker_mismatch"));
    }
    match commit_success(&pool, job_id, prepared.release_id, prepared.entry_id, prepared.revision_id).await {
        Ok(()) => release_body("succeeded", true, None),
        Err(ConfirmError::Stale) => {
            restore_pointer(&root, previous);
            release_body("failed", false, Some("stale_generation"))
        }
        Err(ConfirmError::Withdrawn) => {
            restore_pointer(&root, previous);
            release_body("failed", false, Some("withdrawn_entry"))
        }
        Err(ConfirmError::Database) => release_body("deploying", false, Some("db_confirm_failed")),
    }
}

/// 개념: 포인터와 DB 성공 기록을 다시 맞춘다.
/// 목적: 포인터는 바뀌었는데 DB 갱신이 실패한 경우를 파일과 DB를 따로 읽어 정리한다.
/// 사용처: `POST /api/admin/publication-jobs/reconcile`.
async fn reconcile_releases(State(pool): State<PgPool>, headers: HeaderMap) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let root = release_root();
    let pointer = read_pointer(&root).ok().flatten();
    let marker = read_marker(&root).ok().flatten();
    let db_active: Option<Uuid> = match sqlx::query_scalar("SELECT id FROM site_release WHERE phase = 'active' ORDER BY confirmed_at DESC NULLS LAST LIMIT 1")
        .fetch_optional(&pool)
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    match plan_reconcile(pointer, marker, db_active) {
        Reconcile::InSync => json_response(StatusCode::OK, json!({"schemaVersion": 1, "action": "in_sync", "succeeded": false}).to_string(), None),
        Reconcile::RecordSuccess => {
            let Some(release_id) = pointer else {
                return unavailable();
            };
            let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(
                "SELECT job_id, entry_id, revision_id FROM site_release WHERE id = $1",
            )
            .bind(release_id)
            .fetch_optional(&pool)
            .await;
            let Ok(Some((job_id, entry_id, revision_id))) = row else {
                return unavailable();
            };
            match commit_success(&pool, job_id, release_id, entry_id, revision_id).await {
                Ok(()) => json_response(StatusCode::OK, json!({"schemaVersion": 1, "action": "record_success", "succeeded": true}).to_string(), None),
                Err(ConfirmError::Stale) | Err(ConfirmError::Withdrawn) => {
                    json_response(StatusCode::OK, json!({"schemaVersion": 1, "action": "withheld", "succeeded": false}).to_string(), None)
                }
                Err(ConfirmError::Database) => unavailable(),
            }
        }
        Reconcile::RestorePrevious => {
            let previous = pointer_previous(&root);
            if let Some(previous_id) = previous {
                let entry_id: Result<Option<Uuid>, _> = sqlx::query_scalar("SELECT entry_id FROM site_release WHERE id = $1")
                    .bind(previous_id)
                    .fetch_optional(&pool)
                    .await;
                let Ok(Some(entry_id)) = entry_id else {
                    return json_response(StatusCode::OK, json!({"schemaVersion": 1, "action": "held", "succeeded": false}).to_string(), None);
                };
                let denied = match entry_is_denied(&pool, entry_id).await {
                    Ok(denied) => denied,
                    Err(()) => return unavailable(),
                };
                if rollback_decision(denied) == "refuse" {
                    return json_response(StatusCode::OK, json!({"schemaVersion": 1, "action": "held", "succeeded": false}).to_string(), None);
                }
                let _ = swap_pointer(&root, previous_id, pointer);
            } else {
                let _ = fs::remove_file(root.join("current"));
            }
            json_response(StatusCode::OK, json!({"schemaVersion": 1, "action": "restore_previous", "succeeded": false}).to_string(), None)
        }
    }
}

/// 개념: 빌드 토큰으로 고정 스냅샷만 읽는다.
/// 목적: 토큰이 다른 릴리스나 현재 초안을 보지 못하게 한다.
/// 사용처: 후보 빌드.
async fn read_snapshot(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    let Some(release_id) = parse_id(&id) else {
        return not_found();
    };
    let Some(token) = headers.get("x-build-token").and_then(|value| value.to_str().ok()) else {
        return crate::admin::unauthorized();
    };
    let row = sqlx::query_as::<_, (String, String)>("SELECT snapshot::text, build_token_hash FROM site_release WHERE id = $1")
        .bind(release_id)
        .fetch_optional(&pool)
        .await;
    let Ok(Some((snapshot, hash))) = row else {
        return not_found();
    };
    if !crate::auth::tokens_match(&sha256_hex(token), &hash) {
        return crate::admin::forbidden();
    }
    json_response(StatusCode::OK, snapshot, None)
}

struct Prepared {
    release_id: Uuid,
    entry_id: Uuid,
    revision_id: Uuid,
    dir: PathBuf,
    snapshot: Value,
    token: Option<String>,
}

enum ReleaseError {
    Conflict,
    Missing,
    Unavailable,
}

async fn prepare_snapshot(pool: &PgPool, job_id: Uuid, root: &FsPath) -> Result<Prepared, ReleaseError> {
    let mut tx = pool.begin().await.map_err(|_| ReleaseError::Unavailable)?;
    let job = sqlx::query_as::<_, (String, Uuid, Uuid)>("SELECT state, entry_id, revision_id FROM publication_job WHERE id = $1 FOR UPDATE")
        .bind(job_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|_| ReleaseError::Unavailable)?;
    let Some((state, entry_id, revision_id)) = job else {
        return Err(ReleaseError::Missing);
    };
    if state != JobState::Deploying.as_str() {
        return Err(ReleaseError::Conflict);
    }
    let existing = sqlx::query_as::<_, (Uuid, String)>("SELECT id, snapshot::text FROM site_release WHERE job_id = $1")
        .bind(job_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|_| ReleaseError::Unavailable)?;
    if let Some((release_id, snapshot)) = existing {
        let snapshot: Value = serde_json::from_str(&snapshot).map_err(|_| ReleaseError::Unavailable)?;
        tx.commit().await.map_err(|_| ReleaseError::Unavailable)?;
        return Ok(Prepared { release_id, entry_id, revision_id, dir: root.join(release_id.to_string()), snapshot, token: None });
    }
    let entry = sqlx::query_as::<_, (String, String)>("SELECT slug, kind FROM entry WHERE id = $1")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|_| ReleaseError::Unavailable)?;
    let Some((slug, kind)) = entry else {
        return Err(ReleaseError::Missing);
    };
    let body: Option<String> = sqlx::query_scalar("SELECT body::text FROM revision WHERE id = $1 AND entry_id = $2")
        .bind(revision_id)
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|_| ReleaseError::Unavailable)?;
    let Some(body) = body else {
        return Err(ReleaseError::Missing);
    };
    let document: Value = serde_json::from_str(&body).map_err(|_| ReleaseError::Unavailable)?;
    let title = document.get("title").and_then(Value::as_str).unwrap_or("").to_string();
    let blocks = document.get("blocks").cloned().unwrap_or_else(|| json!([]));
    let media_rows = sqlx::query_as::<_, (Uuid, String)>("SELECT media.id, media.checksum FROM media JOIN revision_media ON revision_media.media_id = media.id WHERE revision_media.revision_id = $1")
        .bind(revision_id)
        .fetch_all(tx.as_mut())
        .await
        .map_err(|_| ReleaseError::Unavailable)?;
    let media = media_rows
        .into_iter()
        .map(|(id, checksum)| json!({"id": id.to_string(), "checksum": checksum}))
        .collect::<Vec<_>>();
    let release_id = Uuid::new_v4();
    let snapshot = snapshot_document(release_id, entry_id, revision_id, &slug, &kind, &title, &blocks, &media);
    let token = random_token();
    let prefix = release_id.to_string();
    sqlx::query(
        "INSERT INTO site_release (id, job_id, entry_id, revision_id, schema_version, snapshot, snapshot_hash, build_token_hash, candidate_prefix, phase)
         VALUES ($1, $2, $3, $4, 1, $5::jsonb, $6, $7, $8, 'snapshot')",
    )
    .bind(release_id)
    .bind(job_id)
    .bind(entry_id)
    .bind(revision_id)
    .bind(snapshot.to_string())
    .bind(sha256_hex(&snapshot.to_string()))
    .bind(sha256_hex(&token))
    .bind(&prefix)
    .execute(tx.as_mut())
    .await
    .map_err(|_| ReleaseError::Unavailable)?;
    tx.commit().await.map_err(|_| ReleaseError::Unavailable)?;
    // 토큰 원문은 응답 JSON에 넣지 않고, 릴리스 폴더의 비공개 파일로만 남긴다.
    Ok(Prepared { release_id, entry_id, revision_id, dir: root.join(prefix), snapshot, token: Some(token) })
}

enum ConfirmError {
    Database,
    Stale,
    Withdrawn,
}

async fn commit_success(pool: &PgPool, job_id: Uuid, release_id: Uuid, entry_id: Uuid, revision_id: Uuid) -> Result<(), ConfirmError> {
    let mut tx = pool.begin().await.map_err(|_| ConfirmError::Database)?;
    let _locked: i64 = sqlx::query_scalar("SELECT sequence FROM site_generation WHERE id = 1 FOR UPDATE")
        .fetch_one(tx.as_mut())
        .await
        .map_err(|_| ConfirmError::Database)?;
    let denied: Option<i32> = sqlx::query_scalar("SELECT 1 FROM entry_denylist WHERE entry_id = $1")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|_| ConfirmError::Database)?;
    let newer: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM site_release
            WHERE phase = 'active'
              AND confirmed_at > (SELECT created_at FROM publication_job WHERE id = $1)
        )",
    )
    .bind(job_id)
    .fetch_one(tx.as_mut())
    .await
    .map_err(|_| ConfirmError::Database)?;
    match activation_decision(newer, denied.is_some()) {
        "withheld" => {
            let _ = tx.rollback().await;
            let _ = mark_failed(pool, job_id, release_id, "withdrawn_entry").await;
            return Err(ConfirmError::Withdrawn);
        }
        "stale" => {
            let _ = tx.rollback().await;
            let _ = mark_failed(pool, job_id, release_id, "stale_generation").await;
            return Err(ConfirmError::Stale);
        }
        _ => {}
    }
    let sequence: i64 = sqlx::query_scalar("UPDATE site_generation SET sequence = sequence + 1 WHERE id = 1 RETURNING sequence")
        .fetch_one(tx.as_mut())
        .await
        .map_err(|_| ConfirmError::Database)?;
    let updated = sqlx::query("UPDATE publication_job SET state = 'succeeded', release_id = $2, failure_code = NULL, updated_at = now() WHERE id = $1 AND state = 'deploying'")
        .bind(job_id)
        .bind(release_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ConfirmError::Database)?;
    if updated.rows_affected() != 1 {
        let _ = tx.rollback().await;
        return Err(ConfirmError::Database);
    }
    sqlx::query("UPDATE site_release SET phase = 'superseded' WHERE phase = 'active' AND id <> $1")
        .bind(release_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ConfirmError::Database)?;
    sqlx::query("UPDATE entry SET published_revision_id = $2, updated_at = now() WHERE id = $1 AND NOT EXISTS (SELECT 1 FROM entry_denylist WHERE entry_denylist.entry_id = entry.id)")
        .bind(entry_id)
        .bind(revision_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ConfirmError::Database)?;
    sqlx::query("UPDATE site_release SET phase = 'active', release_sequence = $2, confirmed_at = now(), failure_code = NULL WHERE id = $1")
        .bind(release_id)
        .bind(sequence)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ConfirmError::Database)?;
    tx.commit().await.map_err(|_| ConfirmError::Database)
}

async fn mark_failed(pool: &PgPool, job_id: Uuid, release_id: Uuid, code: &str) -> Result<(), ()> {
    let mut tx = pool.begin().await.map_err(|_| ())?;
    sqlx::query("UPDATE publication_job SET state = 'failed', failure_code = $2, updated_at = now() WHERE id = $1 AND state = 'deploying'")
        .bind(job_id)
        .bind(code)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ())?;
    sqlx::query("UPDATE site_release SET phase = 'failed', failure_code = $2 WHERE id = $1")
        .bind(release_id)
        .bind(code)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ())?;
    tx.commit().await.map_err(|_| ())
}

/// 개념: 후보 폴더를 채운다.
/// 목적: 기본은 고정 스냅샷 HTML이다. `DG_PUBLISH_REACT=1`일 때만 React 빌드 스크립트를 부른다.
/// 사용처: `release_job`.
fn materialize(dir: &FsPath, release_id: Uuid, snapshot: &Value) -> Result<String, &'static str> {
    if std::env::var("DG_PUBLISH_REACT").ok().as_deref() == Some("1") {
        return react_candidate(dir, release_id, snapshot);
    }
    write_candidate(dir, release_id, snapshot)
}

/// 개념: React 정적 빌드를 후보 폴더에만 실행한다.
/// 목적: 활성 `apps/web/dist`를 후보 출력으로 쓰지 않는다. 스크립트가 그 경로를 거부한다.
/// 사용처: `DG_PUBLISH_REACT=1`.
fn react_candidate(dir: &FsPath, release_id: Uuid, snapshot: &Value) -> Result<String, &'static str> {
    fs::create_dir_all(dir).map_err(|_| "build_failed")?;
    let snapshot_path = dir.join("snapshot.json");
    fs::write(&snapshot_path, snapshot.to_string()).map_err(|_| "build_failed")?;
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../infra/publish/build-candidate.mjs");
    let status = std::process::Command::new("node")
        .arg(&script)
        .arg(&snapshot_path)
        .arg(dir)
        .arg(release_id.to_string())
        .status()
        .map_err(|_| "build_failed")?;
    if !status.success() {
        return Err("build_failed");
    }
    let manifest = fs::read_to_string(dir.join("manifest.json")).map_err(|_| "build_failed")?;
    Ok(sha256_hex(&manifest))
}

/// 개념: 고정 스냅샷으로 후보 폴더를 만든다.
/// 목적: 활성 dist가 아닌 릴리스 ID 폴더에만 HTML, 자산, 사이트맵, 표시를 쓴다.
/// 사용처: 릴리스 확인. React 전체 빌드는 `infra/publish/build-candidate.mjs`가 같은 표시 계약을 쓴다.
pub fn write_candidate(dir: &FsPath, release_id: Uuid, snapshot: &Value) -> Result<String, &'static str> {
    fs::create_dir_all(dir.join("assets")).map_err(|_| "candidate_write_failed")?;
    let title = snapshot.get("title").and_then(Value::as_str).unwrap_or("");
    let paragraphs = snapshot
        .get("blocks")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter(|block| block.get("type").and_then(Value::as_str) == Some("paragraph"))
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .map(escape_html)
                .collect::<Vec<_>>()
                .join("</p><p>")
        })
        .unwrap_or_default();
    let marker = format!("dg-release:{release_id}");
    let html = format!(
        "<!doctype html><html><head><meta name=\"dg-release\" content=\"{release_id}\"><link rel=\"stylesheet\" href=\"/assets/site.css\"></head><body><p>{marker}</p><h1>{}</h1><p>{paragraphs}</p><a href=\"/sitemap.xml\">sitemap</a></body></html>",
        escape_html(title)
    );
    fs::write(dir.join("index.html"), html).map_err(|_| "candidate_write_failed")?;
    fs::write(dir.join("assets").join("site.css"), "/* release */\n").map_err(|_| "candidate_write_failed")?;
    let loc = public_path(snapshot.get("kind").and_then(Value::as_str).unwrap_or("note"), snapshot.get("slug").and_then(Value::as_str).unwrap_or(""));
    let sitemap = format!("<?xml version=\"1.0\"?><urlset><url><loc>{loc}</loc></url></urlset>");
    fs::write(dir.join("sitemap.xml"), sitemap).map_err(|_| "candidate_write_failed")?;
    fs::write(dir.join("release.txt"), format!("{release_id}\n")).map_err(|_| "candidate_write_failed")?;
    fs::write(dir.join("snapshot.json"), snapshot.to_string()).map_err(|_| "candidate_write_failed")?;
    let manifest = json!({
        "schemaVersion": 1,
        "releaseId": release_id.to_string(),
        "snapshotHash": sha256_hex(&snapshot.to_string()),
        "builder": "snapshot-html",
    })
    .to_string();
    let manifest_hash = sha256_hex(&manifest);
    fs::write(dir.join("manifest.json"), &manifest).map_err(|_| "candidate_write_failed")?;
    fs::write(dir.join("build.log"), format!("builder=snapshot-html\nmanifestHash={manifest_hash}\n")).map_err(|_| "candidate_write_failed")?;
    Ok(manifest_hash)
}

/// 개념: 후보 폴더 검사.
/// 목적: 자산이 없거나 초안 표시가 있으면 포인터를 옮기지 않는다.
/// 사용처: `release_job`.
pub fn validate_candidate(dir: &FsPath, release_id: Uuid) -> Result<(), &'static str> {
    let index = fs::read_to_string(dir.join("index.html")).map_err(|_| "missing_asset")?;
    if index.contains("data-draft") {
        return Err("draft_included");
    }
    if !index.contains(&format!("dg-release:{release_id}")) {
        return Err("marker_mismatch");
    }
    if !dir.join("assets").join("site.css").is_file() {
        return Err("missing_asset");
    }
    let sitemap = fs::read_to_string(dir.join("sitemap.xml")).map_err(|_| "missing_sitemap")?;
    if sitemap.contains("/admin") || sitemap.contains("data-draft") {
        return Err("draft_included");
    }
    let marker = fs::read_to_string(dir.join("release.txt")).map_err(|_| "marker_mismatch")?;
    if marker.trim() != release_id.to_string() {
        return Err("marker_mismatch");
    }
    Ok(())
}

fn swap_pointer(root: &FsPath, release_id: Uuid, previous: Option<Uuid>) -> Result<(), ()> {
    fs::create_dir_all(root).map_err(|_| ())?;
    let body = json!({
        "releaseId": release_id.to_string(),
        "previousReleaseId": previous.map(|value| value.to_string()),
    });
    let tmp = root.join("current.next");
    fs::write(&tmp, body.to_string()).map_err(|_| ())?;
    let current = root.join("current");
    // Windows에서는 있는 파일 위로 rename이 되지 않는다. 바꾸기 전에 치우고, 실패하면 되돌린다.
    if current.exists() {
        let backup = root.join("current.bak");
        let _ = fs::remove_file(&backup);
        fs::rename(&current, &backup).map_err(|_| ())?;
        if let Err(error) = fs::rename(&tmp, &current) {
            let _ = fs::rename(&backup, &current);
            let _ = error;
            return Err(());
        }
        let _ = fs::remove_file(&backup);
        return Ok(());
    }
    fs::rename(&tmp, &current).map_err(|_| ())
}

fn read_pointer(root: &FsPath) -> Result<Option<Uuid>, ()> {
    let text = match fs::read_to_string(root.join("current")) {
        Ok(text) => text,
        Err(_) => return Ok(None),
    };
    let value: Value = serde_json::from_str(&text).map_err(|_| ())?;
    Ok(value.get("releaseId").and_then(Value::as_str).and_then(parse_id))
}

fn pointer_previous(root: &FsPath) -> Option<Uuid> {
    let text = fs::read_to_string(root.join("current")).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value.get("previousReleaseId").and_then(Value::as_str).and_then(parse_id)
}

fn restore_pointer(root: &FsPath, previous: Option<Uuid>) {
    if let Some(previous_id) = previous {
        let _ = swap_pointer(root, previous_id, None);
    } else {
        let _ = fs::remove_file(root.join("current"));
    }
}

fn read_marker(root: &FsPath) -> Result<Option<Uuid>, ()> {
    let Some(release_id) = read_pointer(root)? else {
        return Ok(None);
    };
    let text = fs::read_to_string(root.join(release_id.to_string()).join("release.txt")).map_err(|_| ())?;
    Ok(parse_id(text.trim()))
}

fn public_path(kind: &str, slug: &str) -> String {
    let section = match kind {
        "log" => "logs",
        "paper" | "news" => "research",
        _ => "notes",
    };
    format!("/{section}/{slug}")
}

fn escape_html(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn release_root() -> PathBuf {
    PathBuf::from(std::env::var("RELEASE_ROOT").unwrap_or_else(|_| "var/releases".to_string()))
}

fn release_body(state: &str, succeeded: bool, failure: Option<&str>) -> Response {
    json_response(
        StatusCode::OK,
        json!({"schemaVersion": 1, "state": state, "succeeded": succeeded, "failureCode": failure}).to_string(),
        None,
    )
}

fn parse_id(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

#[cfg(test)]
mod tests {
    use super::{next_after_check, plan_reconcile, pointer_previous, read_marker, read_pointer, restore_pointer, snapshot_document, swap_pointer, validate_candidate, write_candidate, Reconcile};
    use serde_json::json;
    use uuid::Uuid;

    /// 개념: 성공 기록의 조건.
    /// 목적: 검증 실패와 표시 불일치에서는 succeeded가 되지 않는지 확인한다.
    /// 사용처: `cargo test`.
    #[test]
    fn success_requires_validation_and_marker() {
        assert_eq!(next_after_check(false, true), "failed");
        assert_eq!(next_after_check(true, false), "deploying");
        assert_eq!(next_after_check(true, true), "succeeded");
        let release_id = Uuid::from_u128(7);
        let pointer = Some(release_id);
        assert_eq!(plan_reconcile(pointer, pointer, None), Reconcile::RecordSuccess);
        assert_eq!(plan_reconcile(pointer, None, None), Reconcile::RestorePrevious);
        assert_eq!(plan_reconcile(pointer, pointer, pointer), Reconcile::InSync);
    }

    /// 개념: 후보 폴더와 초안 차단.
    /// 목적: 스냅샷 HTML은 자산을 갖추고, 초안 표시가 있으면 검증이 멈추는지 확인한다.
    /// 사용처: `cargo test`.
    #[test]
    fn candidate_rejects_a_draft_marker_and_keeps_the_snapshot_text() {
        let release_id = Uuid::new_v4();
        let entry_id = Uuid::new_v4();
        let revision_id = Uuid::new_v4();
        let snapshot = snapshot_document(
            release_id,
            entry_id,
            revision_id,
            "field-note",
            "note",
            "현장 <메모>",
            &json!([{"type": "paragraph", "text": "저장한 문장"}]),
            &[],
        );
        let dir = std::env::temp_dir().join(format!("dg-release-{release_id}"));
        let _ = std::fs::remove_dir_all(&dir);
        let frozen = snapshot.to_string();
        let changed = snapshot_document(release_id, entry_id, revision_id, "field-note", "note", "다른 제목", &json!([]), &[]);
        assert_ne!(crate::auth::sha256_hex(&frozen), crate::auth::sha256_hex(&changed.to_string()));
        assert!(frozen.contains("현장 <메모>"));
        let manifest_hash = write_candidate(&dir, release_id, &snapshot).expect("write");
        assert!(!manifest_hash.is_empty());
        validate_candidate(&dir, release_id).expect("valid");
        std::fs::remove_file(dir.join("sitemap.xml")).expect("sitemap");
        assert_eq!(validate_candidate(&dir, release_id), Err("missing_sitemap"));
        std::fs::write(dir.join("sitemap.xml"), "<?xml version=\"1.0\"?><urlset></urlset>").expect("sitemap");
        std::fs::remove_file(dir.join("assets").join("site.css")).expect("asset");
        assert_eq!(validate_candidate(&dir, release_id), Err("missing_asset"));
        std::fs::write(dir.join("assets").join("site.css"), "/* release */\n").expect("asset");
        let html = std::fs::read_to_string(dir.join("index.html")).expect("html");
        assert!(html.contains("현장 &lt;메모&gt;"));
        assert!(html.contains("저장한 문장"));
        assert!(!html.contains("apps/web/dist"));
        let mut drafted = html.clone();
        drafted.push_str("<p data-draft=\"true\"></p>");
        std::fs::write(dir.join("index.html"), drafted).expect("draft");
        assert_eq!(validate_candidate(&dir, release_id), Err("draft_included"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 개념: 로컬 포인터와 릴리스 표시.
    /// 목적: 표시가 포인터와 같을 때만 성공으로 보고, 표시가 없으면 이전 포인터를 되돌릴 계획인지 확인한다.
    /// 사용처: `cargo test`.
    #[test]
    fn pointer_swap_reads_the_marker_before_success() {
        let root = std::env::temp_dir().join(format!("dg-pointer-{}", Uuid::new_v4()));
        let previous = Uuid::new_v4();
        let release_id = Uuid::new_v4();
        let snapshot = snapshot_document(release_id, Uuid::new_v4(), Uuid::new_v4(), "note", "note", "제목", &json!([]), &[]);
        write_candidate(&root.join(previous.to_string()), previous, &snapshot).expect("previous");
        swap_pointer(&root, previous, None).expect("first pointer");
        write_candidate(&root.join(release_id.to_string()), release_id, &snapshot).expect("next");
        swap_pointer(&root, release_id, Some(previous)).expect("swap");
        assert_eq!(read_marker(&root).expect("marker"), Some(release_id));
        assert_eq!(next_after_check(true, true), "succeeded");
        std::fs::write(root.join(release_id.to_string()).join("release.txt"), "other\n").expect("bad marker");
        assert!(read_marker(&root).expect("read").is_none() || read_marker(&root).expect("read") != Some(release_id));
        assert_eq!(plan_reconcile(Some(release_id), None, None), Reconcile::RestorePrevious);
        assert_eq!(pointer_previous(&root), Some(previous));
        restore_pointer(&root, Some(previous));
        assert_eq!(read_pointer(&root).expect("restored"), Some(previous));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 개념: 격리 DB에서 표시 확인과 재조정.
    /// 목적: 초안을 나중에 고쳐도 스냅샷이 그대로이고, 표시가 맞을 때만 공개 포인터가 바뀌는지 확인한다.
    /// 사용처: Docker의 `datagrizzly_m2`. 개발 DB 행은 지우지 않는다.
    #[tokio::test]
    #[ignore = "needs DB_URL, DB_USER, DB_PASSWORD and datagrizzly_m2"]
    async fn confirm_reads_the_marker_before_changing_the_published_pointer() {
        let root = std::env::temp_dir().join(format!("dg-release-db-{}", Uuid::new_v4()));
        std::env::set_var("RELEASE_ROOT", &root);
        let pool = crate::db::connect_database("datagrizzly_m2").await.expect("isolated database");
        crate::db::migrate(&pool).await.expect("migrate");
        for table in ["site_release", "publication_upload", "publication_request", "publication_job", "withdrawal", "entry_denylist", "revision_media", "series_entry", "entry_taxonomy", "media", "taxonomy", "project_log", "revision", "entry", "project", "audit_event", "admin_session", "owner"] {
            sqlx::query(&format!("DELETE FROM {table}")).execute(&pool).await.expect("clear");
        }
        let password = crate::auth::random_token();
        sqlx::query("INSERT INTO owner (password_hash) VALUES ($1)")
            .bind(crate::auth::hash_password(&password).expect("hash"))
            .execute(&pool)
            .await
            .expect("owner");
        let logged_in = hit(&pool, "POST", "/api/admin/session", None, None, &format!(r#"{{"password":"{password}"}}"#)).await;
        let session = logged_in.cookie.expect("cookie").split(';').next().expect("pair").to_string();
        let csrf = json_of(&logged_in.body)["csrfToken"].as_str().expect("csrf").to_string();
        let created = hit(&pool, "POST", "/api/admin/entries", Some(&session), Some(&csrf), r#"{"slug":"release-note","title":"현장 메모","kind":"note"}"#).await;
        assert_eq!(created.status, axum::http::StatusCode::CREATED);
        let entry_id = json_of(&created.body)["id"].as_str().expect("id").to_string();
        let current = hit(&pool, "GET", &format!("/api/admin/entries/{entry_id}"), Some(&session), Some(&csrf), "").await;
        let revision_id = json_of(&current.body)["draftRevisionId"].as_str().expect("draft").to_string();
        let accepted = hit(
            &pool,
            "POST",
            &format!("/api/admin/entries/{entry_id}/publications"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"schemaVersion":1,"revisionId":"{revision_id}","idempotencyKey":"edit.release-1"}}"#),
        )
        .await;
        assert_eq!(accepted.status, axum::http::StatusCode::ACCEPTED);
        let job_id = json_of(&accepted.body)["jobId"].as_str().expect("job").to_string();
        let polled = hit(&pool, "POST", "/api/admin/publication-jobs/poll", Some(&session), Some(&csrf), "").await;
        assert_eq!(json_of(&polled.body)["succeeded"], false);
        let confirmed = hit(&pool, "POST", &format!("/api/admin/publication-jobs/{job_id}/release"), Some(&session), Some(&csrf), "").await;
        let confirmed_json = json_of(&confirmed.body);
        assert_eq!(confirmed_json["succeeded"], true);
        assert_eq!(confirmed_json["state"], "succeeded");
        let published: uuid::Uuid = sqlx::query_scalar("SELECT published_revision_id FROM entry WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&entry_id).expect("entry"))
            .fetch_one(&pool)
            .await
            .expect("published");
        assert_eq!(published, uuid::Uuid::parse_str(&revision_id).expect("revision"));
        let release_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM site_release WHERE job_id = $1")
            .bind(uuid::Uuid::parse_str(&job_id).expect("job"))
            .fetch_one(&pool)
            .await
            .expect("release");
        let token = std::fs::read_to_string(root.join(release_id.to_string()).join("build-token.txt")).expect("token");
        let snapshot = hit_token(&pool, &format!("/api/publish/snapshots/{release_id}"), &token).await;
        assert_eq!(snapshot.status, axum::http::StatusCode::OK);
        assert!(snapshot.body.contains("현장 메모"));
        let saved = hit(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1,"title":"바뀐 제목","summary":"","blocks":[{"id":"11111111-1111-4111-8111-111111111111","type":"paragraph","text":"나중 문장"}],"metadata":{"type":"note"}}"#,
        )
        .await;
        assert_eq!(saved.status, axum::http::StatusCode::OK);
        let again = hit_token(&pool, &format!("/api/publish/snapshots/{release_id}"), &token).await;
        assert!(again.body.contains("현장 메모"));
        assert!(!again.body.contains("바뀐 제목"));
        let wrong = hit_token(&pool, &format!("/api/publish/snapshots/{release_id}"), "not-the-token").await;
        assert_eq!(wrong.status, axum::http::StatusCode::FORBIDDEN);
        sqlx::query("UPDATE publication_job SET state = 'deploying', release_id = NULL WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&job_id).expect("job"))
            .execute(&pool)
            .await
            .expect("rewind job");
        sqlx::query("UPDATE site_release SET phase = 'candidate', confirmed_at = NULL WHERE id = $1")
            .bind(release_id)
            .execute(&pool)
            .await
            .expect("rewind release");
        sqlx::query("UPDATE entry SET published_revision_id = NULL WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&entry_id).expect("entry"))
            .execute(&pool)
            .await
            .expect("rewind pointer");
        let reconciled = hit(&pool, "POST", "/api/admin/publication-jobs/reconcile", Some(&session), Some(&csrf), "").await;
        assert_eq!(json_of(&reconciled.body)["action"], "record_success");
        assert_eq!(json_of(&reconciled.body)["succeeded"], true);
        std::fs::write(root.join(release_id.to_string()).join("release.txt"), "other\n").expect("bad marker");
        sqlx::query("UPDATE publication_job SET state = 'deploying', release_id = NULL WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&job_id).expect("job"))
            .execute(&pool)
            .await
            .expect("rewind job");
        sqlx::query("UPDATE site_release SET phase = 'candidate', confirmed_at = NULL WHERE id = $1")
            .bind(release_id)
            .execute(&pool)
            .await
            .expect("rewind release");
        sqlx::query("UPDATE entry SET published_revision_id = NULL WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&entry_id).expect("entry"))
            .execute(&pool)
            .await
            .expect("rewind pointer");
        let restored = hit(&pool, "POST", "/api/admin/publication-jobs/reconcile", Some(&session), Some(&csrf), "").await;
        assert_eq!(json_of(&restored.body)["succeeded"], false);
        assert_eq!(json_of(&restored.body)["action"], "restore_previous");
        let still_empty: Option<uuid::Uuid> = sqlx::query_scalar("SELECT published_revision_id FROM entry WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&entry_id).expect("entry"))
            .fetch_one(&pool)
            .await
            .expect("published");
        assert!(still_empty.is_none());
        let drafty = hit(&pool, "POST", "/api/admin/entries", Some(&session), Some(&csrf), r#"{"slug":"draft-note","title":"초안 글","kind":"note"}"#).await;
        let draft_entry = json_of(&drafty.body)["id"].as_str().expect("id").to_string();
        let draft_saved = hit(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{draft_entry}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1,"title":"초안 글","summary":"","blocks":[{"id":"22222222-2222-4222-8222-222222222222","type":"paragraph","text":"data-draft"}],"metadata":{"type":"note"}}"#,
        )
        .await;
        let draft_revision = json_of(&draft_saved.body)["draftRevisionId"].as_str().expect("draft").to_string();
        let draft_job = hit(
            &pool,
            "POST",
            &format!("/api/admin/entries/{draft_entry}/publications"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"schemaVersion":1,"revisionId":"{draft_revision}","idempotencyKey":"edit.release-2"}}"#),
        )
        .await;
        let draft_job_id = json_of(&draft_job.body)["jobId"].as_str().expect("job").to_string();
        sqlx::query("UPDATE publication_job SET state = 'canceled' WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&job_id).expect("job"))
            .execute(&pool)
            .await
            .expect("park");
        let _ = hit(&pool, "POST", "/api/admin/publication-jobs/poll", Some(&session), Some(&csrf), "").await;
        let rejected = hit(&pool, "POST", &format!("/api/admin/publication-jobs/{draft_job_id}/release"), Some(&session), Some(&csrf), "").await;
        assert_eq!(json_of(&rejected.body)["succeeded"], false);
        assert_eq!(json_of(&rejected.body)["failureCode"], "draft_included");
        let draft_published: Option<uuid::Uuid> = sqlx::query_scalar("SELECT published_revision_id FROM entry WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&draft_entry).expect("entry"))
            .fetch_one(&pool)
            .await
            .expect("published");
        assert!(draft_published.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    fn json_of(body: &str) -> serde_json::Value {
        serde_json::from_str(body).expect("json")
    }

    struct Hit {
        status: axum::http::StatusCode,
        body: String,
        cookie: Option<String>,
    }

    async fn hit(pool: &sqlx::PgPool, method: &str, uri: &str, cookie: Option<&str>, csrf: Option<&str>, body: &str) -> Hit {
        use axum::body::Body;
        use axum::http::Request;
        use http_body_util::BodyExt;
        use tower::ServiceExt;
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder = builder.header("cookie", cookie);
        }
        if let Some(csrf) = csrf {
            builder = builder.header("x-csrf-token", csrf);
        }
        let response = crate::app(pool.clone())
            .oneshot(builder.body(Body::from(body.to_string())).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let set_cookie = response.headers().get("set-cookie").and_then(|value| value.to_str().ok()).map(str::to_string);
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        Hit { status, body: String::from_utf8(bytes.to_vec()).expect("utf-8"), cookie: set_cookie }
    }

    async fn hit_token(pool: &sqlx::PgPool, uri: &str, token: &str) -> Hit {
        use axum::body::Body;
        use axum::http::Request;
        use http_body_util::BodyExt;
        use tower::ServiceExt;
        let response = crate::app(pool.clone())
            .oneshot(Request::builder().uri(uri).header("x-build-token", token).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        Hit { status, body: String::from_utf8(bytes.to_vec()).expect("utf-8"), cookie: None }
    }
}
