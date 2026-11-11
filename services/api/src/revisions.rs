//! 개념: 초안 미리보기와 수정 이력.
//! 목적: 인증된 요청만 초안을 보게 하고, 복원은 새 수정본만 만들며 공개본은 건드리지 않는다.
//! 사용처: `/api/admin/entries/{id}/preview`, `/revisions`, `/restore`.

use crate::admin::{audit, authorize, conflict, invalid, not_found, unavailable};
use crate::content::utc_stamp;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// 개념: 복원이 글을 고치는 SQL.
/// 목적: 버전, 제목, 초안 포인터만 바꾼다. 공개 포인터 열은 이 문장에 없다.
/// 사용처: `restore_revision`. 단위 시험이 이 문장을 검사한다.
pub const RESTORE_UPDATE: &str = "UPDATE entry SET version = version + 1, title = $2, draft_revision_id = $3, updated_at = now() WHERE id = $1 AND version = $4";

/// 개념: 수정 이력 한 페이지의 조회.
/// 목적: 본문 JSON은 고르지 않는다. 작성자가 없으면 '기록 없음', 있으면 '소유자'다.
/// 공개 포인터가 비어 있으면 등호 비교가 NULL이 되므로 false로 바꿔 목록이 실패하지 않게 한다.
/// 사용처: `list_revisions`.
const LIST_SQL: &str = "SELECT revision.id, revision.version,
        to_char(revision.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
        CASE WHEN revision.created_by IS NULL THEN '기록 없음' ELSE '소유자' END,
        COALESCE(revision.id = entry.draft_revision_id, false),
        COALESCE(revision.id = entry.published_revision_id, false)
 FROM revision
 JOIN entry ON entry.id = revision.entry_id
 WHERE revision.entry_id = $1
 ORDER BY revision.version DESC
 LIMIT $2 OFFSET $3";

/// 개념: 이력 목록 한 페이지의 행 수.
/// 목적: 본문 없이 가벼운 목록만 브라우저에 보낸다.
/// 사용처: `list_revisions`의 LIMIT.
const PAGE_SIZE: i64 = 8;

/// 개념: 미리보기와 이력 경로.
/// 목적: 초안 조회, 목록, 비교용 한 건, 복원을 소유자 API에 붙인다.
/// 사용처: `main`의 `app`.
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/admin/entries/{id}/preview", get(preview_entry))
        .route("/api/admin/entries/{id}/revisions", get(list_revisions))
        .route("/api/admin/entries/{id}/revisions/{revision_id}", get(read_revision))
        .route("/api/admin/entries/{id}/restore", post(restore_revision))
}

/// 개념: 이력 목록의 페이지 번호.
/// 목적: 없으면 1페이지로 보고, 호출자가 너무 큰 번호를 넣어도 100에서 멈춘다.
/// 사용처: `GET /revisions?page=`.
#[derive(Deserialize)]
struct PageQuery {
    page: Option<i32>,
}

/// 개념: 복원 요청 본문.
/// 목적: 어느 수정본을 복사할지와, 그 사이 다른 저장이 없었는지를 함께 받는다.
/// 사용처: `POST /restore`.
#[derive(Deserialize)]
struct RestoreBody {
    #[serde(rename = "revisionId")]
    revision_id: String,
    #[serde(rename = "expectedVersion")]
    expected_version: i32,
}

/// 개념: 복원할 수정본의 새 문서.
/// 목적: 블록은 복사하고, 새 수정본 ID와 작성 시각만 바꾼다. 원본 문자열은 호출자가 보관한다.
/// 사용처: `restore_revision`.
pub fn restored_document(source: &str, new_id: Uuid, created_at: &str) -> Result<(String, String), ()> {
    // 원본 문자열은 파싱한 값과 분리된다. 과거 수정본 행은 이 함수가 바꾸지 않는다.
    let mut value: Value = serde_json::from_str(source).map_err(|_| ())?;
    let object = value.as_object_mut().ok_or(())?;
    let title = object.get("title").and_then(Value::as_str).unwrap_or("").to_string();
    // 제목이나 블록 배열이 없으면 새 초안으로 삼지 않는다.
    if title.is_empty() || object.get("blocks").and_then(Value::as_array).is_none() {
        return Err(());
    }
    object.insert("revisionId".to_string(), Value::String(new_id.to_string()));
    object.insert("createdAt".to_string(), Value::String(created_at.to_string()));
    Ok((value.to_string(), title))
}

/// 개념: 인증된 초안 미리보기.
/// 목적: 현재 초안 문서를 돌려주고, 캐시와 검색 색인에는 넣지 않는다. 접근 통제는 세션이다.
/// 사용처: `GET /api/admin/entries/{id}/preview`.
async fn preview_entry(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    // 세션이 없으면 초안 본문을 읽기 전에 거절한다.
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (String, String, String, String, i32, Uuid, String)>(
        "SELECT entry.slug, entry.kind, entry.title, entry.summary, entry.version, revision.id, revision.body::text
         FROM entry
         JOIN revision ON revision.id = entry.draft_revision_id
         WHERE entry.id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;
    let Ok(row) = row else {
        return unavailable();
    };
    // 초안 포인터가 없으면 조인 결과가 없다. 미리보기를 만들지 않는다.
    let Some((slug, kind, title, summary, version, revision_id, body)) = row else {
        return not_found();
    };
    let Ok(document) = serde_json::from_str::<Value>(&body) else {
        return unavailable();
    };
    private_json(
        StatusCode::OK,
        json!({
            "entryId": id.to_string(),
            "revisionId": revision_id.to_string(),
            "version": version,
            "slug": slug,
            "kind": kind,
            "title": title,
            "summary": summary,
            "noindex": true,
            "document": document,
        })
        .to_string(),
        true,
    )
}

/// 개념: 수정 이력 목록.
/// 목적: 수정본 ID, 시각, 작성자, 현재 초안·공개본 여부만 페이지로 돌려준다.
/// 사용처: `GET /api/admin/entries/{id}/revisions`.
async fn list_revisions(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, Query(query): Query<PageQuery>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let exists: Option<i32> = match sqlx::query_scalar("SELECT 1 FROM entry WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
    {
        Ok(exists) => exists,
        Err(_) => return unavailable(),
    };
    // 글이 없으면 빈 목록과 구분한다. 수정본이 없는 글은 아래에서 빈 배열이 된다.
    if exists.is_none() {
        return not_found();
    }
    let page = query.page.unwrap_or(1).clamp(1, 100);
    let offset = i64::from(page - 1) * PAGE_SIZE;
    let total: i64 = match sqlx::query_scalar("SELECT count(*) FROM revision WHERE entry_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
    {
        Ok(total) => total,
        Err(_) => return unavailable(),
    };
    let rows = sqlx::query_as::<_, (Uuid, i32, String, String, bool, bool)>(LIST_SQL)
        .bind(id)
        .bind(PAGE_SIZE)
        .bind(offset)
        .fetch_all(&pool)
        .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let revisions = rows
        .into_iter()
        .map(|(revision_id, version, created_at, author, draft, published)| {
            json!({
                "id": revision_id.to_string(),
                "version": version,
                "createdAt": created_at,
                "author": author,
                "draft": draft,
                "published": published,
            })
        })
        .collect::<Vec<_>>();
    private_json(
        StatusCode::OK,
        json!({"revisions": revisions, "page": page, "pageSize": PAGE_SIZE, "total": total}).to_string(),
        true,
    )
}

/// 개념: 비교용 수정본 한 건.
/// 목적: 목록에는 본문을 넣지 않고, 화면이 고른 수정본만 따로 읽게 한다.
/// 사용처: `GET /api/admin/entries/{id}/revisions/{revision_id}`.
async fn read_revision(State(pool): State<PgPool>, headers: HeaderMap, Path((id, revision_id)): Path<(String, String)>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Some(revision_id) = parse_id(&revision_id) else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (i32, String)>(
        "SELECT version, body::text FROM revision WHERE id = $1 AND entry_id = $2",
    )
    .bind(revision_id)
    .bind(id)
    .fetch_optional(&pool)
    .await;
    let Ok(row) = row else {
        return unavailable();
    };
    // 다른 글의 수정본 ID로는 본문을 주지 않는다.
    let Some((version, body)) = row else {
        return not_found();
    };
    let Ok(document) = serde_json::from_str::<Value>(&body) else {
        return unavailable();
    };
    private_json(
        StatusCode::OK,
        json!({"id": revision_id.to_string(), "version": version, "document": document}).to_string(),
        true,
    )
}

/// 개념: 고른 수정본을 새 초안으로 복사한다.
/// 목적: 과거 행과 공개 포인터는 그대로 두고, 맞는 버전일 때만 초안을 옮긴다.
/// 사용처: `POST /api/admin/entries/{id}/restore`.
async fn restore_revision(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<RestoreBody>(&body) else {
        return invalid();
    };
    let Some(source_id) = parse_id(&input.revision_id) else {
        return invalid();
    };
    if input.expected_version < 1 {
        return invalid();
    }
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    // 글을 잠그고 공개 포인터는 읽기만 한다. 이 값은 응답에 다시 넣으며 UPDATE에는 쓰지 않는다.
    let locked = sqlx::query_as::<_, (i32, Option<Uuid>)>(
        "SELECT version, published_revision_id FROM entry WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(tx.as_mut())
    .await;
    let Ok(locked) = locked else {
        return unavailable();
    };
    let Some((version, published_id)) = locked else {
        return not_found();
    };
    // 그 사이 다른 저장이 있으면 409다. 초안과 공개 포인터는 그대로 둔다.
    if version != input.expected_version {
        let _ = tx.rollback().await;
        return conflict();
    }
    let source: Option<String> = match sqlx::query_scalar(
        "SELECT body::text FROM revision WHERE id = $1 AND entry_id = $2",
    )
    .bind(source_id)
    .bind(id)
    .fetch_optional(tx.as_mut())
    .await
    {
        Ok(source) => source,
        Err(_) => return unavailable(),
    };
    let Some(source) = source else {
        let _ = tx.rollback().await;
        return not_found();
    };
    // 새 수정본 ID다. 고른 과거 행의 ID를 재사용하지 않는다.
    let new_id = Uuid::new_v4();
    let created_at = utc_stamp();
    let Ok((document, title)) = restored_document(&source, new_id, &created_at) else {
        let _ = tx.rollback().await;
        return invalid();
    };
    let inserted = sqlx::query(
        "INSERT INTO revision (id, entry_id, version, body, created_by) VALUES ($1, $2, $3, $4::jsonb, $5)",
    )
    .bind(new_id)
    .bind(id)
    .bind(version + 1)
    .bind(&document)
    .bind(principal.owner_id)
    .execute(tx.as_mut())
    .await;
    let updated = sqlx::query(RESTORE_UPDATE)
        .bind(id)
        .bind(&title)
        .bind(new_id)
        .bind(version)
        .execute(tx.as_mut())
        .await;
    if inserted.is_err() || updated.as_ref().map(|result| result.rows_affected()).unwrap_or(0) != 1 {
        let _ = tx.rollback().await;
        return unavailable();
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    // 감사 기록 실패는 이미 커밋된 복원을 되돌리지 않는다.
    let _ = audit(&pool, Some(principal.owner_id), "revision_restored").await;
    private_json(
        StatusCode::OK,
        json!({
            "id": id.to_string(),
            "version": version + 1,
            "draftRevisionId": new_id.to_string(),
            "publishedRevisionId": published_id.map(|value| value.to_string()),
        })
        .to_string(),
        true,
    )
}

/// 개념: 경로의 글 ID와 수정본 ID.
/// 목적: UUID가 아니면 없는 자원으로 보고 본문을 조회하지 않는다.
/// 사용처: 미리보기, 목록, 조회, 복원.
fn parse_id(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

/// 개념: 초안 응답의 공통 헤더.
/// 목적: 브라우저와 중간 캐시에 남기지 않고, 미리보기와 이력은 검색 로봇에도 알린다.
/// 사용처: 미리보기, 목록, 한 건 조회, 복원 성공 본문. 401과 403은 `authorize`의 응답을 그대로 쓴다.
fn private_json(status: StatusCode, body: String, noindex: bool) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        // 초안과 이력은 공유 캐시에 남기지 않는다.
        .header("cache-control", "no-store");
    // noindex는 접근 통제가 아니다. 세션 검사가 먼저이고, 이 헤더는 검색 색인에 넣지 말라는 보조 표시다.
    if noindex {
        builder = builder.header("x-robots-tag", "noindex");
    }
    builder.body(Body::from(body)).expect("response body")
}

#[cfg(test)]
mod tests {
    use super::{restored_document, LIST_SQL, RESTORE_UPDATE};
    use axum::http::StatusCode;
    use uuid::Uuid;

    /// 개념: 목록과 복원 SQL의 경계.
    /// 목적: 목록이 본문을 고르지 않고, 복원 UPDATE가 공개 포인터를 쓰지 않는지 DB 없이 확인한다.
    /// 사용처: `cargo test`.
    #[test]
    fn list_omits_body_and_restore_leaves_published_pointer() {
        assert!(!LIST_SQL.to_lowercase().contains("body"));
        assert!(RESTORE_UPDATE.contains("draft_revision_id"));
        assert!(!RESTORE_UPDATE.contains("published_revision_id"));
    }

    /// 개념: 복원 문서 복사.
    /// 목적: 새 ID와 시각만 바꾸고, 호출자가 가진 원문 문자열과 블록 문장은 유지한다.
    /// 사용처: `cargo test`.
    #[test]
    fn restore_copies_blocks_and_keeps_the_source_text() {
        let source = r#"{"schemaVersion":1,"revisionId":"11111111-1111-4111-8111-111111111111","entryId":"22222222-2222-4222-8222-222222222222","createdAt":"2026-11-01T00:00:00Z","title":"현장 메모","canonicalUrl":"https://datagrizzly.local/notes/field-note","blocks":[{"id":"33333333-3333-4333-8333-333333333333","type":"paragraph","text":"저장한 문장"}],"metadata":{"type":"note"}}"#;
        let original = source.to_string();
        let new_id = Uuid::from_u128(9);
        let (copy, title) = restored_document(source, new_id, "2026-11-11T00:00:00Z").expect("copy");
        assert_eq!(source, original);
        assert_eq!(title, "현장 메모");
        assert!(copy.contains(&new_id.to_string()));
        assert!(copy.contains("저장한 문장"));
        assert!(!copy.contains("11111111-1111-4111-8111-111111111111"));
    }

    /// 개념: 격리 DB에서 미리보기와 복원 흐름.
    /// 목적: 익명 거절, no-store, 새 수정본, 원문 유지, 공개 포인터 유지, 오래된 버전의 409를 확인한다.
    /// 사용처: Docker의 `datagrizzly_m2`. 개발 DB `datagrizzly`의 행은 지우지 않는다. 포트가 닫히면 무시 상태로 남긴다.
    #[tokio::test]
    #[ignore = "needs DB_URL, DB_USER, DB_PASSWORD and datagrizzly_m2"]
    async fn preview_and_restore_keep_the_published_pointer() {
        // 시험 데이터는 격리 DB에만 넣는다.
        let pool = crate::db::connect_database("datagrizzly_m2")
            .await
            .expect("isolated database");
        crate::db::migrate(&pool).await.expect("migrate");
        for table in [
            "revision_media",
            "series_entry",
            "entry_taxonomy",
            "media",
            "taxonomy",
            "project_log",
            "revision",
            "entry",
            "project",
            "audit_event",
            "admin_session",
            "owner",
        ] {
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&pool)
                .await
                .expect("clear isolated rows");
        }

        // 세션 없이 미리보기를 열면 초안을 주지 않는다.
        let password = crate::auth::random_token();
        let anonymous = hit(
            &pool,
            "GET",
            "/api/admin/entries/00000000-0000-4000-8000-000000000001/preview",
            None,
            None,
            "",
        )
        .await;
        assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);

        sqlx::query("INSERT INTO owner (password_hash) VALUES ($1)")
            .bind(crate::auth::hash_password(&password).expect("hash"))
            .execute(&pool)
            .await
            .expect("owner");
        let logged_in = hit(
            &pool,
            "POST",
            "/api/admin/session",
            None,
            None,
            &format!(r#"{{"password":"{password}"}}"#),
        )
        .await;
        assert_eq!(logged_in.status, StatusCode::OK);
        let session = logged_in.cookie.expect("cookie").split(';').next().expect("pair").to_string();
        let csrf = serde_json::from_str::<serde_json::Value>(&logged_in.body).expect("login json")["csrfToken"]
            .as_str()
            .expect("csrf")
            .to_string();

        let created = hit(
            &pool,
            "POST",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            r#"{"slug":"field-note","title":"현장 메모","kind":"note"}"#,
        )
        .await;
        assert_eq!(created.status, StatusCode::CREATED);
        let entry_id = serde_json::from_str::<serde_json::Value>(&created.body).expect("created")["id"]
            .as_str()
            .expect("id")
            .to_string();
        let current = hit(
            &pool,
            "GET",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(current.status, StatusCode::OK);
        let current_json = serde_json::from_str::<serde_json::Value>(&current.body).expect("entry");
        let source_id = current_json["draftRevisionId"].as_str().expect("draft").to_string();
        let source_body: String = sqlx::query_scalar("SELECT body::text FROM revision WHERE id = $1")
            .bind(Uuid::parse_str(&source_id).expect("source id"))
            .fetch_one(&pool)
            .await
            .expect("source body");
        // 발행 기능은 아직 없다. 공개 포인터만 첫 수정본으로 고정해 복원 뒤에도 같은지 본다.
        sqlx::query("UPDATE entry SET published_revision_id = draft_revision_id WHERE id = $1")
            .bind(Uuid::parse_str(&entry_id).expect("entry id"))
            .execute(&pool)
            .await
            .expect("mark published pointer");

        let preview = hit(
            &pool,
            "GET",
            &format!("/api/admin/entries/{entry_id}/preview"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(preview.status, StatusCode::OK);
        assert_eq!(preview.cache.as_deref(), Some("no-store"));
        assert_eq!(preview.robots.as_deref(), Some("noindex"));
        assert!(preview.body.contains("\"noindex\":true"));

        let saved = hit(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1,"title":"현장 메모","summary":"짧은 요약","blocks":[{"id":"11111111-1111-4111-8111-111111111111","type":"paragraph","text":"두번째 문장"}],"metadata":{"type":"note"}}"#,
        )
        .await;
        assert_eq!(saved.status, StatusCode::OK);

        let listed = hit(
            &pool,
            "GET",
            &format!("/api/admin/entries/{entry_id}/revisions"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(listed.status, StatusCode::OK);
        let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).expect("list");
        let rows = listed_json["revisions"].as_array().expect("rows");
        assert_eq!(rows.len(), 2);
        // 목록 항목에는 본문 키를 넣지 않고, 이번 저장의 작성자는 소유자다.
        for row in rows {
            assert!(row.get("body").is_none());
            assert!(row.get("blocks").is_none());
            assert_eq!(row["author"], "소유자");
        }

        // 첫 수정본을 새 초안으로 복사한다. 기대 버전은 방금 저장한 2다.
        let restored = hit(
            &pool,
            "POST",
            &format!("/api/admin/entries/{entry_id}/restore"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"revisionId":"{source_id}","expectedVersion":2}}"#),
        )
        .await;
        assert_eq!(restored.status, StatusCode::OK);
        let restored_json = serde_json::from_str::<serde_json::Value>(&restored.body).expect("restore");
        let new_id = restored_json["draftRevisionId"].as_str().expect("new draft").to_string();
        assert_ne!(new_id, source_id);
        assert_eq!(restored_json["publishedRevisionId"], source_id);
        assert_eq!(restored_json["version"], 3);
        let source_after: String = sqlx::query_scalar("SELECT body::text FROM revision WHERE id = $1")
            .bind(Uuid::parse_str(&source_id).expect("source id"))
            .fetch_one(&pool)
            .await
            .expect("source after");
        assert_eq!(source_after, source_body);
        let published: Uuid = sqlx::query_scalar("SELECT published_revision_id FROM entry WHERE id = $1")
            .bind(Uuid::parse_str(&entry_id).expect("entry id"))
            .fetch_one(&pool)
            .await
            .expect("published pointer");
        assert_eq!(published, Uuid::parse_str(&source_id).expect("source id"));

        // 이미 버전이 3인데 2로 다시 복원하면 거절되고 공개 포인터는 유지된다.
        let stale = hit(
            &pool,
            "POST",
            &format!("/api/admin/entries/{entry_id}/restore"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"revisionId":"{source_id}","expectedVersion":2}}"#),
        )
        .await;
        assert_eq!(stale.status, StatusCode::CONFLICT);
        let published_after: Uuid = sqlx::query_scalar("SELECT published_revision_id FROM entry WHERE id = $1")
            .bind(Uuid::parse_str(&entry_id).expect("entry id"))
            .fetch_one(&pool)
            .await
            .expect("published after conflict");
        assert_eq!(published_after, published);

        // 로그아웃한 세션으로는 같은 미리보기를 다시 열 수 없다.
        let signed_out = hit(
            &pool,
            "DELETE",
            "/api/admin/session",
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(signed_out.status, StatusCode::OK);
        let rejected = hit(
            &pool,
            "GET",
            &format!("/api/admin/entries/{entry_id}/preview"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
    }

    /// 개념: 시험 요청의 상태, 본문, 쿠키와 캐시 헤더.
    /// 목적: 미리보기의 no-store와 noindex를 본문과 함께 확인한다.
    /// 사용처: `preview_and_restore_keep_the_published_pointer`.
    struct Hit {
        status: StatusCode,
        body: String,
        cookie: Option<String>,
        cache: Option<String>,
        robots: Option<String>,
    }

    /// 개념: 앱에 요청 한 번을 보내고 응답을 모은다.
    /// 목적: 쿠키와 CSRF를 선택적으로 붙여 로그인 전과 후를 같은 방식으로 호출한다.
    /// 사용처: 격리 DB 시험.
    async fn hit(
        pool: &sqlx::PgPool,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        csrf: Option<&str>,
        body: &str,
    ) -> Hit {
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
        let headers = response.headers().clone();
        let cookie = headers.get("set-cookie").and_then(|value| value.to_str().ok()).map(str::to_string);
        let cache = headers.get("cache-control").and_then(|value| value.to_str().ok()).map(str::to_string);
        let robots = headers.get("x-robots-tag").and_then(|value| value.to_str().ok()).map(str::to_string);
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        Hit {
            status,
            body: String::from_utf8(bytes.to_vec()).expect("utf-8"),
            cookie,
            cache,
            robots,
        }
    }
}
