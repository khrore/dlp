use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum ReplicaState {
    Assigned,
    Failed,
    Pending,
    Pulling,
    Ready,
    Starting,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplicaDto {
    pub deployment_id:  String,
    pub id:             String,
    pub lease_id:       Option<String>,
    pub state:          ReplicaState,
    pub status_message: Option<String>,
    pub worker_id:      Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListReplicasResponse {
    pub replicas: Vec<ReplicaDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateReplicaStatusRequest {
    pub lease_id:       String,
    pub state:          ReplicaState,
    pub status_message: Option<String>,
}
