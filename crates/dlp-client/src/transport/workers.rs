use async_trait::async_trait;
use dlp_api::{
    health::StatusDto,
    workers::{
        ListWorkersResponse, RegisterWorkerRequest, RegisterWorkerResponse, WorkerHeartbeatRequest,
        WorkerHeartbeatResponse,
    },
};

use super::DlpClient;
use crate::ClientError;

/// Worker endpoints exposed by the API client.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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
