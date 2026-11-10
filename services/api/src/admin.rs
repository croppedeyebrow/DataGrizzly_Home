//! 개념: 소유자 전용 관리 API.
//! 목적: 세션과 CSRF가 있을 때만 글을 읽고, 같은 버전일 때만 초안을 저장한다.
//! 사용처: `app`의 `/api/admin` 경로.

use crate::auth::{
    clear_session_cookie, cookie_value, password_matches, random_token, session_cookie, sha256_hex,
    tokens_match,
};
use crate::content::{self, FieldError};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/admin/session", post(login).delete(logout))
        .route("/api/admin/entries", get(list_entries).post(create_entry))
        .route("/api/admin/entries/{id}", get(read_entry).put(save_entry))
}

#[derive(Deserialize)]
struct LoginBody {
    password: String,
}

#[derive(Deserialize)]
struct CreateEntry {
    slug: String,
    title: String,
    kind: String,
}

pub(crate) struct Principal {
    pub(crate) session_id: Uuid,
    pub(crate) owner_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    owner_id: Uuid,
    csrf_hash: String,
    fresh: bool,
    open: bool,
    disabled: bool,
}

pub async fn login(State(pool): State<PgPool>, body: String) -> Response {
    let password = serde_json::from_str::<LoginBody>(&body)
        .ok()
        .map(|body| body.password)
        .filter(|password| !password.is_empty());
    let Some(password) = password else {
        return unauthorized();
    };

    let owner = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, password_hash FROM owner WHERE disabled = false ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await;
    let Ok(owner) = owner else {
        return unavailable();
    };

    let allowed = owner
        .as_ref()
        .is_some_and(|(_, hash)| password_matches(&password, hash));
    if !allowed {
        let _ = audit(&pool, None, "login_failed").await;
        return unauthorized();
    }
    let owner_id = owner.expect("checked").0;
    match issue_session(&pool, owner_id).await {
        Ok((token, csrf)) => {
            let _ = audit(&pool, Some(owner_id), "login").await;
            json_response(
                StatusCode::OK,
                format!(r#"{{"csrfToken":"{csrf}"}}"#),
                Some(session_cookie(&token)),
            )
        }
        Err(response) => response,
    }
}

pub async fn logout(State(pool): State<PgPool>, headers: HeaderMap) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let revoked = sqlx::query(
        "UPDATE admin_session SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(principal.session_id)
    .execute(&pool)
    .await;
    if revoked.is_err() {
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "logout").await;
    json_response(
        StatusCode::OK,
        r#"{"ok":true}"#.to_string(),
        Some(clear_session_cookie()),
    )
}

async fn list_entries(State(pool): State<PgPool>, headers: HeaderMap) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let rows = sqlx::query_as::<_, (Uuid, String, String, String, i32, String, bool)>(
        "SELECT id, slug, title, kind, version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                published_revision_id IS NOT NULL
         FROM entry
         ORDER BY updated_at DESC
         LIMIT 20",
    )
    .fetch_all(&pool)
    .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let entries = rows
        .into_iter()
        .map(|(id, slug, title, kind, version, updated_at, published)| {
            format!(
                r#"{{"id":"{id}","slug":{},"title":{},"kind":{},"version":{version},"updatedAt":{},"published":{published}}}"#,
                json_string(&slug),
                json_string(&title),
                json_string(&kind),
                json_string(&updated_at),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    json_response(
        StatusCode::OK,
        format!(r#"{{"entries":[{entries}]}}"#),
        None,
    )
}

async fn read_entry(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Ok(id) = Uuid::parse_str(&id) else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (Uuid, String, String, String, String, i32, Option<Uuid>, Option<Uuid>, Option<Uuid>, String, Option<String>)>(
        "SELECT entry.id, entry.slug, entry.title, entry.summary, entry.kind, entry.version,
                entry.project_id, entry.draft_revision_id, entry.published_revision_id,
                to_char(entry.updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                revision.body::text
         FROM entry
         LEFT JOIN revision ON revision.id = entry.draft_revision_id
         WHERE entry.id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;
    let Ok(row) = row else {
        return unavailable();
    };
    let Some((id, slug, title, summary, kind, version, project_id, draft_id, published_id, updated_at, body)) = row else {
        return not_found();
    };
    let document = body.and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    json_response(
        StatusCode::OK,
        serde_json::json!({
            "id": id.to_string(),
            "slug": slug,
            "title": title,
            "summary": summary,
            "kind": kind,
            "version": version,
            "projectId": project_id.map(|value| value.to_string()),
            "draftRevisionId": draft_id.map(|value| value.to_string()),
            "publishedRevisionId": published_id.map(|value| value.to_string()),
            "updatedAt": updated_at,
            "document": document,
        })
        .to_string(),
        None,
    )
}

/// 개념: 필드 오류 응답.
/// 목적: 잘못된 블록의 경로를 알리고 저장본은 바꾸지 않는다.
/// 사용처: `save_entry`.
fn invalid_fields(errors: &[FieldError]) -> Response {
    let items = errors
        .iter()
        .map(|error| {
            format!(
                r#"{{"path":{},"code":{}}}"#,
                json_string(&error.path),
                json_string(&error.code)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    json_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        format!(r#"{{"error":"invalid","fieldErrors":[{items}]}}"#),
        None,
    )
}

async fn create_entry(State(pool): State<PgPool>, headers: HeaderMap, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Ok(input) = serde_json::from_str::<CreateEntry>(&body) else {
        return invalid();
    };
    let title = input.title.trim();
    if !valid_slug(&input.slug) || title.is_empty() || title.chars().count() > 200 {
        return invalid();
    }
    if !matches!(input.kind.as_str(), "note" | "paper" | "news" | "log") {
        return invalid();
    }

    let id = match insert_entry(&pool, &input.slug, title, &input.kind).await {
        Ok(id) => id,
        Err(response) => return response,
    };
    let _ = audit(&pool, Some(principal.owner_id), "entry_created").await;
    json_response(
        StatusCode::CREATED,
        format!(r#"{{"id":"{id}","slug":{},"version":1}}"#, json_string(&input.slug)),
        None,
    )
}

/// 개념: 글과 첫 수정본을 함께 만든다.
/// 목적: 빈 객체가 아니라 schemaVersion 1 문서를 초안으로 연결한다.
/// 사용처: `create_entry`.
async fn insert_entry(pool: &PgPool, slug: &str, title: &str, kind: &str) -> Result<Uuid, Response> {
    let revision_id = Uuid::new_v4();
    let entry_id = Uuid::new_v4();
    let document = content::revision_document(
        revision_id,
        entry_id,
        title,
        &content::canonical_url(kind, slug),
        &[],
        &content::initial_metadata(kind),
    );
    let body = document.to_string();
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return Err(unavailable()),
    };
    let created = async {
        sqlx::query("INSERT INTO entry (id, slug, title, kind, summary) VALUES ($1, $2, $3, $4, '')")
            .bind(entry_id)
            .bind(slug)
            .bind(title)
            .bind(kind)
            .execute(tx.as_mut())
            .await
            .map_err(|error| if is_unique(&error) { conflict() } else { unavailable() })?;
        sqlx::query("INSERT INTO revision (id, entry_id, version, body) VALUES ($1, $2, 1, $3::jsonb)")
            .bind(revision_id)
            .bind(entry_id)
            .bind(&body)
            .execute(tx.as_mut())
            .await
            .map_err(|_| unavailable())?;
        sqlx::query("UPDATE entry SET draft_revision_id = $1 WHERE id = $2")
            .bind(revision_id)
            .bind(entry_id)
            .execute(tx.as_mut())
            .await
            .map_err(|_| unavailable())?;
        Ok(entry_id)
    }
    .await;
    match created {
        Ok(id) => {
            if tx.commit().await.is_err() {
                return Err(unavailable());
            }
            Ok(id)
        }
        Err(response) => {
            let _ = tx.rollback().await;
            Err(response)
        }
    }
}

/// 개념: 초안 저장.
/// 목적: expectedVersion이 현재 버전과 같을 때만 새 수정본을 남긴다.
/// 사용처: PUT /api/admin/entries/{id}.
async fn save_entry(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Ok(id) = Uuid::parse_str(&id) else {
        return not_found();
    };
    let current = sqlx::query_as::<_, (String, String)>(
        "SELECT kind, slug FROM entry WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;
    let Ok(current) = current else {
        return unavailable();
    };
    let Some((kind, slug)) = current else {
        return not_found();
    };
    let input = match content::parse_save(&body, &kind) {
        Ok(input) => input,
        Err(errors) => return invalid_fields(&errors),
    };
    let revision_id = Uuid::new_v4();
    let document = content::revision_document(
        revision_id,
        id,
        &input.title,
        &content::canonical_url(&kind, &slug),
        &input.blocks,
        &input.metadata,
    );
    let saved = sqlx::query_as::<_, (Option<Uuid>, Option<i32>, Option<i32>, Option<Uuid>)>(
        "WITH locked AS (
            SELECT id, version FROM entry WHERE id = $2 FOR UPDATE
         ),
         inserted AS (
            INSERT INTO revision (id, entry_id, version, body)
            SELECT $1, id, version + 1, $4::jsonb
            FROM locked
            WHERE version = $3
            RETURNING id, entry_id, version
         ),
         updated AS (
            UPDATE entry AS target
            SET version = inserted.version,
                title = $5,
                summary = $6,
                draft_revision_id = inserted.id,
                updated_at = now()
            FROM inserted
            WHERE target.id = inserted.entry_id
            RETURNING target.version, target.draft_revision_id
         )
         SELECT
            (SELECT id FROM locked),
            (SELECT version FROM locked),
            (SELECT version FROM updated),
            (SELECT draft_revision_id FROM updated)",
    )
    .bind(revision_id)
    .bind(id)
    .bind(input.expected_version)
    .bind(document.to_string())
    .bind(&input.title)
    .bind(&input.summary)
    .fetch_one(&pool)
    .await;
    let Ok((entry_id, current_version, saved_version, draft_id)) = saved else {
        return unavailable();
    };
    let Some(_) = entry_id else {
        return not_found();
    };
    let Some(saved_version) = saved_version else {
        return conflict_document(&pool, id, current_version.unwrap_or(input.expected_version)).await;
    };
    let Some(draft_id) = draft_id else {
        return unavailable();
    };
    json_response(
        StatusCode::OK,
        format!(r#"{{"id":"{id}","version":{saved_version},"draftRevisionId":"{draft_id}"}}"#),
        None,
    )
}

/// 개념: 충돌한 저장의 서버 본문.
/// 목적: 클라이언트가 최신본과 로컬 편집본을 비교하게 하고, 서버 본문을 바꾸지 않는다.
/// 사용처: `save_entry`의 409.
async fn conflict_document(pool: &PgPool, id: Uuid, version: i32) -> Response {
    let row = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT entry.title, entry.summary, revision.body::text
         FROM entry
         LEFT JOIN revision ON revision.id = entry.draft_revision_id
         WHERE entry.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await;
    let Ok(Some((title, summary, body))) = row else {
        return conflict();
    };
    let document = body
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    json_response(
        StatusCode::CONFLICT,
        serde_json::json!({
            "error": "conflict",
            "version": version,
            "title": title,
            "summary": summary,
            "document": document,
        })
        .to_string(),
        None,
    )
}

async fn issue_session(pool: &PgPool, owner_id: Uuid) -> Result<(String, String), Response> {
    let token = random_token();
    let csrf = random_token();
    let inserted = sqlx::query(
        "INSERT INTO admin_session (owner_id, token_hash, csrf_hash, expires_at)
         VALUES ($1, $2, $3, now() + interval '12 hours')",
    )
    .bind(owner_id)
    .bind(sha256_hex(&token))
    .bind(sha256_hex(&csrf))
    .execute(pool)
    .await;
    if inserted.is_err() {
        return Err(unavailable());
    }
    Ok((token, csrf))
}

pub(crate) async fn authorize(pool: &PgPool, headers: &HeaderMap) -> Result<Principal, Response> {
    let Some(token) = cookie_value(
        headers.get("cookie").and_then(|value| value.to_str().ok()),
        "dg_session",
    ) else {
        return Err(unauthorized());
    };
    let row = sqlx::query_as::<_, SessionRow>(
        "SELECT s.id, s.owner_id, s.csrf_hash,
                (s.expires_at > now()) AS fresh,
                (s.revoked_at IS NULL) AS open,
                o.disabled
         FROM admin_session s
         JOIN owner o ON o.id = s.owner_id
         WHERE s.token_hash = $1",
    )
    .bind(sha256_hex(token))
    .fetch_optional(pool)
    .await
    .map_err(|_| unavailable())?;
    let Some(row) = row else {
        return Err(unauthorized());
    };
    if !row.fresh || !row.open {
        return Err(unauthorized());
    }
    if row.disabled {
        return Err(forbidden());
    }
    let csrf = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if csrf.is_empty() || !tokens_match(&sha256_hex(csrf), &row.csrf_hash) {
        return Err(forbidden());
    }
    Ok(Principal {
        session_id: row.id,
        owner_id: row.owner_id,
    })
}

pub(crate) async fn audit(pool: &PgPool, owner_id: Option<Uuid>, action: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO audit_event (owner_id, action) VALUES ($1, $2)")
        .bind(owner_id)
        .bind(action)
        .execute(pool)
        .await
        .map(|_| ())
}

pub(crate) fn valid_slug(slug: &str) -> bool {
    let bytes = slug.as_bytes();
    if bytes.is_empty() || bytes.len() > 80 || bytes[0] == b'-' || bytes[bytes.len() - 1] == b'-' {
        return false;
    }
    let mut previous_dash = false;
    for byte in bytes {
        let allowed = byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-';
        if !allowed || (*byte == b'-' && previous_dash) {
            return false;
        }
        previous_dash = *byte == b'-';
    }
    true
}

pub(crate) fn is_unique(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database| database.code())
        .is_some_and(|code| code == "23505")
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

pub(crate) fn json_response(status: StatusCode, body: String, cookie: Option<String>) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8");
    if let Some(cookie) = cookie {
        builder = builder.header("set-cookie", cookie);
    }
    builder.body(Body::from(body)).expect("response body")
}

pub(crate) fn unauthorized() -> Response {
    json_response(
        StatusCode::UNAUTHORIZED,
        r#"{"error":"unauthorized"}"#.to_string(),
        None,
    )
}

pub(crate) fn forbidden() -> Response {
    json_response(
        StatusCode::FORBIDDEN,
        r#"{"error":"forbidden"}"#.to_string(),
        None,
    )
}

pub(crate) fn not_found() -> Response {
    json_response(
        StatusCode::NOT_FOUND,
        r#"{"error":"not_found"}"#.to_string(),
        None,
    )
}

pub(crate) fn invalid() -> Response {
    json_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        r#"{"error":"invalid"}"#.to_string(),
        None,
    )
}

pub(crate) fn conflict() -> Response {
    json_response(
        StatusCode::CONFLICT,
        r#"{"error":"conflict"}"#.to_string(),
        None,
    )
}

pub(crate) fn unavailable() -> Response {
    json_response(
        StatusCode::SERVICE_UNAVAILABLE,
        r#"{"error":"unavailable"}"#.to_string(),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::valid_slug;
    use crate::auth::{hash_password, random_token, sha256_hex};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    #[test]
    fn slug_rejects_uppercase_and_double_dash() {
        assert!(valid_slug("field-note"));
        assert!(!valid_slug("Field"));
        assert!(!valid_slug("field--note"));
        assert!(!valid_slug("-field"));
    }

    #[test]
    fn schema_meta_migration_is_unchanged() {
        let text = include_str!("../migrations/0001_schema_meta.sql");
        assert!(text.contains("CREATE TABLE schema_meta"));
        assert!(text.contains("VALUES (1)"));
        assert!(!text.to_lowercase().contains("owner"));
        assert!(!text.to_lowercase().contains("drop"));
    }

    #[tokio::test]
    #[ignore]
    async fn owner_access_uses_the_isolated_database() {
        let dev = crate::db::connect_database("datagrizzly")
            .await
            .expect("dev database");
        let dev_name: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&dev)
            .await
            .expect("dev name");
        assert_eq!(dev_name, "datagrizzly");
        crate::db::migrate(&dev).await.expect("dev migration");
        let version: i32 = sqlx::query_scalar("SELECT schema_version FROM schema_meta LIMIT 1")
            .fetch_one(&dev)
            .await
            .expect("schema version");
        assert_eq!(version, 1);

        let pool = crate::db::connect_database("datagrizzly_m2")
            .await
            .expect("isolated database");
        let name: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&pool)
            .await
            .expect("database name");
        assert_eq!(name, "datagrizzly_m2");
        crate::db::migrate(&pool).await.expect("isolated migration");
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

        let password = random_token();
        let missing = call(
            &pool,
            "POST",
            "/api/admin/session",
            None,
            None,
            &format!(r#"{{"password":"{password}"}}"#),
        )
        .await;
        assert_eq!(missing.0, StatusCode::UNAUTHORIZED);
        assert_eq!(missing.1, r#"{"error":"unauthorized"}"#);

        let owner_id: Uuid = sqlx::query_scalar(
            "INSERT INTO owner (password_hash) VALUES ($1) RETURNING id",
        )
        .bind(hash_password(&password).expect("hash"))
        .fetch_one(&pool)
        .await
        .expect("owner");

        let wrong = call(
            &pool,
            "POST",
            "/api/admin/session",
            None,
            None,
            r#"{"password":"not-the-owner-password"}"#,
        )
        .await;
        assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong.1, missing.1);

        let logged_in = call(
            &pool,
            "POST",
            "/api/admin/session",
            None,
            None,
            &format!(r#"{{"password":"{password}"}}"#),
        )
        .await;
        assert_eq!(logged_in.0, StatusCode::OK);
        let cookie = logged_in.2.expect("session cookie");
        assert!(cookie.contains("dg_session="));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(!cookie.contains("Secure"));
        let session = cookie.split(';').next().expect("cookie pair").to_string();
        let csrf = logged_in
            .1
            .split("csrfToken\":\"")
            .nth(1)
            .and_then(|value| value.split('"').next())
            .expect("csrf")
            .to_string();

        let anonymous = call(&pool, "GET", "/api/admin/entries", None, None, "").await;
        assert_eq!(anonymous.0, StatusCode::UNAUTHORIZED);

        let hidden = call(
            &pool,
            "GET",
            "/api/admin/entries/00000000-0000-0000-0000-000000000000",
            None,
            None,
            "",
        )
        .await;
        assert_eq!(hidden.0, StatusCode::UNAUTHORIZED);

        let cookie_only = call(
            &pool,
            "GET",
            "/api/admin/entries",
            Some(&session),
            None,
            "",
        )
        .await;
        assert_eq!(cookie_only.0, StatusCode::FORBIDDEN);

        let wrong_csrf = call(
            &pool,
            "GET",
            "/api/admin/entries",
            Some(&session),
            Some("wrong-csrf"),
            "",
        )
        .await;
        assert_eq!(wrong_csrf.0, StatusCode::FORBIDDEN);

        let empty = call(
            &pool,
            "GET",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(empty.0, StatusCode::OK);
        assert_eq!(empty.1, r#"{"entries":[]}"#);

        let invalid = call(
            &pool,
            "POST",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            r#"{"slug":"Bad","title":"메모","kind":"note"}"#,
        )
        .await;
        assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);

        let created = call(
            &pool,
            "POST",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            r#"{"slug":"field-note","title":"현장 메모","kind":"note"}"#,
        )
        .await;
        assert_eq!(created.0, StatusCode::CREATED);
        let duplicate = call(
            &pool,
            "POST",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            r#"{"slug":"field-note","title":"현장 메모","kind":"note"}"#,
        )
        .await;
        assert_eq!(duplicate.0, StatusCode::CONFLICT);

        let missing_entry = call(
            &pool,
            "GET",
            "/api/admin/entries/00000000-0000-0000-0000-000000000000",
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(missing_entry.0, StatusCode::NOT_FOUND);
        assert_eq!(missing_entry.1, r#"{"error":"not_found"}"#);

        let entry_id = created
            .1
            .split("\"id\":\"")
            .nth(1)
            .and_then(|value| value.split('"').next())
            .expect("entry id");
        let saved = call(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1,"title":"현장 메모","summary":"짧은 요약","blocks":[{"id":"11111111-1111-4111-8111-111111111111","type":"paragraph","text":"저장한 문장"}],"metadata":{"type":"note"}}"#,
        )
        .await;
        assert_eq!(saved.0, StatusCode::OK);
        assert!(saved.1.contains("\"version\":2"));
        let stale = call(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1,"title":"덮어쓴 제목","summary":"","blocks":[{"id":"22222222-2222-4222-8222-222222222222","type":"paragraph","text":"오래된 탭"}],"metadata":{"type":"note"}}"#,
        )
        .await;
        assert_eq!(stale.0, StatusCode::CONFLICT);
        assert!(stale.1.contains("저장한 문장"));
        assert!(!stale.1.contains("오래된 탭"));
        let version: i32 = sqlx::query_scalar("SELECT version FROM entry WHERE slug = 'field-note'")
            .fetch_one(&pool)
            .await
            .expect("version");
        assert_eq!(version, 2);
        let invalid_block = call(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":2,"title":"현장 메모","summary":"","blocks":[{"id":"11111111-1111-4111-8111-111111111111","type":"widget"}],"metadata":{"type":"note"}}"#,
        )
        .await;
        assert_eq!(invalid_block.0, StatusCode::UNPROCESSABLE_ENTITY);
        let anonymous_save = call(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{entry_id}"),
            None,
            None,
            r#"{"expectedVersion":2,"title":"현장 메모","summary":"","blocks":[],"metadata":{"type":"note"}}"#,
        )
        .await;
        assert_eq!(anonymous_save.0, StatusCode::UNAUTHORIZED);
        let reloaded = call(
            &pool,
            "GET",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(reloaded.0, StatusCode::OK);
        assert!(reloaded.1.contains("저장한 문장"));
        assert!(reloaded.1.contains("짧은 요약"));

        let disabled_id: Uuid = sqlx::query_scalar(
            "INSERT INTO owner (password_hash, disabled) VALUES ($1, true) RETURNING id",
        )
        .bind(hash_password("disabled-owner").expect("disabled hash"))
        .fetch_one(&pool)
        .await
        .expect("disabled owner");
        let outsider = random_token();
        let outsider_csrf = random_token();
        sqlx::query(
            "INSERT INTO admin_session (owner_id, token_hash, csrf_hash, expires_at)
             VALUES ($1, $2, $3, now() + interval '1 hour')",
        )
        .bind(disabled_id)
        .bind(sha256_hex(&outsider))
        .bind(sha256_hex(&outsider_csrf))
        .execute(&pool)
        .await
        .expect("outsider session");
        let outsider_cookie = format!("dg_session={outsider}");
        let blocked = call(
            &pool,
            "GET",
            "/api/admin/entries",
            Some(&outsider_cookie),
            Some(&outsider_csrf),
            "",
        )
        .await;
        assert_eq!(blocked.0, StatusCode::FORBIDDEN);

        let revoked = call(
            &pool,
            "DELETE",
            "/api/admin/session",
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(revoked.0, StatusCode::OK);
        let after = call(
            &pool,
            "GET",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(after.0, StatusCode::UNAUTHORIZED);
        let _ = owner_id;
    }

    async fn call(
        pool: &sqlx::PgPool,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        csrf: Option<&str>,
        body: &str,
    ) -> (StatusCode, String, Option<String>) {
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
        let cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        let text = String::from_utf8(bytes.to_vec()).expect("utf-8");
        (status, text, cookie)
    }
}
