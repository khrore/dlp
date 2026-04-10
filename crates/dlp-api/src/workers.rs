use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::shared::{DeviceClass, Framework, WorkloadMode, WorkloadRequirementDto};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum WorkerState {
    Draining,
    Lost,
    Ready,
    Starting,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerCapabilityDto {
    pub accelerator_runtime:    String,
    pub architecture_family:    String,
    pub available_memory_bytes: u64,
    pub concurrency_slots:      u32,
    pub device:                 DeviceClass,
    pub framework:              Framework,
    pub mode:                   WorkloadMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerDto {
    pub assigned_replicas: u32,
    pub available_slots:   u32,
    pub capabilities:      Vec<WorkerCapabilityDto>,
    pub display_name:      String,
    pub id:                String,
    pub state:             WorkerState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerAssignmentDto {
    pub artifact_ref:  String,
    pub deployment_id: String,
    pub lease_id:      String,
    pub replica_id:    String,
    pub requirement:   WorkloadRequirementDto,
    pub worker_id:     String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterWorkerRequest {
    pub capabilities: Vec<WorkerCapabilityDto>,
    pub display_name: String,
    pub worker_id:    String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterWorkerResponse {
    pub worker: WorkerDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerHeartbeatRequest {
    pub state: WorkerState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerHeartbeatResponse {
    pub acknowledged: bool,
    pub assignments:  Vec<WorkerAssignmentDto>,
    pub worker:       WorkerDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListWorkersResponse {
    pub workers: Vec<WorkerDto>,
}

#[cfg(test)]
mod tests {
    use crate::{
        Framework, HealthResponse, ListWorkersResponse, WorkerCapabilityDto, WorkerDto,
        WorkerState, WorkloadMode,
    };

    #[test]
    fn health_ok_response_uses_expected_defaults() {
        assert_eq!(HealthResponse::ok("control-plane"), HealthResponse {
            service: "control-plane".to_owned(),
            status:  "ok".to_owned(),
        });
    }

    #[test]
    fn serializes_workers() {
        let response = ListWorkersResponse {
            workers: vec![WorkerDto {
                assigned_replicas: 0,
                available_slots:   1,
                capabilities:      vec![WorkerCapabilityDto {
                    accelerator_runtime:    "cpu".to_owned(),
                    architecture_family:    "generic".to_owned(),
                    available_memory_bytes: 1024,
                    concurrency_slots:      1,
                    device:                 crate::DeviceClass::Cpu,
                    framework:              Framework::Pytorch,
                    mode:                   WorkloadMode::Training,
                }],
                display_name:      "trainer".to_owned(),
                id:                "worker-1".to_owned(),
                state:             WorkerState::Ready,
            }],
        };

        let json = serde_json::to_string(&response).expect("serializes");
        assert!(json.contains("worker-1"));
    }
}
