use dlp_api::{ListReplicasResponse, ReplicaDto, UpdateReplicaStatusRequest};

use crate::{errors::ClientError, transport::DlpClient};

pub trait ReplicasClientExt {
    async fn list_replicas(
        &self,
        deployment_id: Option<&str>,
    ) -> Result<ListReplicasResponse, ClientError>;

    async fn update_replica_status(
        &self,
        replica_id: &str,
        request: &UpdateReplicaStatusRequest,
    ) -> Result<ReplicaDto, ClientError>;
}

impl ReplicasClientExt for DlpClient {
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
