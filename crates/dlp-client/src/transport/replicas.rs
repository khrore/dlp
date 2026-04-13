use dlp_api::replicas::{ListReplicasResponse, ReplicaDto, UpdateReplicaStatusRequest};

use super::DlpClient;
use crate::ClientError;

/// Replica endpoints exposed by the API client.
#[expect(
    async_fn_in_trait,
    reason = "These traits are consumed internally by this workspace and do not need Send future guarantees."
)]
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
