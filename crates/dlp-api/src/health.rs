use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub service: String,
    pub status:  String,
}

impl HealthResponse {
    #[must_use]
    pub fn ok(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            status:  "ok".to_owned(),
        }
    }
}
