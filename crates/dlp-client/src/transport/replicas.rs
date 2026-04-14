use async_trait::async_trait;
use dlp_api::replicas::{ListReplicasResponse, ReplicaDto, UpdateReplicaStatusRequest};

use super::DlpClient;
use crate::ClientError;

/// Replica endpoints exposed by the API client.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Client {
    /// Lists replicas, optionally filtered by deployment id.
    async fn list_replicas(
        &self,
        deployment_id: Option<&str>,
    ) -> Result<ListReplicasResponse, ClientError>;

    /// Updates a replica status for a worker lease.
    async fn update_replica_status(
        &self,
        replica_id: &str,
        request: &UpdateReplicaStatusRequest,
    ) -> Result<ReplicaDto, ClientError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Client for DlpClient {
    async fn list_replicas(
        &self,
        deployment_id: Option<&str>,
    ) -> Result<ListReplicasResponse, ClientError> {
        let url = deployment_id.map_or_else(
            || format!("{}/replicas", self.base_url()),
            |value| format!("{}/replicas?deployment_id={value}", self.base_url()),
        );
        self.get_json(url).await
    }

    async fn update_replica_status(
        &self,
        replica_id: &str,
        request: &UpdateReplicaStatusRequest,
    ) -> Result<ReplicaDto, ClientError> {
        self.post_json(
            format!("{}/replicas/{replica_id}/status", self.base_url()),
            request,
        )
        .await
    }
}
