//! 개념: 발행 작업의 접수, 임대, 세대 검사.
//! 목적: 같은 요청은 한 작업으로 모으고, 작업자 재시작이 공개본을 다시 만들지 않게 한다.
//! 사용처: `/api/admin/entries/{id}/publications`, `/api/admin/publication-jobs`.
//! 이 모듈은 공개 파일과 published_revision_id를 쓰지 않는다. 성공 상태는 도메인 규칙으로만 검사한다.

use crate::admin::{audit, authorize, conflict, invalid, json_response, not_found, unavailable};
use crate::auth::sha256_hex;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// 개념: 자동 재시도 상한.
/// 목적: 죽은 작업자를 무한히 다시 돌리지 않는다.
/// 사용처: 임대 회수와 작업 획득.
pub const AUTO_ATTEMPT_LIMIT: i32 = 3;

/// 개념: 운영자가 직접 누르는 재시도 상한.
/// 목적: 자동 재시도 횟수와 따로 세어 폭주를 막는다.
/// 사용처: `retry_job`.
pub const OPERATOR_RETRY_LIMIT: i32 = 5;

/// 개념: 한 번의 작업 임대 시간(초).
/// 목적: 이 시간이 지나면 다른 회차가 같은 작업을 이어 받는다.
/// 사용처: `claim`.
pub const LEASE_SECONDS: f64 = 30.0;

const PAGE_SIZE: i64 = 8;

/// 개념: 새 발행 작업 행.
/// 목적: 공개 포인터 열을 이 문장에 넣지 않는다.
/// 사용처: 접수 트랜잭션과 단위 시험.
pub const JOB_INSERT: &str = "INSERT INTO publication_job (id, entry_id, revision_id, state) VALUES ($1, $2, $3, 'queued')";

/// 개념: 만료된 임대를 대기열로 되돌리거나 실패로 닫는다.
/// 목적: 세대 번호를 올려 이전 작업자의 완료 요청을 무효로 만든다.
/// 사용처: `poll_jobs`.
pub const RECLAIM_SQL: &str = "UPDATE publication_job
SET state = CASE WHEN attempt >= $1 THEN 'failed' ELSE 'queued' END,
    failure_code = CASE WHEN attempt >= $1 THEN 'retry_exhausted' ELSE NULL END,
    fence = fence + 1,
    lease_until = NULL,
    updated_at = now()
WHERE state IN ('building', 'deploying')
  AND lease_until IS NOT NULL
  AND lease_until <= now()";

/// 개념: 대기 작업 하나를 빌드 상태로 집는다.
/// 목적: SKIP LOCKED로 두 폴러가 같은 행을 동시에 활성화하지 않는다.
/// 사용처: `poll_jobs`.
pub const CLAIM_SQL: &str = "UPDATE publication_job AS job
SET state = 'building',
    fence = job.fence + 1,
    attempt = job.attempt + 1,
    lease_until = now() + make_interval(secs => $1::double precision),
    updated_at = now()
WHERE job.id = (
    SELECT id FROM publication_job
    WHERE state = 'queued'
      AND (retry_at IS NULL OR retry_at <= now())
      AND attempt < $2
    ORDER BY created_at
    FOR UPDATE SKIP LOCKED
    LIMIT 1
)
RETURNING job.id, job.fence";

/// 개념: 작업과 세대의 업로드 기록.
/// 목적: 같은 세대의 재전송은 새 행을 만들지 않는다.
/// 사용처: `note_upload`.
pub const UPLOAD_SQL: &str = "INSERT INTO publication_upload (job_id, generation) VALUES ($1, $2) ON CONFLICT DO NOTHING";

/// 개념: 발행 작업이 머물 수 있는 상태.
/// 목적: 화면과 저장 값이 같은 이름을 쓰게 한다.
/// 사용처: 전이 검사.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Building,
    Deploying,
    Succeeded,
    Failed,
    Canceled,
}

impl JobState {
    /// 개념: 상태에 대응하는 저장 문자열.
    /// 목적: DB와 JSON이 서로 다른 철자를 쓰지 않게 한다.
    /// 사용처: 조회와 갱신.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Building => "building",
            Self::Deploying => "deploying",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "building" => Some(Self::Building),
            "deploying" => Some(Self::Deploying),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}

/// 개념: 허용된 상태 이동.
/// 목적: 대기 중인 작업이 확인 완료로 건너뛰거나, 끝난 작업이 다시 빌드되지 않게 한다.
/// 사용처: 취소, 재시도, 업로드 기록, 단위 시험.
pub fn transition(from: JobState, to: JobState) -> Result<(), ()> {
    let allowed = matches!(
        (from, to),
        (JobState::Queued, JobState::Building)
            | (JobState::Queued, JobState::Canceled)
            | (JobState::Building, JobState::Deploying)
            | (JobState::Building, JobState::Failed)
            | (JobState::Building, JobState::Queued)
            | (JobState::Deploying, JobState::Succeeded)
            | (JobState::Deploying, JobState::Failed)
            | (JobState::Deploying, JobState::Queued)
            | (JobState::Failed, JobState::Queued)
    );
    if allowed {
        Ok(())
    } else {
        Err(())
    }
}

/// 개념: 접수 본문의 지문.
/// 목적: 같은 키라도 글이나 수정본이 다르면 다른 요청으로 본다.
/// 사용처: 발행 접수.
pub fn payload_hash(entry_id: Uuid, revision_id: Uuid) -> String {
    sha256_hex(&format!("{entry_id}|{revision_id}|1"))
}

/// 개념: 클라이언트가 보낸 접수 키.
/// 목적: 공백이나 너무 짧은 키가 서로 다른 요청을 한 작업으로 합치지 않게 한다.
/// 사용처: 발행 접수.
pub fn valid_idempotency_key(value: &str) -> bool {
    let length = value.chars().count();
    (8..=80).contains(&length) && value.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-'))
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/admin/entries/{id}/publications", post(accept_publication))
        .route("/api/admin/publication-jobs", get(list_jobs))
        .route("/api/admin/publication-jobs/poll", post(poll_jobs))
        .route("/api/admin/publication-jobs/{id}", get(read_job))
        .route("/api/admin/publication-jobs/{id}/retry", post(retry_job))
        .route("/api/admin/publication-jobs/{id}/cancel", post(cancel_job))
        .route("/api/admin/publication-jobs/{id}/advance", post(advance_job))
}

#[derive(Deserialize)]
struct AcceptBody {
    #[serde(rename = "schemaVersion")]
    schema_version: i32,
    #[serde(rename = "revisionId")]
    revision_id: String,
    #[serde(rename = "idempotencyKey")]
    idempotency_key: String,
}

#[derive(Deserialize)]
struct PageQuery {
    page: Option<i32>,
}

#[derive(Deserialize)]
struct FenceBody {
    fence: i64,
}

/// 개념: 발행 작업 접수.
/// 목적: 수정본 확인, 키 저장, 작업 생성을 한 트랜잭션으로 끝내고 202를 공개 완료로 쓰지 않는다.
/// 사용처: `POST /api/admin/entries/{id}/publications`.
async fn accept_publication(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(entry_id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<AcceptBody>(&body) else {
        return invalid();
    };
    if input.schema_version != 1 {
        return invalid_field("schemaVersion");
    }
    if !valid_idempotency_key(&input.idempotency_key) {
        return invalid_field("idempotencyKey");
    }
    let Some(revision_id) = parse_id(&input.revision_id) else {
        return invalid_field("revisionId");
    };
    let hash = payload_hash(entry_id, revision_id);
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let entry_exists: Option<i32> = match sqlx::query_scalar("SELECT 1 FROM entry WHERE id = $1")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    if entry_exists.is_none() {
        let _ = tx.rollback().await;
        return not_found();
    }
    // 그 글의 수정본만 접수한다. 다른 글의 수정본 ID는 거절하고 작업을 만들지 않는다.
    let revision: Option<Uuid> = match sqlx::query_scalar("SELECT id FROM revision WHERE id = $1 AND entry_id = $2")
        .bind(revision_id)
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    if revision.is_none() {
        let _ = tx.rollback().await;
        return invalid_field("revisionId");
    }
    let existing: Option<(String, Uuid)> = match sqlx::query_as("SELECT payload_hash, job_id FROM publication_request WHERE idempotency_key = $1")
        .bind(&input.idempotency_key)
        .fetch_optional(tx.as_mut())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    if let Some((stored_hash, job_id)) = existing {
        let _ = tx.rollback().await;
        // 같은 키와 같은 본문은 같은 작업이다. 본문이 다르면 새 작업을 만들지 않는다.
        return if stored_hash == hash {
            accepted(job_id)
        } else {
            conflict()
        };
    }
    let job_id = Uuid::new_v4();
    let inserted = sqlx::query(JOB_INSERT)
        .bind(job_id)
        .bind(entry_id)
        .bind(revision_id)
        .execute(tx.as_mut())
        .await;
    let linked = sqlx::query("INSERT INTO publication_request (idempotency_key, payload_hash, job_id) VALUES ($1, $2, $3)")
        .bind(&input.idempotency_key)
        .bind(&hash)
        .bind(job_id)
        .execute(tx.as_mut())
        .await;
    if inserted.is_err() || linked.is_err() {
        let _ = tx.rollback().await;
        // 동시에 같은 키가 들어왔으면 먼저 저장된 작업을 다시 읽는다.
        return replay_or_conflict(&pool, &input.idempotency_key, &hash).await;
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "publication_accepted").await;
    accepted(job_id)
}

async fn replay_or_conflict(pool: &PgPool, key: &str, hash: &str) -> Response {
    let existing: Option<(String, Uuid)> = match sqlx::query_as("SELECT payload_hash, job_id FROM publication_request WHERE idempotency_key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    match existing {
        Some((stored_hash, job_id)) if stored_hash == hash => accepted(job_id),
        Some(_) => conflict(),
        None => unavailable(),
    }
}

/// 개념: 발행 작업 목록.
/// 목적: 상태, 대상 수정본, 실패 코드, 재시도 가능 여부만 보여 주고 본문과 비밀은 빼 둔다.
/// 사용처: `/admin/publications`.
async fn list_jobs(State(pool): State<PgPool>, headers: HeaderMap, Query(query): Query<PageQuery>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let page = query.page.unwrap_or(1).clamp(1, 100);
    let offset = i64::from(page - 1) * PAGE_SIZE;
    let total: i64 = match sqlx::query_scalar("SELECT count(*) FROM publication_job").fetch_one(&pool).await {
        Ok(total) => total,
        Err(_) => return unavailable(),
    };
    let rows = sqlx::query_as::<_, (Uuid, i64, Uuid, i32, String, Option<String>, Option<Uuid>, i32, bool)>(
        "SELECT publication_job.id, publication_job.job_number, publication_job.revision_id, revision.version,
                publication_job.state, publication_job.failure_code, publication_job.release_id,
                publication_job.operator_retries, publication_job.state = 'failed' AND publication_job.operator_retries < $3
         FROM publication_job
         JOIN revision ON revision.id = publication_job.revision_id
         ORDER BY publication_job.job_number DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(PAGE_SIZE)
    .bind(offset)
    .bind(OPERATOR_RETRY_LIMIT)
    .fetch_all(&pool)
    .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let jobs = rows
        .into_iter()
        .map(|(id, number, revision_id, version, state, failure, release_id, operator_retries, retryable)| {
            json!({
                "id": id.to_string(),
                "jobNumber": number,
                "revisionId": revision_id.to_string(),
                "revisionVersion": version,
                "state": state,
                "failureCode": failure,
                "releaseId": release_id.map(|value| value.to_string()),
                "operatorRetries": operator_retries,
                "retryable": retryable,
            })
        })
        .collect::<Vec<_>>();
    json_response(
        StatusCode::OK,
        json!({"jobs": jobs, "page": page, "pageSize": PAGE_SIZE, "total": total}).to_string(),
        None,
    )
}

/// 개념: 작업 한 건의 계약 응답.
/// 목적: 상태와 공개 세대, 실패만 알리고 접수 자체를 성공으로 적지 않는다.
/// 사용처: `GET /api/admin/publication-jobs/{id}`.
async fn read_job(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (String, Option<Uuid>, Option<String>)>(
        "SELECT state, release_id, failure_code FROM publication_job WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;
    let Ok(row) = row else {
        return unavailable();
    };
    let Some((state, release_id, failure)) = row else {
        return not_found();
    };
    json_response(
        StatusCode::OK,
        json!({
            "schemaVersion": 1,
            "state": state,
            "releaseId": release_id.map(|value| value.to_string()),
            "failureCode": failure,
            "error": failure.map(|code| json!({"code": code})),
        })
        .to_string(),
        None,
    )
}

/// 개념: 폴러 한 번.
/// 목적: 만료된 임대를 회수한 뒤 대기 작업 하나만 집고, 그 세대의 업로드 기록을 한 번만 남긴다.
/// 사용처: `POST /api/admin/publication-jobs/poll`. 공개 확인 성공으로는 옮기지 않는다.
async fn poll_jobs(State(pool): State<PgPool>, headers: HeaderMap) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let reclaimed = match sqlx::query(RECLAIM_SQL).bind(AUTO_ATTEMPT_LIMIT).execute(tx.as_mut()).await {
        Ok(result) => result.rows_affected(),
        Err(_) => return unavailable(),
    };
    let claimed = sqlx::query_as::<_, (Uuid, i64)>(CLAIM_SQL)
        .bind(LEASE_SECONDS)
        .bind(AUTO_ATTEMPT_LIMIT)
        .fetch_optional(tx.as_mut())
        .await;
    let Ok(claimed) = claimed else {
        return unavailable();
    };
    let claimed_job = if let Some((job_id, fence)) = claimed {
        // 방금 집은 세대의 업로드 기록만 남긴다. 공개 파일은 만들지 않는다.
        if note_upload(tx.as_mut(), job_id, fence).await.is_err() {
            let _ = tx.rollback().await;
            return unavailable();
        }
        Some((job_id, fence))
    } else {
        None
    };
    if tx.commit().await.is_err() {
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "publication_polled").await;
    json_response(
        StatusCode::OK,
        json!({
            "schemaVersion": 1,
            "claimedJobId": claimed_job.map(|(id, _)| id.to_string()),
            "fence": claimed_job.map(|(_, fence)| fence),
            "reclaimed": reclaimed,
            "succeeded": false,
        })
        .to_string(),
        None,
    )
}

/// 개념: 현재 세대로 업로드를 다시 기록한다.
/// 목적: 세대가 다르면 409로 거절하고 작업 상태는 그대로 둔다.
/// 사용처: 작업을 집었던 폴러의 재전송.
async fn advance_job(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<FenceBody>(&body) else {
        return invalid();
    };
    if input.fence < 1 {
        return invalid_field("fence");
    }
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    match note_upload(tx.as_mut(), id, input.fence).await {
        Ok(()) => {
            if tx.commit().await.is_err() {
                return unavailable();
            }
            json_response(StatusCode::OK, json!({"schemaVersion": 1, "succeeded": false}).to_string(), None)
        }
        Err(AdvanceError::Conflict) => {
            let _ = tx.rollback().await;
            conflict()
        }
        Err(AdvanceError::Missing) => {
            let _ = tx.rollback().await;
            not_found()
        }
        Err(AdvanceError::Unavailable) => unavailable(),
    }
}

/// 개념: 실패한 작업을 운영자가 다시 대기열에 넣는다.
/// 목적: 자동 시도 횟수와 별도인 운영자 횟수가 상한에 닿으면 상태를 바꾸지 않는다.
/// 사용처: 발행 작업 화면의 재시도.
async fn retry_job(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let row = sqlx::query_as::<_, (String, i32)>("SELECT state, operator_retries FROM publication_job WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(tx.as_mut())
        .await;
    let Ok(row) = row else {
        return unavailable();
    };
    let Some((state, operator_retries)) = row else {
        return not_found();
    };
    let Some(current) = JobState::parse(&state) else {
        return unavailable();
    };
    if transition(current, JobState::Queued).is_err() || operator_retries >= OPERATOR_RETRY_LIMIT {
        let _ = tx.rollback().await;
        return conflict();
    }
    let updated = sqlx::query(
        "UPDATE publication_job
         SET state = 'queued', attempt = 0, operator_retries = operator_retries + 1,
             failure_code = NULL, lease_until = NULL, fence = fence + 1, updated_at = now()
         WHERE id = $1 AND state = 'failed'",
    )
    .bind(id)
    .execute(tx.as_mut())
    .await;
    if updated.map(|result| result.rows_affected()).unwrap_or(0) != 1 {
        let _ = tx.rollback().await;
        return conflict();
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "publication_retried").await;
    json_response(StatusCode::OK, json!({"schemaVersion": 1, "state": "queued", "succeeded": false}).to_string(), None)
}

/// 개념: 아직 시작하지 않은 접수를 취소한다.
/// 목적: 대기 상태에서만 취소하고, 진행 중이거나 끝난 작업은 그대로 둔다.
/// 사용처: 발행 작업 화면.
async fn cancel_job(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let state: Option<String> = match sqlx::query_scalar("SELECT state FROM publication_job WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(tx.as_mut())
        .await
    {
        Ok(state) => state,
        Err(_) => return unavailable(),
    };
    let Some(state) = state else {
        return not_found();
    };
    let Some(current) = JobState::parse(&state) else {
        return unavailable();
    };
    if transition(current, JobState::Canceled).is_err() {
        let _ = tx.rollback().await;
        return conflict();
    }
    let updated = sqlx::query(
        "UPDATE publication_job
         SET state = 'canceled', lease_until = NULL, fence = fence + 1, updated_at = now()
         WHERE id = $1 AND state = 'queued'",
    )
    .bind(id)
    .execute(tx.as_mut())
    .await;
    if updated.map(|result| result.rows_affected()).unwrap_or(0) != 1 {
        let _ = tx.rollback().await;
        return conflict();
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "publication_canceled").await;
    json_response(StatusCode::OK, json!({"schemaVersion": 1, "state": "canceled", "succeeded": false}).to_string(), None)
}

enum AdvanceError {
    Conflict,
    Missing,
    Unavailable,
}

/// 개념: 현재 세대의 업로드를 한 행으로 남긴다.
/// 목적: 빌드 중인 작업만 배포 대기로 옮기고, 같은 세대의 재전송은 추가 행 없이 같은 결과를 돌려준다.
/// 사용처: 폴링과 `advance_job`. 확인 완료로는 옮기지 않는다.
async fn note_upload(tx: &mut sqlx::PgConnection, job_id: Uuid, fence: i64) -> Result<(), AdvanceError> {
    let row = sqlx::query_as::<_, (String, i64)>("SELECT state, fence FROM publication_job WHERE id = $1 FOR UPDATE")
        .bind(job_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AdvanceError::Unavailable)?;
    let Some((state, current_fence)) = row else {
        return Err(AdvanceError::Missing);
    };
    if current_fence != fence {
        return Err(AdvanceError::Conflict);
    }
    let Some(current) = JobState::parse(&state) else {
        return Err(AdvanceError::Unavailable);
    };
    sqlx::query(UPLOAD_SQL)
        .bind(job_id)
        .bind(fence)
        .execute(&mut *tx)
        .await
        .map_err(|_| AdvanceError::Unavailable)?;
    if current == JobState::Deploying {
        return Ok(());
    }
    if transition(current, JobState::Deploying).is_err() {
        return Err(AdvanceError::Conflict);
    }
    let updated = sqlx::query(
        "UPDATE publication_job SET state = 'deploying', updated_at = now() WHERE id = $1 AND fence = $2 AND state = 'building'",
    )
    .bind(job_id)
    .bind(fence)
    .execute(&mut *tx)
    .await
    .map_err(|_| AdvanceError::Unavailable)?;
    if updated.rows_affected() == 1 {
        Ok(())
    } else {
        Err(AdvanceError::Conflict)
    }
}

fn accepted(job_id: Uuid) -> Response {
    json_response(
        StatusCode::ACCEPTED,
        json!({"schemaVersion": 1, "jobId": job_id.to_string()}).to_string(),
        None,
    )
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
    use super::{
        payload_hash, transition, valid_idempotency_key, JobState, CLAIM_SQL, JOB_INSERT, RECLAIM_SQL, UPLOAD_SQL,
        AUTO_ATTEMPT_LIMIT, OPERATOR_RETRY_LIMIT,
    };
    use axum::http::StatusCode;
    use uuid::Uuid;

    /// 개념: 상태 이동 규칙.
    /// 목적: 완료와 취소에서 뒤로 가지 않고, 대기가 확인 완료로 건너뛰지 않는지 확인한다.
    /// 사용처: `cargo test`.
    #[test]
    fn only_listed_transitions_are_allowed() {
        assert_eq!(JobState::Queued.as_str(), "queued");
        assert_eq!(JobState::Deploying.as_str(), "deploying");
        assert!(transition(JobState::Queued, JobState::Building).is_ok());
        assert!(transition(JobState::Queued, JobState::Canceled).is_ok());
        assert!(transition(JobState::Building, JobState::Deploying).is_ok());
        assert!(transition(JobState::Deploying, JobState::Succeeded).is_ok());
        assert!(transition(JobState::Deploying, JobState::Queued).is_ok());
        assert!(transition(JobState::Failed, JobState::Queued).is_ok());
        assert!(transition(JobState::Queued, JobState::Succeeded).is_err());
        assert!(transition(JobState::Succeeded, JobState::Queued).is_err());
        assert!(transition(JobState::Canceled, JobState::Queued).is_err());
        assert!(transition(JobState::Failed, JobState::Succeeded).is_err());
        assert_eq!(AUTO_ATTEMPT_LIMIT, 3);
        assert_eq!(OPERATOR_RETRY_LIMIT, 5);
    }

    /// 개념: 접수 SQL과 업로드 키.
    /// 목적: 작업 생성이 공개 포인터를 쓰지 않고, 업로드는 작업과 세대가 같을 때 한 번만 들어간다.
    /// 사용처: `cargo test`.
    #[test]
    fn accept_sql_leaves_the_published_pointer_and_upload_is_idempotent() {
        assert!(!JOB_INSERT.contains("published_revision_id"));
        assert!(CLAIM_SQL.contains("SKIP LOCKED"));
        assert!(CLAIM_SQL.contains("fence = job.fence + 1"));
        assert!(RECLAIM_SQL.contains("fence = fence + 1"));
        assert!(UPLOAD_SQL.contains("ON CONFLICT DO NOTHING"));
        let entry_id = Uuid::from_u128(1);
        let revision_id = Uuid::from_u128(2);
        assert_eq!(payload_hash(entry_id, revision_id), payload_hash(entry_id, revision_id));
        assert_ne!(payload_hash(entry_id, revision_id), payload_hash(entry_id, Uuid::from_u128(3)));
        assert!(valid_idempotency_key("edit.key-1"));
        assert!(!valid_idempotency_key("bad key"));
        assert!(!valid_idempotency_key("short"));
    }

    /// 개념: 격리 DB의 접수와 세대 회수.
    /// 목적: 중복 클릭, 다른 본문, 오래된 세대, 재시도 상한이 공개 포인터를 바꾸지 않는지 확인한다.
    /// 사용처: Docker의 `datagrizzly_m2`. 개발 DB 행은 지우지 않는다.
    #[tokio::test]
    #[ignore = "needs DB_URL, DB_USER, DB_PASSWORD and datagrizzly_m2"]
    async fn duplicate_accept_and_stale_worker_do_not_publish() {
        let pool = crate::db::connect_database("datagrizzly_m2").await.expect("isolated database");
        crate::db::migrate(&pool).await.expect("migrate");
        for table in [
            "publication_upload",
            "publication_request",
            "publication_job",
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
            sqlx::query(&format!("DELETE FROM {table}")).execute(&pool).await.expect("clear");
        }
        let password = crate::auth::random_token();
        let anonymous = hit(&pool, "POST", "/api/admin/entries/00000000-0000-4000-8000-000000000009/publications", None, None, r#"{"schemaVersion":1,"revisionId":"00000000-0000-4000-8000-000000000010","idempotencyKey":"edit.key-1"}"#).await;
        assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
        sqlx::query("INSERT INTO owner (password_hash) VALUES ($1)")
            .bind(crate::auth::hash_password(&password).expect("hash"))
            .execute(&pool)
            .await
            .expect("owner");
        let logged_in = hit(&pool, "POST", "/api/admin/session", None, None, &format!(r#"{{"password":"{password}"}}"#)).await;
        assert_eq!(logged_in.status, StatusCode::OK);
        let session = logged_in.cookie.expect("cookie").split(';').next().expect("pair").to_string();
        let csrf = json_of(&logged_in.body)["csrfToken"].as_str().expect("csrf").to_string();
        let created = hit(&pool, "POST", "/api/admin/entries", Some(&session), Some(&csrf), r#"{"slug":"field-note","title":"현장 메모","kind":"note"}"#).await;
        assert_eq!(created.status, StatusCode::CREATED);
        let entry_id = json_of(&created.body)["id"].as_str().expect("id").to_string();
        let current = hit(&pool, "GET", &format!("/api/admin/entries/{entry_id}"), Some(&session), Some(&csrf), "").await;
        let revision_id = json_of(&current.body)["draftRevisionId"].as_str().expect("draft").to_string();
        sqlx::query("UPDATE entry SET published_revision_id = draft_revision_id WHERE id = $1")
            .bind(Uuid::parse_str(&entry_id).expect("entry"))
            .execute(&pool)
            .await
            .expect("pointer");
        let body = format!(r#"{{"schemaVersion":1,"revisionId":"{revision_id}","idempotencyKey":"edit.key-1"}}"#);
        let first = hit(&pool, "POST", &format!("/api/admin/entries/{entry_id}/publications"), Some(&session), Some(&csrf), &body).await;
        assert_eq!(first.status, StatusCode::ACCEPTED);
        let job_id = json_of(&first.body)["jobId"].as_str().expect("job").to_string();
        let second = hit(&pool, "POST", &format!("/api/admin/entries/{entry_id}/publications"), Some(&session), Some(&csrf), &body).await;
        assert_eq!(second.status, StatusCode::ACCEPTED);
        assert_eq!(json_of(&second.body)["jobId"].as_str(), Some(job_id.as_str()));
        let saved = hit(
            &pool,
            "PUT",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1,"title":"현장 메모","summary":"","blocks":[{"id":"11111111-1111-4111-8111-111111111111","type":"paragraph","text":"두번째 문장"}],"metadata":{"type":"note"}}"#,
        )
        .await;
        assert_eq!(saved.status, StatusCode::OK);
        let other_revision = json_of(&saved.body)["draftRevisionId"].as_str().expect("new draft").to_string();
        let changed = format!(r#"{{"schemaVersion":1,"revisionId":"{other_revision}","idempotencyKey":"edit.key-1"}}"#);
        let mismatched = hit(&pool, "POST", &format!("/api/admin/entries/{entry_id}/publications"), Some(&session), Some(&csrf), &changed).await;
        assert_eq!(mismatched.status, StatusCode::CONFLICT);
        let polled = hit(&pool, "POST", "/api/admin/publication-jobs/poll", Some(&session), Some(&csrf), "").await;
        assert_eq!(polled.status, StatusCode::OK);
        let poll_json = json_of(&polled.body);
        assert_eq!(poll_json["succeeded"], false);
        assert_eq!(poll_json["claimedJobId"].as_str(), Some(job_id.as_str()));
        let fence = poll_json["fence"].as_i64().expect("fence");
        let again = hit(&pool, "POST", &format!("/api/admin/publication-jobs/{job_id}/advance"), Some(&session), Some(&csrf), &format!(r#"{{"fence":{fence}}}"#)).await;
        assert_eq!(again.status, StatusCode::OK);
        let uploads: i64 = sqlx::query_scalar("SELECT count(*) FROM publication_upload WHERE job_id = $1 AND generation = $2")
            .bind(Uuid::parse_str(&job_id).expect("job"))
            .bind(fence)
            .fetch_one(&pool)
            .await
            .expect("uploads");
        assert_eq!(uploads, 1);
        sqlx::query("UPDATE publication_job SET lease_until = now() - interval '1 minute' WHERE id = $1")
            .bind(Uuid::parse_str(&job_id).expect("job"))
            .execute(&pool)
            .await
            .expect("expire");
        let resumed = hit(&pool, "POST", "/api/admin/publication-jobs/poll", Some(&session), Some(&csrf), "").await;
        assert_eq!(json_of(&resumed.body)["succeeded"], false);
        let stale = hit(&pool, "POST", &format!("/api/admin/publication-jobs/{job_id}/advance"), Some(&session), Some(&csrf), &format!(r#"{{"fence":{fence}}}"#)).await;
        assert_eq!(stale.status, StatusCode::CONFLICT);
        let published: Uuid = sqlx::query_scalar("SELECT published_revision_id FROM entry WHERE id = $1")
            .bind(Uuid::parse_str(&entry_id).expect("entry"))
            .fetch_one(&pool)
            .await
            .expect("published");
        assert_eq!(published, Uuid::parse_str(&revision_id).expect("revision"));
    }

    fn json_of(body: &str) -> serde_json::Value {
        serde_json::from_str(body).expect("json")
    }

    struct Hit {
        status: StatusCode,
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
        let cookie = response.headers().get("set-cookie").and_then(|value| value.to_str().ok()).map(str::to_string);
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        Hit { status, body: String::from_utf8(bytes.to_vec()).expect("utf-8"), cookie }
    }
}
