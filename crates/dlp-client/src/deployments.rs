use dlp_api::deployments::{
    CreateDeploymentRequest, CreateDeploymentResponse, GetDeploymentResponse,
};

use crate::{errors::ClientError, transport::DlpClient};

/// Deployment endpoints exposed by the API client.
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
