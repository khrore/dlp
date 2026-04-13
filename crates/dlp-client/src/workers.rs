use dlp_api::{
    health::StatusDto,
    workers::{
        ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse,
        WorkerHeartbeatRequest, WorkerHeartbeatResponse,
    },
};

use crate::{errors::ClientError, transport::DlpClient};

/// Worker endpoints exposed by the API client.
pub trait Client {
    /// Calls the health-check endpoint.
    async fn health_check(&self) -> Result<StatusDto, ClientError>;

    /// Lists registered workers.
    async fn list_workers(&self) -> Result<ListWorkersResponse, ClientError>;

    /// Registers or refreshes a worker.
    async fn register_worker(
        &self,
        request: &RegisterWorkerRequest,
    ) -> Result<RegisterWorkerResponse, ClientError>;

    /// Sends a worker heartbeat and receives assignments.
    async fn heartbeat_worker(
        &self,
        worker_id: &str,
        request: &WorkerHeartbeatRequest,
    ) -> Result<WorkerHeartbeatResponse, ClientError>;
}

impl Client for DlpClient {
    async fn health_check(&self) -> Result<StatusDto, ClientError> {
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
