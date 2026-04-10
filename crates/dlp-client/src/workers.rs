use dlp_api::{
    HealthResponse, ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse,
    WorkerHeartbeatRequest, WorkerHeartbeatResponse,
};

use crate::{errors::ClientError, transport::DlpClient};

pub trait WorkersClientExt {
    async fn health_check(&self) -> Result<HealthResponse, ClientError>;

    async fn list_workers(&self) -> Result<ListWorkersResponse, ClientError>;

    async fn register_worker(
        &self,
        request: &RegisterWorkerRequest,
    ) -> Result<RegisterWorkerResponse, ClientError>;

    async fn heartbeat_worker(
        &self,
        worker_id: &str,
        request: &WorkerHeartbeatRequest,
    ) -> Result<WorkerHeartbeatResponse, ClientError>;
}

impl WorkersClientExt for DlpClient {
    async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        self.get_json(format!("{}/health", self.base_url())).await
    }

    async fn list_workers(&self) -> Result<ListWorkersResponse, ClientError> {
        self.get_json(format!("{}/workers", self.base_url())).await
    }

    async fn register_worker(
        &self,
        request: &RegisterWorkerRequest,
    ) -> Result<RegisterWorkerResponse, ClientError> {
        self.post_json(format!("{}/workers/register", self.base_url()), request)
            .await
    }

    async fn heartbeat_worker(
        &self,
        worker_id: &str,
        request: &WorkerHeartbeatRequest,
    ) -> Result<WorkerHeartbeatResponse, ClientError> {
        self.post_json(
            format!("{}/workers/{worker_id}/heartbeat", self.base_url()),
            request,
        )
        .await
    }
}
