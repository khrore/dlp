use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status:  String,
    pub service: String,
}

impl HealthResponse {
    pub fn ok(service: impl Into<String>) -> Self {
        Self {
            status:  "ok".to_string(),
            service: service.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    Transport(String),
    HttpStatus {
        code:            u16,
        body:            String,
        body_read_error: Option<String>,
    },
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) => write!(f, "transport error: {message}"),
            Self::HttpStatus {
                code,
                body,
                body_read_error,
            } => {
                if !body.is_empty() {
                    write!(f, "request failed with status {code}: {body}")
                } else if let Some(error) = body_read_error {
                    write!(
                        f,
                        "request failed with status {code} (failed to read error body: {error})"
                    )
                } else {
                    write!(f, "request failed with status {code}")
                }
            }
        }
    }
}

impl Error for ClientError {}

#[derive(Debug, Clone)]
pub struct DlpClient {
    base_url: String,
}

impl DlpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: normalize_base_url(base_url.into()),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn health_url(&self) -> String {
        format!("{}/health", self.base_url)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        let response = reqwest::Client::new()
            .get(self.health_url())
            .send()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let (body, body_read_error) = match response.text().await {
                Ok(body) => (body, None),
                Err(error) => (String::new(), Some(error.to_string())),
            };
            return Err(ClientError::HttpStatus {
                code: status.as_u16(),
                body,
                body_read_error,
            });
        }

        response
            .json::<HealthResponse>()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        let response = gloo_net::http::Request::get(&self.health_url())
            .send()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        let status = response.status();
        if !(200..300).contains(&status) {
            let (body, body_read_error) = match response.text().await {
                Ok(body) => (body, None),
                Err(error) => (String::new(), Some(error.to_string())),
            };
            return Err(ClientError::HttpStatus {
                code: status,
                body,
                body_read_error,
            });
        }

        response
            .json::<HealthResponse>()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }
}

fn normalize_base_url(base_url: String) -> String {
    base_url.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::{ClientError, DlpClient, HealthResponse};

    #[test]
    fn trims_trailing_slashes_from_base_url() {
        let client = DlpClient::new("http://127.0.0.1:3000///");
        assert_eq!(client.base_url(), "http://127.0.0.1:3000");
        assert_eq!(client.health_url(), "http://127.0.0.1:3000/health");
    }

    #[test]
    fn health_ok_response_uses_expected_defaults() {
        assert_eq!(HealthResponse::ok("control-plane"), HealthResponse {
            status:  "ok".to_string(),
            service: "control-plane".to_string(),
        });
    }

    #[test]
    fn http_status_display_includes_error_body() {
        let error = ClientError::HttpStatus {
            code:            500,
            body:            "backend unavailable".to_owned(),
            body_read_error: None,
        };

        assert_eq!(
            error.to_string(),
            "request failed with status 500: backend unavailable"
        );
    }

    #[test]
    fn http_status_display_reports_body_read_failure() {
        let error = ClientError::HttpStatus {
            code:            502,
            body:            String::new(),
            body_read_error: Some("connection reset".to_owned()),
        };

        assert_eq!(
            error.to_string(),
            "request failed with status 502 (failed to read error body: connection reset)"
        );
    }
}
