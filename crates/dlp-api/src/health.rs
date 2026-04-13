use serde::{Deserialize, Serialize};

/// Health-check payload returned by the control plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusDto {
    /// Service identifier for the responding component.
    pub service: String,
    /// Health status string for the service.
    pub status:  String,
}

impl StatusDto {
    /// Creates a standard successful health response.
    pub fn ok<Service>(service: Service) -> Self
    where
        Service: Into<String>,
    {
        Self {
            service: service.into(),
            status:  "ok".to_owned(),
        }
    }
}
