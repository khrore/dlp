use axum::{routing::get, Json, Router};
use client_sdk::HealthResponse;

pub fn app() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok("control-plane"))
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use client_sdk::HealthResponse;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_returns_expected_payload() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: HealthResponse = serde_json::from_slice(&body).expect("json");
        assert_eq!(payload, HealthResponse::ok("control-plane"));
    }
}

