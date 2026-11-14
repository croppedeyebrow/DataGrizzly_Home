//! 개념: 발행 예약.
//! 목적: 서울 시간 입력을 UTC로 저장하고, 취소가 먼저 끝나면 그 예약으로 작업을 만들지 않는다.
//! 사용처: `/api/admin/entries/{id}/schedule`, `/api/admin/schedules/run`.
//! 예약 실행은 대기 작업을 만들 뿐이다. 공개 포인터는 여기서 바꾸지 않는다.

use crate::admin::{audit, authorize, conflict, invalid, json_response, not_found, unavailable};
use crate::publication::JOB_INSERT;
use crate::seo_sync::canonical_path;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// 개념: 놓친 예약의 처리.
/// 목적: 서버가 쉬고 있던 동안의 예약은 다음 확인에서 바로 대기열에 넣는다.
/// 사용처: 예약 화면과 `run_due`.
pub const LATE_POLICY: &str = "run_immediately";

/// 개념: 예약 상태 이동.
/// 목적: 실행 중에는 취소를 받지 않고, 취소된 예약은 실행하지 않는다.
/// 사용처: 수정, 취소, 실행.
pub fn schedule_transition(state: &str, action: &str) -> Option<&'static str> {
    match (state, action) {
        ("scheduled", "edit") => Some("scheduled"),
        ("scheduled", "cancel") => Some("canceled"),
        ("scheduled", "run") => Some("running"),
        ("running", "finish") => Some("done"),
        ("done", "run") => Some("done"),
        _ => None,
    }
}

/// 개념: 서울 오프셋 입력을 UTC 문자열로 바꾼다.
/// 목적: 화면에 보인 서울 시간과 저장한 UTC가 같은 순간이 되게 한다.
/// 사용처: 예약 생성과 수정.
pub fn seoul_to_utc(input: &str) -> Option<(String, String)> {
    let (body, offset) = input.rsplit_once('+')?;
    if offset != "09:00" {
        return None;
    }
    let (date, time) = body.split_once('T')?;
    let mut date_parts = date.split('-');
    let year: i32 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
    let day: u32 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() {
        return None;
    }
    let mut time_parts = time.split(':');
    let hour: i32 = time_parts.next()?.parse().ok()?;
    let minute: i32 = time_parts.next()?.parse().ok()?;
    let second: i32 = time_parts.next().unwrap_or("00").parse().ok()?;
    if time_parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) || !(0..24).contains(&hour) || !(0..60).contains(&minute) || !(0..60).contains(&second) {
        return None;
    }
    let days = days_from_civil(year, month, day)?;
    let mut minutes = days * 24 * 60 + hour as i64 * 60 + minute as i64 - 9 * 60;
    let mut seconds = second;
    if seconds == 60 {
        seconds = 0;
        minutes += 1;
    }
    let day_index = minutes.div_euclid(24 * 60);
    let minute_of_day = minutes.rem_euclid(24 * 60);
    let (year, month, day) = civil_from_days(day_index)?;
    let hour = minute_of_day / 60;
    let minute = minute_of_day % 60;
    let utc = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{seconds:02}Z");
    let display = normalize_display(input)?;
    Some((utc, display))
}

fn normalize_display(input: &str) -> Option<String> {
    let (body, _) = input.rsplit_once('+')?;
    let (date, time) = body.split_once('T')?;
    let mut time_parts = time.split(':');
    let hour = time_parts.next()?;
    let minute = time_parts.next()?;
    let second = time_parts.next().unwrap_or("00");
    if time_parts.next().is_some() || hour.len() != 2 || minute.len() != 2 || second.len() != 2 {
        return None;
    }
    Some(format!("{date}T{hour}:{minute}:{second}+09:00"))
}

fn days_from_civil(mut year: i32, month: u32, day: u32) -> Option<i64> {
    if month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }
    year -= i32::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = (year - era * 400) as u32;
    let shifted = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * shifted + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era as i64 * 146097 + doe as i64 - 719468)
}

fn civil_from_days(days: i64) -> Option<(i32, u32, u32)> {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    Some((year, month, day))
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/admin/entries/{id}/schedule", post(create_schedule))
        .route("/api/admin/schedules", get(list_schedules))
        .route("/api/admin/schedules/run", post(run_due))
        .route("/api/admin/schedules/{id}/edit", post(edit_schedule))
        .route("/api/admin/schedules/{id}/cancel", post(cancel_schedule))
        .route("/api/redirects", get(read_redirect))
}

#[derive(Deserialize)]
struct ScheduleBody {
    #[serde(rename = "schemaVersion")]
    schema_version: i32,
    #[serde(rename = "revisionId")]
    revision_id: String,
    #[serde(rename = "scheduledAt")]
    scheduled_at: String,
}

#[derive(Deserialize)]
struct EditBody {
    #[serde(rename = "schemaVersion")]
    schema_version: i32,
    #[serde(rename = "expectedVersion")]
    expected_version: i32,
    #[serde(rename = "scheduledAt")]
    scheduled_at: String,
}

#[derive(Deserialize)]
struct CancelBody {
    #[serde(rename = "schemaVersion")]
    schema_version: i32,
    #[serde(rename = "expectedVersion")]
    expected_version: i32,
}

#[derive(Deserialize)]
struct RedirectQuery {
    from: Option<String>,
}

/// 개념: 서울 시간으로 발행 예약을 만든다.
/// 목적: 저장한 UTC와 화면에 돌려주는 서울 시간이 같은 순간이 되게 한다.
/// 사용처: 편집 화면.
async fn create_schedule(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(entry_id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<ScheduleBody>(&body) else {
        return invalid();
    };
    if input.schema_version != 1 {
        return invalid_field("schemaVersion");
    }
    let Some(revision_id) = parse_id(&input.revision_id) else {
        return invalid_field("revisionId");
    };
    let Some((utc, display)) = seoul_to_utc(input.scheduled_at.trim()) else {
        return invalid_field("scheduledAt");
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let entry = sqlx::query_as::<_, (String, String)>("SELECT slug, kind FROM entry WHERE id = $1")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await;
    let (slug, kind) = match entry {
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
    let open: Option<Uuid> = match sqlx::query_scalar("SELECT id FROM publication_schedule WHERE entry_id = $1 AND state IN ('scheduled', 'running')")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    if open.is_some() {
        let _ = tx.rollback().await;
        return conflict();
    }
    let schedule_id = Uuid::new_v4();
    if sqlx::query(
        "INSERT INTO publication_schedule (id, entry_id, revision_id, run_at, display_at, from_path, state)
         VALUES ($1, $2, $3, $4::timestamptz, $5, $6, 'scheduled')",
    )
    .bind(schedule_id)
    .bind(entry_id)
    .bind(revision_id)
    .bind(&utc)
    .bind(&display)
    .bind(canonical_path(&kind, &slug))
    .execute(tx.as_mut())
    .await
    .is_err()
    {
        let _ = tx.rollback().await;
        return unavailable();
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "schedule_created").await;
    schedule_body(schedule_id, &display, &utc, 1, "scheduled")
}

/// 개념: 예약 시각을 바꾼다.
/// 목적: 기대한 버전과 예약 상태가 같을 때만 저장한다.
/// 사용처: 예약 수정.
async fn edit_schedule(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<EditBody>(&body) else {
        return invalid();
    };
    if input.schema_version != 1 {
        return invalid_field("schemaVersion");
    }
    if schedule_transition("scheduled", "edit").is_none() {
        return conflict();
    }
    let Some((utc, display)) = seoul_to_utc(input.scheduled_at.trim()) else {
        return invalid_field("scheduledAt");
    };
    let updated = sqlx::query(
        "UPDATE publication_schedule
         SET run_at = $3::timestamptz, display_at = $4, version = version + 1, updated_at = now()
         WHERE id = $1 AND version = $2 AND state = 'scheduled'",
    )
    .bind(id)
    .bind(input.expected_version)
    .bind(&utc)
    .bind(&display)
    .execute(&pool)
    .await;
    let Ok(updated) = updated else {
        return unavailable();
    };
    if updated.rows_affected() != 1 {
        return conflict();
    }
    schedule_body(id, &display, &utc, input.expected_version + 1, "scheduled")
}

/// 개념: 아직 실행 전인 예약만 취소한다.
/// 목적: 취소가 저장되면 이후 실행이 작업을 만들지 못하게 한다.
/// 사용처: 발행 작업 화면.
async fn cancel_schedule(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<CancelBody>(&body) else {
        return invalid();
    };
    if input.schema_version != 1 {
        return invalid_field("schemaVersion");
    }
    if schedule_transition("running", "cancel").is_some() {
        return conflict();
    }
    let updated = sqlx::query(
        "UPDATE publication_schedule
         SET state = 'canceled', version = version + 1, updated_at = now()
         WHERE id = $1 AND version = $2 AND state = 'scheduled'",
    )
    .bind(id)
    .bind(input.expected_version)
    .execute(&pool)
    .await;
    let Ok(updated) = updated else {
        return unavailable();
    };
    if updated.rows_affected() != 1 {
        return conflict();
    }
    let _ = audit(&pool, Some(principal.owner_id), "schedule_canceled").await;
    json_response(StatusCode::OK, json!({"schemaVersion": 1, "state": "canceled", "activated": false}).to_string(), None)
}

/// 개념: 시각이 지난 예약 하나를 대기 작업으로 넣는다.
/// 목적: 놓친 예약을 바로 실행하되, 취소되었거나 이미 끝난 예약은 다시 넣지 않는다.
/// 사용처: 발행 작업 화면의 예약 확인.
async fn run_due(State(pool): State<PgPool>, headers: HeaderMap) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let due = sqlx::query_as::<_, (Uuid, i32, Uuid, Uuid, String, Option<Uuid>)>(
        "SELECT id, version, entry_id, revision_id, from_path, job_id
         FROM publication_schedule
         WHERE state = 'scheduled' AND run_at <= now()
         ORDER BY run_at
         FOR UPDATE SKIP LOCKED
         LIMIT 1",
    )
    .fetch_optional(tx.as_mut())
    .await;
    let Ok(due) = due else {
        let _ = tx.rollback().await;
        return unavailable();
    };
    let Some((id, version, entry_id, revision_id, scheduled_slug, existing_job)) = due else {
        let _ = tx.rollback().await;
        return json_response(
            StatusCode::OK,
            json!({"schemaVersion": 1, "policy": LATE_POLICY, "activated": false, "enqueued": false}).to_string(),
            None,
        );
    };
    if schedule_transition("scheduled", "run") != Some("running") {
        let _ = tx.rollback().await;
        return conflict();
    }
    let claimed = sqlx::query(
        "UPDATE publication_schedule
         SET state = 'running', version = version + 1, updated_at = now()
         WHERE id = $1 AND version = $2 AND state = 'scheduled'",
    )
    .bind(id)
    .bind(version)
    .execute(tx.as_mut())
    .await;
    let Ok(claimed) = claimed else {
        let _ = tx.rollback().await;
        return unavailable();
    };
    if claimed.rows_affected() != 1 {
        let _ = tx.rollback().await;
        return json_response(
            StatusCode::OK,
            json!({"schemaVersion": 1, "policy": LATE_POLICY, "activated": false, "enqueued": false, "scheduleId": id.to_string()}).to_string(),
            None,
        );
    }
    let job_id = if let Some(job_id) = existing_job {
        job_id
    } else {
        let job_id = Uuid::new_v4();
        if sqlx::query(JOB_INSERT)
            .bind(job_id)
            .bind(entry_id)
            .bind(revision_id)
            .execute(tx.as_mut())
            .await
            .is_err()
        {
            let _ = tx.rollback().await;
            return unavailable();
        }
        job_id
    };
    let current = sqlx::query_as::<_, (String, String)>("SELECT slug, kind FROM entry WHERE id = $1")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await;
    if let Ok(Some((slug, kind))) = current {
        let to_path = canonical_path(&kind, &slug);
        if let Some((from_path, to_path)) = crate::seo_sync::redirect_for(Some(&scheduled_slug), &to_path) {
            let _ = sqlx::query(
                "INSERT INTO entry_redirect (id, entry_id, from_path, to_path)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (from_path) DO UPDATE SET to_path = EXCLUDED.to_path",
            )
            .bind(Uuid::new_v4())
            .bind(entry_id)
            .bind(from_path)
            .bind(to_path)
            .execute(tx.as_mut())
            .await;
        }
    }
    if schedule_transition("running", "finish") != Some("done") {
        let _ = tx.rollback().await;
        return conflict();
    }
    if sqlx::query("UPDATE publication_schedule SET state = 'done', job_id = $2, version = version + 1, updated_at = now() WHERE id = $1 AND state = 'running'")
        .bind(id)
        .bind(job_id)
        .execute(tx.as_mut())
        .await
        .is_err()
    {
        let _ = tx.rollback().await;
        return unavailable();
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "schedule_enqueued").await;
    json_response(
        StatusCode::OK,
        json!({"schemaVersion": 1, "policy": LATE_POLICY, "activated": false, "enqueued": true, "scheduleId": id.to_string(), "jobId": job_id.to_string()}).to_string(),
        None,
    )
}

/// 개념: 예약 목록.
/// 목적: 서울 표시 시각과 상태만 보여 주고, 놓친 예약의 정책을 같이 알린다.
/// 사용처: `/admin/publications`.
async fn list_schedules(State(pool): State<PgPool>, headers: HeaderMap) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, i32, String)>(
        "SELECT id, entry_id, display_at, to_char(run_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), version, state
         FROM publication_schedule
         ORDER BY run_at DESC
         LIMIT 8",
    )
    .fetch_all(&pool)
    .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let schedules = rows
        .into_iter()
        .map(|(id, entry_id, display_at, run_at, version, state)| {
            json!({
                "id": id.to_string(),
                "entryId": entry_id.to_string(),
                "displayAt": display_at,
                "runAt": run_at,
                "version": version,
                "state": state,
                "cancelable": state == "scheduled",
            })
        })
        .collect::<Vec<_>>();
    json_response(
        StatusCode::OK,
        json!({"schemaVersion": 1, "policy": LATE_POLICY, "schedules": schedules}).to_string(),
        None,
    )
}

/// 개념: 저장된 슬러그 변경을 읽는다.
/// 목적: 옛 경로의 301 대상을 도메인 없이 돌려준다.
/// 사용처: `GET /api/redirects`.
async fn read_redirect(State(pool): State<PgPool>, Query(query): Query<RedirectQuery>) -> Response {
    let Some(from) = query.from.as_deref() else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (String,)>("SELECT to_path FROM entry_redirect WHERE from_path = $1")
        .bind(from)
        .fetch_optional(&pool)
        .await;
    let Ok(Some((to_path,))) = row else {
        return not_found();
    };
    json_response(
        StatusCode::OK,
        json!({"schemaVersion": 1, "from": from, "to": to_path, "status": 301}).to_string(),
        None,
    )
}

fn schedule_body(id: Uuid, display: &str, utc: &str, version: i32, state: &str) -> Response {
    json_response(
        StatusCode::OK,
        json!({
            "schemaVersion": 1,
            "scheduleId": id.to_string(),
            "displayAt": display,
            "runAt": utc,
            "version": version,
            "state": state,
            "activated": false,
        })
        .to_string(),
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
    use super::{schedule_transition, seoul_to_utc, LATE_POLICY};

    /// 개념: 서울 입력과 취소 경합.
    /// 목적: 표시 시간과 UTC가 같은 순간이고, 취소 뒤에는 실행이 활성화로 이어지지 않는지 확인한다.
    /// 사용처: `cargo test`.
    #[test]
    fn seoul_display_matches_utc_and_cancel_blocks_the_run() {
        let (utc, display) = seoul_to_utc("2026-11-14T21:30:00+09:00").expect("seoul");
        assert_eq!(utc, "2026-11-14T12:30:00Z");
        assert_eq!(display, "2026-11-14T21:30:00+09:00");
        let (rolled, _) = seoul_to_utc("2026-11-14T00:30:00+09:00").expect("midnight");
        assert_eq!(rolled, "2026-11-13T15:30:00Z");
        let (leap, _) = seoul_to_utc("2024-03-01T08:00:00+09:00").expect("leap");
        assert_eq!(leap, "2024-02-29T23:00:00Z");
        assert!(seoul_to_utc("2026-11-14T21:30:00Z").is_none());
        assert!(seoul_to_utc("2026-11-14T21:30:00+00:00").is_none());
        assert_eq!(schedule_transition("scheduled", "cancel"), Some("canceled"));
        assert_eq!(schedule_transition("canceled", "run"), None);
        assert_eq!(schedule_transition("running", "cancel"), None);
        assert_eq!(schedule_transition("scheduled", "run"), Some("running"));
        assert_eq!(schedule_transition("done", "run"), Some("done"));
        assert_eq!(LATE_POLICY, "run_immediately");
    }
}
