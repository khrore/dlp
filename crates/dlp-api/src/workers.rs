use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::shared::{DeviceClass, Framework, WorkloadMode, WorkloadRequirementDto};

/// Control-plane view of worker lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum WorkerState {
    /// The worker is draining work and should not receive new assignments.
    Draining,
    /// The worker stopped heartbeating and is considered lost.
    Lost,
    /// The worker is ready to accept assignments.
    Ready,
    /// The worker is starting up.
    Starting,
    /// The worker is reachable but unhealthy.
    Unhealthy,
}

/// Capability advertised by a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerCapabilityDto {
    /// Accelerator runtime name, such as `cpu` or `cuda`.
    pub accelerator_runtime:    String,
    /// Architecture family identifier.
    pub architecture_family:    String,
    /// Available memory in bytes.
    pub available_memory_bytes: u64,
    /// Number of concurrent slots supported.
    pub concurrency_slots:      u32,
    /// Device class supported by the worker.
    pub device:                 DeviceClass,
    /// Framework supported by the worker.
    pub framework:              Framework,
    /// Workload mode supported by the worker.
    pub mode:                   WorkloadMode,
}

/// Worker resource returned by API responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerDto {
    /// Number of replicas currently assigned to the worker.
    pub assigned_replicas: u32,
    /// Number of free scheduling slots remaining.
    pub available_slots:   u32,
    /// Capabilities advertised by the worker.
    pub capabilities:      Vec<WorkerCapabilityDto>,
    /// Human-readable worker name.
    pub display_name:      String,
    /// Stable worker identifier.
    pub id:                String,
    /// Current worker state.
    pub state:             WorkerState,
}

/// Assignment payload delivered to a worker heartbeat response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerAssignmentDto {
    /// Artifact reference the worker should run.
    pub artifact_ref:  String,
    /// Owning deployment identifier.
    pub deployment_id: String,
    /// Lease identifier used for status updates.
    pub lease_id:      String,
    /// Replica identifier being assigned.
    pub replica_id:    String,
    /// Scheduling requirement for the assignment.
    pub requirement:   WorkloadRequirementDto,
    /// Worker identifier that owns the assignment.
    pub worker_id:     String,
}

/// Request body for registering a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterWorkerRequest {
    /// Capabilities advertised by the worker.
    pub capabilities: Vec<WorkerCapabilityDto>,
    /// Human-readable worker name.
    pub display_name: String,
    /// Stable worker identifier.
    pub worker_id:    String,
}

/// Response body returned after worker registration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterWorkerResponse {
    /// The registered worker resource.
    pub worker: WorkerDto,
}

/// Heartbeat request body sent by a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerHeartbeatRequest {
    /// Worker state observed at heartbeat time.
    pub state: WorkerState,
}

/// Heartbeat response body returned to a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerHeartbeatResponse {
    /// Whether the control plane accepted the heartbeat.
    pub acknowledged: bool,
    /// Assignments pending delivery to the worker.
    pub assignments:  Vec<WorkerAssignmentDto>,
    /// The latest worker resource snapshot.
    pub worker:       WorkerDto,
}

/// Response body for listing workers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListWorkersResponse {
    /// Workers currently registered with the control plane.
    pub workers: Vec<WorkerDto>,
}

#[cfg(test)]
mod tests {
    use crate::{
        health::StatusDto,
        shared::{DeviceClass, Framework, WorkloadMode},
        workers::{ListWorkersResponse, WorkerCapabilityDto, WorkerDto, WorkerState},
    };

    #[test]
    fn health_ok_response_uses_expected_defaults() {
        assert_eq!(StatusDto::ok("dlp-control-plane"), StatusDto {
            service: "dlp-control-plane".to_owned(),
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
                    device:                 DeviceClass::Cpu,
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
