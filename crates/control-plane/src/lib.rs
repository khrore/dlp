use app_config as _;
use axum::{Json, Router, routing::get};
use clap as _;
use client_sdk as _;
use client_sdk::HealthResponse;
use env_logger as _;
use log as _;
#[cfg(test)]
use serde_json as _;
use tokio as _;
#[cfg(test)]
use tower as _;

pub fn app() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok("control-plane"))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use client_sdk::HealthResponse;
    use tower::util::ServiceExt as _;

    use super::app;

    #[tokio::test]
    async fn health_endpoint_returns_expected_payload() {
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .expect("request");
        let response = app().oneshot(request).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload = serde_json::from_slice::<HealthResponse>(&body).expect("json");
        assert_eq!(payload, HealthResponse::ok("control-plane"));
    }
}
