//! 기능: 문서 작업의 접수, 목록, 상세, 취소, 재시도.
//! 목적: 업무 상태와 그래프 단계를 나누고, 같은 요청이 작업을 늘리지 않게 한다.
//! 연관: `migrations/0011_ingestion.sql`, `contracts/ingestion/openapi.yaml`, `admin.rs`의 세션 검사.

use crate::admin::{audit, authorize, json_response, not_found, unavailable};
use crate::auth::sha256_hex;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PAGE_SIZE: i64 = 20;
const WORKFLOW_VERSION: i32 = 1;

/// 기능: 저장하는 업무 상태.
/// 목적: 그래프 노드 이름을 작업 상태에 넣지 않는다.
/// 연관: `contracts/ingestion/job-status.schema.json`, `WF03_STATE_MACHINE`.
pub const BUSINESS_STATUSES: &[&str] = &[
    "QUEUED",
    "RUNNING",
    "NEEDS_REVIEW",
    "RESUME_QUEUED",
    "RETRY_WAIT",
    "DRAFT_SAVED",
    "FAILED",
    "REJECTED",
    "CANCELLED",
];

/// 기능: 나중에 체크포인트에만 둘 그래프 단계.
/// 목적: 이 이름을 업무 status 컬럼의 값으로 쓰지 않는다.
/// 연관: `WF03_STATE_MACHINE`의 load → save_draft.
pub const GRAPH_PHASES: &[&str] = &["load", "normalize", "inspect", "retrieve", "review", "save_draft"];

/// 기능: 접수 라우터.
/// 목적: 관리자 세션으로만 자신의 작업을 만들고 읽게 한다.
/// 연관: `main.rs`의 `app`.
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/admin/imports", get(list_imports).post(create_import))
        .route("/api/admin/imports/{id}", get(read_import))
        .route("/api/admin/imports/{id}/cancel", post(cancel_import))
        .route("/api/admin/imports/{id}/retry", post(retry_import))
}

/// 기능: 한 업무 상태에서 다른 업무 상태로 갈 수 있는지 본다.
/// 목적: 워커가 없는 단계에서도 허용 전이를 고정한다.
/// 연관: `cancel_import`, `retry_import`.
pub fn transition_allowed(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("QUEUED", "RUNNING")
            | ("QUEUED", "CANCELLED")
            | ("RUNNING", "NEEDS_REVIEW")
            | ("RUNNING", "RETRY_WAIT")
            | ("RUNNING", "FAILED")
            | ("NEEDS_REVIEW", "RESUME_QUEUED")
            | ("NEEDS_REVIEW", "REJECTED")
            | ("RESUME_QUEUED", "RUNNING")
            | ("RETRY_WAIT", "RUNNING")
            | ("RETRY_WAIT", "FAILED")
            | ("FAILED", "QUEUED")
    )
}

fn business_status(value: &str) -> bool {
    BUSINESS_STATUSES.contains(&value)
}

fn allowed_actions(status: &str, cancel_requested: bool) -> Vec<&'static str> {
    if cancel_requested {
        return Vec::new();
    }
    match status {
        "QUEUED" | "RUNNING" | "NEEDS_REVIEW" | "RESUME_QUEUED" | "RETRY_WAIT" => vec!["cancel"],
        "FAILED" => vec!["retry"],
        _ => Vec::new(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportBody {
    request_id: String,
    source_id: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VersionBody {
    expected_version: i32,
}

#[derive(Deserialize)]
struct ListQuery {
    status: Option<String>,
    page: Option<i32>,
}

struct JobView {
    id: Uuid,
    source_id: Uuid,
    status: String,
    version: i32,
    workflow_version: i32,
    thread_id: Uuid,
    attempt: i32,
    cancel_requested: bool,
    draft_id: Option<Uuid>,
    error_code: Option<String>,
    review_version: Option<i32>,
}

/// 기능: 비공개 업로드를 작업으로 접수한다.
/// 목적: 같은 requestId는 같은 작업을 돌려주고, 다른 본문은 409로 남긴다.
/// 연관: `ingestion_requests`, `POST /api/admin/imports`.
async fn create_import(State(pool): State<PgPool>, headers: HeaderMap, body: String) -> axum::response::Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Ok(input) = serde_json::from_str::<ImportBody>(&body) else {
        return invalid_body();
    };
    if input.url.as_ref().is_some_and(|url| !url.is_empty()) {
        return coded(StatusCode::UNPROCESSABLE_ENTITY, "invalid", "url_not_accepted", json!({}));
    }
    let Some(request_id) = parse_id(&input.request_id) else {
        return invalid_body();
    };
    let Some(source_id) = input.source_id.as_deref().and_then(parse_id) else {
        return coded(StatusCode::UNPROCESSABLE_ENTITY, "invalid", "source_required", json!({}));
    };
    let hash = payload_hash(request_id, source_id);
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let existing: Option<(String, Uuid)> = match sqlx::query_as(
        "SELECT payload_hash, job_id FROM ingestion_requests WHERE owner_id = $1 AND request_id = $2",
    )
    .bind(principal.owner_id)
    .bind(request_id)
    .fetch_optional(tx.as_mut())
    .await
    {
        Ok(row) => row,
        Err(_) => return unavailable(),
    };
    if let Some((stored_hash, job_id)) = existing {
        let _ = tx.rollback().await;
        return if stored_hash == hash {
            stored_job(&pool, job_id, principal.owner_id).await
        } else {
            coded(StatusCode::CONFLICT, "conflict", "payload_conflict", json!({ "jobId": job_id.to_string() }))
        };
    }
    let media: Option<(String, String, i32, String, bool, String)> = match sqlx::query_as(
        "SELECT storage_key, checksum, bytes, media_type, public, status FROM media WHERE id = $1",
    )
    .bind(source_id)
    .fetch_optional(tx.as_mut())
    .await
    {
        Ok(row) => row,
        Err(_) => return unavailable(),
    };
    let Some((object_key, checksum, size, mime, public, media_status)) = media else {
        let _ = tx.rollback().await;
        return not_found();
    };
    if public {
        let _ = tx.rollback().await;
        return coded(StatusCode::UNPROCESSABLE_ENTITY, "invalid", "private_source", json!({}));
    }
    if media_status != "verified" && media_status != "referenced" {
        let _ = tx.rollback().await;
        return coded(StatusCode::UNPROCESSABLE_ENTITY, "invalid", "source_not_ready", json!({}));
    }
    let source_owner: Option<Uuid> = match sqlx::query_scalar("SELECT owner_id FROM ingestion_sources WHERE id = $1")
        .bind(source_id)
        .fetch_optional(tx.as_mut())
        .await
    {
        Ok(row) => row,
        Err(_) => return unavailable(),
    };
    if source_owner.is_some_and(|owner| owner != principal.owner_id) {
        let _ = tx.rollback().await;
        return not_found();
    }
    let same_material: Option<(Uuid, Uuid, String, i32)> = match sqlx::query_as(
        "SELECT j.id, j.source_id, j.status, j.version
         FROM ingestion_sources s
         JOIN ingestion_jobs j ON j.source_id = s.id
         WHERE s.owner_id = $1 AND s.sha256 = $2
         ORDER BY j.created_at ASC
         LIMIT 1",
    )
    .bind(principal.owner_id)
    .bind(&checksum)
    .fetch_optional(tx.as_mut())
    .await
    {
        Ok(row) => row,
        Err(_) => return unavailable(),
    };
    let response = if let Some((job_id, existing_source, status, version)) = same_material {
        let linked = link_request(tx.as_mut(), principal.owner_id, request_id, &hash, job_id).await;
        if linked.is_err() {
            let _ = tx.rollback().await;
            return replay_request(&pool, principal.owner_id, request_id, &hash).await;
        }
        if tx.commit().await.is_err() {
            return unavailable();
        }
        accepted(job_id, existing_source, &status, version, true)
    } else {
        let job_id = Uuid::new_v4();
        let source_inserted = sqlx::query(
            "INSERT INTO ingestion_sources
                (id, owner_id, kind, private_object_key, sha256, size, mime, metadata_json)
             VALUES ($1, $2, 'upload', $3, $4, $5, $6, '{}'::jsonb)",
        )
        .bind(source_id)
        .bind(principal.owner_id)
        .bind(&object_key)
        .bind(&checksum)
        .bind(size)
        .bind(&mime)
        .execute(tx.as_mut())
        .await;
        let job_inserted = sqlx::query(
            "INSERT INTO ingestion_jobs
                (id, owner_id, source_id, status, version, workflow_version, thread_id)
             VALUES ($1, $2, $3, 'QUEUED', 1, $4, $1)",
        )
        .bind(job_id)
        .bind(principal.owner_id)
        .bind(source_id)
        .bind(WORKFLOW_VERSION)
        .execute(tx.as_mut())
        .await;
        let linked = link_request(tx.as_mut(), principal.owner_id, request_id, &hash, job_id).await;
        let noted = record_event(tx.as_mut(), job_id, "accepted", "QUEUED").await;
        if source_inserted.is_err() || job_inserted.is_err() || linked.is_err() || noted.is_err() {
            let _ = tx.rollback().await;
            return replay_request(&pool, principal.owner_id, request_id, &hash).await;
        }
        if tx.commit().await.is_err() {
            return unavailable();
        }
        let _ = audit(&pool, Some(principal.owner_id), "import_accepted").await;
        accepted(job_id, source_id, "QUEUED", 1, false)
    };
    response
}

/// 기능: 자신의 작업을 상태와 페이지로 돌려준다.
/// 목적: 다른 소유자의 작업과 파일 키를 목록에 넣지 않는다.
/// 연관: `GET /api/admin/imports`.
async fn list_imports(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> axum::response::Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let status = query.status.as_deref().filter(|value| !value.is_empty());
    if let Some(status) = status {
        if !business_status(status) {
            return coded(StatusCode::UNPROCESSABLE_ENTITY, "invalid", "invalid_status", json!({}));
        }
    }
    let page = query.page.unwrap_or(1);
    if !(1..=100).contains(&page) {
        return invalid_body();
    }
    let offset = i64::from(page - 1) * PAGE_SIZE;
    let total: i64 = match sqlx::query_scalar(
        "SELECT count(*) FROM ingestion_jobs WHERE owner_id = $1 AND ($2::text IS NULL OR status = $2)",
    )
    .bind(principal.owner_id)
    .bind(status)
    .fetch_one(&pool)
    .await
    {
        Ok(total) => total,
        Err(_) => return unavailable(),
    };
    let rows: Result<Vec<(Uuid, Uuid, String, i32, String)>, sqlx::Error> = sqlx::query_as(
        "SELECT id, source_id, status, version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
         FROM ingestion_jobs
         WHERE owner_id = $1 AND ($2::text IS NULL OR status = $2)
         ORDER BY updated_at DESC, id DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(principal.owner_id)
    .bind(status)
    .bind(PAGE_SIZE)
    .bind(offset)
    .fetch_all(&pool)
    .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let imports = rows
        .into_iter()
        .map(|(id, source_id, status, version, updated_at)| {
            json!({
                "jobId": id.to_string(),
                "sourceId": source_id.to_string(),
                "status": status,
                "version": version,
                "updatedAt": updated_at,
            })
        })
        .collect::<Vec<_>>();
    json_response(
        StatusCode::OK,
        json!({ "imports": imports, "page": page, "pageSize": PAGE_SIZE, "total": total }).to_string(),
        None,
    )
}

/// 기능: 작업 하나의 업무 상태와 허용 action을 돌려준다.
/// 목적: 다른 소유자의 ID는 없는 작업과 같은 응답으로 거절한다.
/// 연관: `GET /api/admin/imports/{id}`, W11.
async fn read_import(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> axum::response::Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    match load_job(&pool, id, principal.owner_id).await {
        Ok(Some(job)) => detail_response(StatusCode::OK, &job),
        Ok(None) => not_found(),
        Err(()) => unavailable(),
    }
}

/// 기능: 기대 버전일 때만 취소를 기록한다.
/// 목적: 대기 작업은 바로 끝내고, 오래된 버전이나 종료 상태는 거절한다.
/// 연관: `transition_allowed`, `POST /api/admin/imports/{id}/cancel`.
async fn cancel_import(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: String,
) -> axum::response::Response {
    change_import(&pool, &headers, &id, &body, Change::Cancel).await
}

/// 기능: 실패한 작업만 다시 대기열에 넣는다.
/// 목적: 입력 파일을 바꾸지 않고, 다른 상태의 재시도를 거절한다.
/// 연관: `transition_allowed`, `POST /api/admin/imports/{id}/retry`.
async fn retry_import(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: String,
) -> axum::response::Response {
    change_import(&pool, &headers, &id, &body, Change::Retry).await
}

enum Change {
    Cancel,
    Retry,
}

async fn change_import(
    pool: &PgPool,
    headers: &HeaderMap,
    id: &str,
    body: &str,
    change: Change,
) -> axum::response::Response {
    let principal = match authorize(pool, headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<VersionBody>(body) else {
        return invalid_body();
    };
    if input.expected_version < 1 {
        return invalid_body();
    }
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let Some(job) = (match lock_job(tx.as_mut(), id, principal.owner_id).await {
        Ok(job) => job,
        Err(()) => return unavailable(),
    }) else {
        let _ = tx.rollback().await;
        return not_found();
    };
    if job.version != input.expected_version {
        let _ = tx.rollback().await;
        return stale_version(job.id, job.version);
    }
    let next = match change {
        Change::Cancel if job.cancel_requested => {
            let _ = tx.rollback().await;
            return match load_job(pool, id, principal.owner_id).await {
                Ok(Some(current)) => detail_response(StatusCode::ACCEPTED, &current),
                _ => unavailable(),
            };
        }
        Change::Cancel if transition_allowed(&job.status, "CANCELLED") => "CANCELLED",
        Change::Cancel if matches!(job.status.as_str(), "RUNNING" | "NEEDS_REVIEW" | "RESUME_QUEUED" | "RETRY_WAIT") => {
            job.status.as_str()
        }
        Change::Retry if transition_allowed(&job.status, "QUEUED") => "QUEUED",
        _ => {
            let _ = tx.rollback().await;
            return coded(
                StatusCode::CONFLICT,
                "conflict",
                "illegal_transition",
                json!({ "jobId": job.id.to_string(), "status": job.status }),
            );
        }
    };
    let saved = sqlx::query(
        "UPDATE ingestion_jobs
         SET status = $4,
             version = version + 1,
             cancel_requested_at = CASE WHEN $5 THEN now() ELSE NULL END,
             error_code = CASE WHEN $4 = 'QUEUED' THEN NULL ELSE error_code END,
             updated_at = now()
         WHERE id = $1 AND owner_id = $2 AND version = $3",
    )
    .bind(id)
    .bind(principal.owner_id)
    .bind(input.expected_version)
    .bind(next)
    .bind(matches!(change, Change::Cancel))
    .execute(tx.as_mut())
    .await;
    let event = match &change {
        Change::Cancel => "cancel_requested",
        Change::Retry => "retry_requested",
    };
    let noted = record_event(tx.as_mut(), id, event, next).await;
    if saved.is_err() || noted.is_err() || tx.commit().await.is_err() {
        return unavailable();
    }
    let _ = audit(pool, Some(principal.owner_id), event).await;
    match load_job(pool, id, principal.owner_id).await {
        Ok(Some(current)) => detail_response(StatusCode::ACCEPTED, &current),
        _ => unavailable(),
    }
}

fn payload_hash(request_id: Uuid, source_id: Uuid) -> String {
    sha256_hex(&format!(
        r#"{{"requestId":"{request_id}","sourceId":"{source_id}"}}"#
    ))
}

async fn link_request(
    tx: &mut sqlx::PgConnection,
    owner_id: Uuid,
    request_id: Uuid,
    hash: &str,
    job_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ingestion_requests (owner_id, request_id, payload_hash, job_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(owner_id)
    .bind(request_id)
    .bind(hash)
    .bind(job_id)
    .execute(tx)
    .await
    .map(|_| ())
}

async fn record_event(tx: &mut sqlx::PgConnection, job_id: Uuid, event_type: &str, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ingestion_events (job_id, event_type, safe_details) VALUES ($1, $2, jsonb_build_object('status', $3::text))",
    )
    .bind(job_id)
    .bind(event_type)
    .bind(status)
    .execute(tx)
    .await
    .map(|_| ())
}

async fn replay_request(pool: &PgPool, owner_id: Uuid, request_id: Uuid, hash: &str) -> axum::response::Response {
    let row: Option<(String, Uuid)> = match sqlx::query_as(
        "SELECT payload_hash, job_id FROM ingestion_requests WHERE owner_id = $1 AND request_id = $2",
    )
    .bind(owner_id)
    .bind(request_id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(_) => return unavailable(),
    };
    match row {
        Some((stored, job_id)) if stored == hash => stored_job(pool, job_id, owner_id).await,
        Some((_, job_id)) => coded(
            StatusCode::CONFLICT,
            "conflict",
            "payload_conflict",
            json!({ "jobId": job_id.to_string() }),
        ),
        None => unavailable(),
    }
}

async fn stored_job(pool: &PgPool, job_id: Uuid, owner_id: Uuid) -> axum::response::Response {
    match load_job(pool, job_id, owner_id).await {
        Ok(Some(job)) => accepted(job.id, job.source_id, &job.status, job.version, true),
        Ok(None) => not_found(),
        Err(()) => unavailable(),
    }
}

async fn lock_job(tx: &mut sqlx::PgConnection, id: Uuid, owner_id: Uuid) -> Result<Option<JobView>, ()> {
    let row: Option<(Uuid, Uuid, String, i32, i32, Uuid, i32, bool, Option<Uuid>, Option<String>, Option<i32>)> = sqlx::query_as(
        "SELECT j.id, j.source_id, j.status, j.version, j.workflow_version, j.thread_id, j.attempt,
                (j.cancel_requested_at IS NOT NULL), j.draft_id, j.error_code,
                (SELECT max(c.expected_review_version) FROM ingestion_commands c WHERE c.job_id = j.id)
         FROM ingestion_jobs j
         WHERE j.id = $1 AND j.owner_id = $2
         FOR UPDATE",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| ())?;
    Ok(row.map(job_view))
}

async fn load_job(pool: &PgPool, id: Uuid, owner_id: Uuid) -> Result<Option<JobView>, ()> {
    let row = sqlx::query_as(
        "SELECT j.id, j.source_id, j.status, j.version, j.workflow_version, j.thread_id, j.attempt,
                (j.cancel_requested_at IS NOT NULL), j.draft_id, j.error_code,
                (SELECT max(c.expected_review_version) FROM ingestion_commands c WHERE c.job_id = j.id)
         FROM ingestion_jobs j
         WHERE j.id = $1 AND j.owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ())?;
    Ok(row.map(job_view))
}

fn job_view(row: (Uuid, Uuid, String, i32, i32, Uuid, i32, bool, Option<Uuid>, Option<String>, Option<i32>)) -> JobView {
    JobView {
        id: row.0,
        source_id: row.1,
        status: row.2,
        version: row.3,
        workflow_version: row.4,
        thread_id: row.5,
        attempt: row.6,
        cancel_requested: row.7,
        draft_id: row.8,
        error_code: row.9,
        review_version: row.10,
    }
}

fn accepted(job_id: Uuid, source_id: Uuid, status: &str, version: i32, duplicate: bool) -> axum::response::Response {
    json_response(
        StatusCode::ACCEPTED,
        json!({
            "jobId": job_id.to_string(),
            "sourceId": source_id.to_string(),
            "status": status,
            "version": version,
            "duplicate": duplicate,
        })
        .to_string(),
        None,
    )
}

fn detail_response(status: StatusCode, job: &JobView) -> axum::response::Response {
    json_response(
        status,
        json!({
            "jobId": job.id.to_string(),
            "sourceId": job.source_id.to_string(),
            "status": job.status,
            "version": job.version,
            "reviewVersion": job.review_version,
            "workflowVersion": job.workflow_version,
            "threadId": job.thread_id.to_string(),
            "attempt": job.attempt,
            "allowedActions": allowed_actions(&job.status, job.cancel_requested),
            "draftId": job.draft_id.map(|id| id.to_string()),
            "errorCode": job.error_code,
        })
        .to_string(),
        None,
    )
}

fn stale_version(job_id: Uuid, version: i32) -> axum::response::Response {
    coded(
        StatusCode::CONFLICT,
        "conflict",
        "stale_version",
        json!({ "jobId": job_id.to_string(), "version": version }),
    )
}

fn coded(status: StatusCode, error: &str, code: &str, extra: Value) -> axum::response::Response {
    let mut body = json!({ "error": error, "code": code });
    if let (Some(object), Some(more)) = (body.as_object_mut(), extra.as_object()) {
        for (key, value) in more {
            object.insert(key.clone(), value.clone());
        }
    }
    json_response(status, body.to_string(), None)
}

fn invalid_body() -> axum::response::Response {
    json_response(StatusCode::UNPROCESSABLE_ENTITY, r#"{"error":"invalid"}"#.to_string(), None)
}

fn parse_id(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

#[cfg(test)]
mod tests {
    use super::{transition_allowed, BUSINESS_STATUSES, GRAPH_PHASES};
    use crate::auth::{hash_password, random_token, sha256_hex};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;
    use uuid::Uuid;

    #[test]
    fn business_transitions_leave_graph_phases_out() {
        assert!(transition_allowed("QUEUED", "RUNNING"));
        assert!(transition_allowed("QUEUED", "CANCELLED"));
        assert!(transition_allowed("FAILED", "QUEUED"));
        assert!(!transition_allowed("QUEUED", "FAILED"));
        assert!(!transition_allowed("CANCELLED", "QUEUED"));
        assert!(!transition_allowed("RUNNING", "CANCELLED"));
        for phase in GRAPH_PHASES {
            assert!(!BUSINESS_STATUSES.contains(phase));
        }
        let migration = include_str!("../migrations/0011_ingestion.sql");
        for status in BUSINESS_STATUSES {
            assert!(migration.contains(status));
        }
        for phase in GRAPH_PHASES {
            assert!(!migration.contains(&format!("'{phase}'")));
        }
        let schema = include_str!("../../../contracts/ingestion/job-status.schema.json");
        for status in BUSINESS_STATUSES {
            assert!(schema.contains(status));
        }
        let openapi = include_str!("../../../contracts/ingestion/openapi.yaml");
        assert!(openapi.contains("/api/admin/imports"));
        assert!(openapi.contains("reviewVersion"));
        let examples: Value = serde_json::from_str(include_str!("../../../contracts/ingestion/examples.json")).expect("examples");
        let ids = ["sourceId", "jobId", "commandId"];
        let mut seen = Vec::new();
        for id in ids {
            let value = examples[id].as_str().expect(id).to_string();
            assert!(!seen.contains(&value));
            seen.push(value);
        }
        assert_ne!(examples["reviewVersion"].as_i64(), examples["jobVersion"].as_i64());
    }

    #[tokio::test]
    #[ignore = "needs DB_URL, DB_USER, DB_PASSWORD, datagrizzly, and datagrizzly_m2"]
    async fn intake_keeps_cms_and_rejects_other_owners() {
        let dev = crate::db::connect_database("datagrizzly").await.expect("dev database");
        let before: i64 = sqlx::query_scalar("SELECT count(*) FROM entry").fetch_one(&dev).await.expect("entries");
        crate::db::migrate(&dev).await.expect("dev migrate");
        crate::db::migrate(&dev).await.expect("dev migrate again");
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM entry").fetch_one(&dev).await.expect("entries again");
        assert_eq!(before, after);
        let version: i32 = sqlx::query_scalar("SELECT schema_version FROM schema_meta LIMIT 1")
            .fetch_one(&dev)
            .await
            .expect("schema");
        assert_eq!(version, 1);
        let jobs: Option<String> = sqlx::query_scalar("SELECT to_regclass('public.ingestion_jobs')::text")
            .fetch_one(&dev)
            .await
            .expect("jobs table");
        assert_eq!(jobs.as_deref(), Some("ingestion_jobs"));
        let wrong = call(&dev, "POST", "/api/admin/session", None, None, r#"{"password":"not-a-real-owner-password"}"#).await;
        assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);

        let pool = crate::db::connect_database("datagrizzly_m2").await.expect("isolated database");
        crate::db::migrate(&pool).await.expect("isolated migrate");
        for table in [
            "ingestion_requests",
            "ingestion_draft_receipts",
            "ingestion_events",
            "ingestion_commands",
            "ingestion_artifacts",
            "ingestion_jobs",
            "ingestion_sources",
            "public_search_section",
            "public_search_document",
            "public_search_load",
            "site_release",
            "publication_upload",
            "publication_request",
            "publication_schedule",
            "publication_job",
            "entry_redirect",
            "withdrawal",
            "entry_denylist",
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
            sqlx::query(&format!("DELETE FROM {table}")).execute(&pool).await.expect("clear isolated rows");
        }

        let password = random_token();
        let owner_id: Uuid = sqlx::query_scalar("INSERT INTO owner (password_hash) VALUES ($1) RETURNING id")
            .bind(hash_password(&password).expect("hash"))
            .fetch_one(&pool)
            .await
            .expect("owner");
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
        let session = logged_in.2.expect("cookie").split(';').next().expect("pair").to_string();
        let csrf = logged_in.1.split("csrfToken\":\"").nth(1).and_then(|value| value.split('"').next()).expect("csrf").to_string();

        let storage_key = "private-object-key-m8";
        let checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let media_id: Uuid = sqlx::query_scalar(
            "INSERT INTO media (storage_key, original_name, media_type, bytes, checksum, status, public)
             VALUES ($1, 'note.md', 'text/markdown', 12, $2, 'verified', false)
             RETURNING id",
        )
        .bind(storage_key)
        .bind(checksum)
        .fetch_one(&pool)
        .await
        .expect("media");
        let public_id: Uuid = sqlx::query_scalar(
            "INSERT INTO media (storage_key, original_name, media_type, bytes, checksum, status, public)
             VALUES ('public-object-key-m8', 'cover.png', 'image/png', 12, 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', 'verified', true)
             RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .expect("public media");

        let request_id = Uuid::new_v4();
        let created = call(
            &pool,
            "POST",
            "/api/admin/imports",
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"requestId":"{request_id}","sourceId":"{media_id}"}}"#),
        )
        .await;
        assert_eq!(created.0, StatusCode::ACCEPTED);
        let created_body: Value = serde_json::from_str(&created.1).expect("created json");
        assert_eq!(created_body["duplicate"], false);
        assert_eq!(created_body["status"], "QUEUED");
        let job_id = created_body["jobId"].as_str().expect("job").to_string();
        let repeated = call(
            &pool,
            "POST",
            "/api/admin/imports",
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"requestId":"{request_id}","sourceId":"{media_id}"}}"#),
        )
        .await;
        assert_eq!(repeated.0, StatusCode::ACCEPTED);
        assert!(repeated.1.contains(&job_id));
        assert!(repeated.1.contains("\"duplicate\":true"));
        let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM ingestion_jobs").fetch_one(&pool).await.expect("job count");
        assert_eq!(jobs, 1);
        let other_source = Uuid::new_v4();
        let conflict = call(
            &pool,
            "POST",
            "/api/admin/imports",
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"requestId":"{request_id}","sourceId":"{other_source}"}}"#),
        )
        .await;
        assert_eq!(conflict.0, StatusCode::CONFLICT);
        assert!(conflict.1.contains("payload_conflict"));
        assert!(!conflict.1.contains(storage_key));
        let still: i64 = sqlx::query_scalar("SELECT count(*) FROM ingestion_jobs").fetch_one(&pool).await.expect("job count");
        assert_eq!(still, 1);
        let copy_id: Uuid = sqlx::query_scalar(
            "INSERT INTO media (storage_key, original_name, media_type, bytes, checksum, status, public)
             VALUES ('private-object-copy-m8', 'note-copy.md', 'text/markdown', 12, $1, 'verified', false)
             RETURNING id",
        )
        .bind(checksum)
        .fetch_one(&pool)
        .await
        .expect("same bytes");
        let guided = call(
            &pool,
            "POST",
            "/api/admin/imports",
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"requestId":"{}","sourceId":"{copy_id}"}}"#, Uuid::new_v4()),
        )
        .await;
        assert_eq!(guided.0, StatusCode::ACCEPTED);
        assert!(guided.1.contains(&job_id));
        assert!(guided.1.contains("\"duplicate\":true"));
        let still_one: i64 = sqlx::query_scalar("SELECT count(*) FROM ingestion_jobs").fetch_one(&pool).await.expect("job count");
        assert_eq!(still_one, 1);

        let public_source = call(
            &pool,
            "POST",
            "/api/admin/imports",
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"requestId":"{}","sourceId":"{public_id}"}}"#, Uuid::new_v4()),
        )
        .await;
        assert_eq!(public_source.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(public_source.1.contains("private_source"));
        let url_source = call(
            &pool,
            "POST",
            "/api/admin/imports",
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"requestId":"{}","url":"https://example.com/note"}}"#, Uuid::new_v4()),
        )
        .await;
        assert_eq!(url_source.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(url_source.1.contains("url_not_accepted"));
        let path_source = call(
            &pool,
            "POST",
            "/api/admin/imports",
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"requestId":"{}","sourceId":"{media_id}","objectKey":"C:\\secret"}}"#, Uuid::new_v4()),
        )
        .await;
        assert_eq!(path_source.0, StatusCode::UNPROCESSABLE_ENTITY);

        let listed = call(&pool, "GET", "/api/admin/imports?status=QUEUED&page=1", Some(&session), Some(&csrf), "").await;
        assert_eq!(listed.0, StatusCode::OK);
        assert!(listed.1.contains(&job_id));
        assert!(!listed.1.contains(storage_key));
        assert!(!listed.1.contains(checksum));
        let detail = call(&pool, "GET", &format!("/api/admin/imports/{job_id}"), Some(&session), Some(&csrf), "").await;
        assert_eq!(detail.0, StatusCode::OK);
        let detail_body: Value = serde_json::from_str(&detail.1).expect("detail");
        assert_eq!(detail_body["reviewVersion"], Value::Null);
        assert_eq!(detail_body["version"], 1);
        assert_eq!(detail_body["allowedActions"][0], "cancel");
        assert!(!detail.1.contains(storage_key));

        let outsider_id: Uuid = sqlx::query_scalar("INSERT INTO owner (password_hash) VALUES ($1) RETURNING id")
            .bind(hash_password("other-owner").expect("other hash"))
            .fetch_one(&pool)
            .await
            .expect("other owner");
        let outsider = random_token();
        let outsider_csrf = random_token();
        sqlx::query(
            "INSERT INTO admin_session (owner_id, token_hash, csrf_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
        )
        .bind(outsider_id)
        .bind(sha256_hex(&outsider))
        .bind(sha256_hex(&outsider_csrf))
        .execute(&pool)
        .await
        .expect("other session");
        let hidden = call(
            &pool,
            "GET",
            &format!("/api/admin/imports/{job_id}"),
            Some(&format!("dg_session={outsider}")),
            Some(&outsider_csrf),
            "",
        )
        .await;
        assert_eq!(hidden.0, StatusCode::NOT_FOUND);
        assert_eq!(hidden.1, r#"{"error":"not_found"}"#);
        assert!(!hidden.1.contains(storage_key));
        assert!(!hidden.1.contains(&job_id));

        let early_retry = call(
            &pool,
            "POST",
            &format!("/api/admin/imports/{job_id}/retry"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1}"#,
        )
        .await;
        assert_eq!(early_retry.0, StatusCode::CONFLICT);
        assert!(early_retry.1.contains("illegal_transition"));
        let stale = call(
            &pool,
            "POST",
            &format!("/api/admin/imports/{job_id}/cancel"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":9}"#,
        )
        .await;
        assert_eq!(stale.0, StatusCode::CONFLICT);
        assert!(stale.1.contains("stale_version"));
        let version: i32 = sqlx::query_scalar("SELECT version FROM ingestion_jobs WHERE id = $1")
            .bind(Uuid::parse_str(&job_id).expect("job uuid"))
            .fetch_one(&pool)
            .await
            .expect("version");
        assert_eq!(version, 1);
        let cancelled = call(
            &pool,
            "POST",
            &format!("/api/admin/imports/{job_id}/cancel"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1}"#,
        )
        .await;
        assert_eq!(cancelled.0, StatusCode::ACCEPTED);
        assert!(cancelled.1.contains("CANCELLED"));
        let retry_done = call(
            &pool,
            "POST",
            &format!("/api/admin/imports/{job_id}/retry"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":2}"#,
        )
        .await;
        assert_eq!(retry_done.0, StatusCode::CONFLICT);

        let search = call(&pool, "GET", &format!("/api/search?q={checksum}"), None, None, "").await;
        assert!(!search.1.contains(checksum));
        assert!(!search.1.contains(storage_key));
        let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM public_search_document").fetch_one(&pool).await.expect("search rows");
        assert_eq!(visible, 0);
        let event: String = sqlx::query_scalar("SELECT safe_details::text FROM ingestion_events WHERE job_id = $1 LIMIT 1")
            .bind(Uuid::parse_str(&job_id).expect("job uuid"))
            .fetch_one(&pool)
            .await
            .expect("event");
        assert!(!event.contains(checksum));
        assert!(!event.contains(storage_key));

        let drafted = call(
            &pool,
            "POST",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            r#"{"slug":"m8-intake-note","title":"접수 확인","kind":"note"}"#,
        )
        .await;
        assert_eq!(drafted.0, StatusCode::CREATED);
        let entry_id = drafted.1.split("\"id\":\"").nth(1).and_then(|value| value.split('"').next()).expect("entry");
        let saved = call(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1,"title":"접수 확인","summary":"","blocks":[{"id":"11111111-1111-4111-8111-111111111111","type":"paragraph","text":"수동 초안"}],"metadata":{"type":"note"}}"#,
        )
        .await;
        assert_eq!(saved.0, StatusCode::OK);
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
        let cookie = response.headers().get("set-cookie").and_then(|value| value.to_str().ok()).map(str::to_string);
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        (status, String::from_utf8(bytes.to_vec()).expect("utf-8"), cookie)
    }
}
