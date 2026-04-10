use dlp_api::{CreateDeploymentRequest, CreateDeploymentResponse, GetDeploymentResponse};

use crate::{errors::ClientError, transport::DlpClient};

pub trait DeploymentsClientExt {
    async fn create_deployment(
        &self,
        request: &CreateDeploymentRequest,
    ) -> Result<CreateDeploymentResponse, ClientError>;

    async fn get_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<GetDeploymentResponse, ClientError>;
}

impl DeploymentsClientExt for DlpClient {
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
