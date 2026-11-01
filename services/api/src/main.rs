//! 개념: CMS API의 시작점.
//! 목적: 127.0.0.1:8080에서 health만 연다. 데이터베이스와 인증은 아직 없다.
//! 사용처: `cargo run`과 `cargo test`.

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
    let address: SocketAddr = "127.0.0.1:8080".parse().expect("loopback address");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind 127.0.0.1:8080");
    axum::serve(listener, app()).await.expect("server");
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
}
