//! 개념: API의 시작점.
//! 목적: 15432에 접속해 마이그레이션을 적용한 뒤 health, /api/status, 소유자 API를 연다.
//! 사용처: `cargo run`과 `cargo test`.

mod admin;
mod auth;
mod db;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use sqlx::PgPool;
use std::net::SocketAddr;

const HEALTH: &str = r#"{"groups":["liveness","readiness"],"status":"UP"}"#;
const STATUS_DOWN: &str = r#"{"service":"datagrizzly-api","status":"UP","database":"DOWN"}"#;

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/actuator/health", get(health))
        .route("/api/status", get(status))
        .merge(admin::router())
        .fallback(not_found)
        .with_state(pool)
}

fn status_body(version: Option<i32>) -> (StatusCode, String) {
    match version {
        Some(version) => (
            StatusCode::OK,
            format!(
                r#"{{"service":"datagrizzly-api","status":"UP","database":"UP","schemaVersion":{version}}}"#
            ),
        ),
        None => (StatusCode::SERVICE_UNAVAILABLE, STATUS_DOWN.to_string()),
    }
}

async fn health() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "application/json; charset=utf-8")],
        HEALTH,
    )
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

async fn status(State(pool): State<PgPool>) -> impl IntoResponse {
    let version = sqlx::query_scalar::<_, i32>("SELECT schema_version FROM schema_meta LIMIT 1")
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();
    let (code, body) = status_body(version);
    (
        code,
        [("content-type", "application/json; charset=utf-8")],
        body,
    )
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("api: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let config = db::DbConfig::from_env()?;
    let pool = db::connect(&config).await?;
    db::migrate(&pool).await?;
    db::ensure_owner(&pool).await?;
    let address: SocketAddr = "127.0.0.1:8080".parse().expect("loopback address");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|_| "bind 127.0.0.1:8080".to_string())?;
    axum::serve(listener, app(pool))
        .await
        .map_err(|_| "server".to_string())
}

#[cfg(test)]
mod tests {
    use super::{app, status_body, HEALTH, STATUS_DOWN};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn router() -> axum::Router {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://127.0.0.1:15432/datagrizzly")
            .expect("lazy pool");
        app(pool)
    }

    #[test]
    fn status_reports_schema_version() {
        let (code, body) = status_body(Some(1));
        assert_eq!(code, StatusCode::OK);
        assert_eq!(
            body,
            r#"{"service":"datagrizzly-api","status":"UP","database":"UP","schemaVersion":1}"#
        );
        let (down, down_body) = status_body(None);
        assert_eq!(down, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(down_body, STATUS_DOWN);
    }

    #[tokio::test]
    async fn health_is_up() {
        let response = router()
            .oneshot(
                Request::get("/actuator/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.expect("body").to_bytes();
        assert_eq!(bytes.as_ref(), HEALTH.as_bytes());
    }

    #[tokio::test]
    async fn env_actuator_is_hidden() {
        let response = router()
            .oneshot(
                Request::get("/actuator/env")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[ignore = "needs DB_URL, DB_USER, DB_PASSWORD and the compose database"]
    async fn schema_meta_is_version_one() {
        let config = crate::db::DbConfig::from_env().expect("db env");
        let pool = crate::db::connect(&config).await.expect("connect");
        crate::db::migrate(&pool).await.expect("migrate");
        crate::db::migrate(&pool).await.expect("migrate again");
        let version: i32 = sqlx::query_scalar::<_, i32>("SELECT schema_version FROM schema_meta")
            .fetch_one(&pool)
            .await
            .expect("schema_meta");
        assert_eq!(version, 1);
        let extra: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name NOT IN ('schema_meta', '_sqlx_migrations')",
        )
        .fetch_one(&pool)
        .await
        .expect("tables");
        assert_eq!(extra, 0);
    }
}
