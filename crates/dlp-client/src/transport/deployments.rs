use async_trait::async_trait;
use dlp_api::deployments::{
    CreateDeploymentRequest, CreateDeploymentResponse, GetDeploymentResponse,
};

use super::DlpClient;
use crate::ClientError;

/// Deployment endpoints exposed by the API client.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Client {
    /// Creates a deployment.
    async fn create_deployment(
        &self,
        request: &CreateDeploymentRequest,
    ) -> Result<CreateDeploymentResponse, ClientError>;

    /// Fetches a deployment by id.
    async fn get_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<GetDeploymentResponse, ClientError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Client for DlpClient {
    async fn create_deployment(
        &self,
        request: &CreateDeploymentRequest,
    ) -> Result<CreateDeploymentResponse, ClientError> {
        self.post_json(format!("{}/deployments", self.base_url()), request)
            .await
    }

    async fn get_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<GetDeploymentResponse, ClientError> {
        self.get_json(format!("{}/deployments/{deployment_id}", self.base_url()))
            .await
    }
}
