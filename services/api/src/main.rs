//! 개념: API의 시작점.
//! 목적: 15432에 접속해 메타 마이그레이션을 적용한 뒤 127.0.0.1:8080에서 health를 연다.
//! 사용처: `cargo run`과 `cargo test`.

mod db;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;

const HEALTH: &str = r#"{"groups":["liveness","readiness"],"status":"UP"}"#;

pub fn app() -> Router {
    Router::new()
        .route("/actuator/health", get(health))
        .fallback(not_found)
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
    let address: SocketAddr = "127.0.0.1:8080".parse().expect("loopback address");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|_| "bind 127.0.0.1:8080".to_string())?;
    axum::serve(listener, app())
        .await
        .map_err(|_| "server".to_string())
}

#[cfg(test)]
mod tests {
    use super::{app, HEALTH};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_is_up() {
        let response = app()
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
        let response = app()
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
