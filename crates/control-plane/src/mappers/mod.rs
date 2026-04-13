#![expect(
    clippy::redundant_pub_crate,
    reason = "Mapper functions are shared between sibling modules through a private parent module."
)]
#![expect(
    clippy::missing_const_for_fn,
    reason = "These tiny mapping helpers stay non-const for readability and consistency."
)]
#![expect(
    clippy::needless_pass_by_value,
    reason = "DTO and enum mapping helpers intentionally consume owned values from request/ORM layers."
)]

use dlp_api::{
    deployments::{DeploymentDto, DeploymentStatusSummaryDto},
    replicas::{ReplicaDto, ReplicaState as ReplicaStateDto},
    shared::{
        DeviceClass as DeviceClassDto, Framework as FrameworkDto, WorkloadMode as WorkloadModeDto,
        WorkloadRequirementDto,
    },
    workers::{WorkerAssignmentDto, WorkerCapabilityDto, WorkerDto, WorkerState as WorkerStateDto},
};
use dlp_domain::{
    ArchitectureFamily, ArtifactRef, Deployment, DeploymentStatusSummary, DeviceClass,
    DomainResult, Framework, Lease, Replica, ReplicaState, RuntimeName, Worker, WorkerCapability,
    WorkerCapabilitySpec, WorkerState, WorkloadMode, WorkloadProfile, WorkloadRequirement,
    WorkloadRequirementSpec,
};

pub(super) fn deployment_to_dto(deployment: &Deployment) -> DeploymentDto {
    DeploymentDto {
        artifact_ref:     deployment.artifact_ref().to_string(),
        id:               deployment.id().to_string(),
        name:             deployment.name().to_owned(),
        replicas_desired: deployment.replicas_desired(),
        requirement:      requirement_to_dto(deployment.requirement()),
        status:           deployment_status_to_dto(deployment.status()),
    }
}

pub(super) fn deployment_status_to_dto(
    status: &DeploymentStatusSummary,
) -> DeploymentStatusSummaryDto {
    DeploymentStatusSummaryDto {
        assigned_replicas: status.assigned_replicas(),
        failed_replicas:   status.failed_replicas(),
        pending_replicas:  status.pending_replicas(),
        pulling_replicas:  status.pulling_replicas(),
        ready_replicas:    status.ready_replicas(),
        starting_replicas: status.starting_replicas(),
        stopped_replicas:  status.stopped_replicas(),
    }
}

pub(super) fn replica_to_dto(replica: &Replica) -> ReplicaDto {
    ReplicaDto {
        deployment_id:  replica.deployment_id().to_string(),
        id:             replica.id().to_string(),
        lease_id:       replica.lease_id().map(ToString::to_string),
        state:          replica_state_to_dto(replica.state()),
        status_message: replica.status_message().map(str::to_owned),
        worker_id:      replica.worker_id().map(ToString::to_string),
    }
}

pub(super) fn worker_to_dto(worker: &Worker) -> WorkerDto {
    WorkerDto {
        assigned_replicas: worker.assigned_replicas(),
        available_slots:   worker.available_slots(),
        capabilities:      worker
            .capabilities()
            .iter()
            .map(capability_to_dto)
            .collect(),
        display_name:      worker.display_name().to_owned(),
        id:                worker.id().to_string(),
        state:             worker_state_to_dto(worker.state()),
    }
}

pub(super) fn capability_to_dto(capability: &WorkerCapability) -> WorkerCapabilityDto {
    WorkerCapabilityDto {
        accelerator_runtime:    capability.accelerator_runtime().to_string(),
        architecture_family:    capability.architecture_family().to_string(),
        available_memory_bytes: capability.available_memory_bytes(),
        concurrency_slots:      capability.concurrency_slots(),
        device:                 device_to_dto(capability.device()),
        framework:              framework_to_dto(capability.framework()),
        mode:                   mode_to_dto(capability.mode()),
    }
}

pub(super) fn requirement_to_dto(requirement: &WorkloadRequirement) -> WorkloadRequirementDto {
    WorkloadRequirementDto {
        accelerator_runtime:      requirement.accelerator_runtime().to_string(),
        architecture_family:      requirement.architecture_family().to_string(),
        concurrency_requirement:  requirement.concurrency_requirement(),
        device:                   device_to_dto(requirement.device()),
        framework:                framework_to_dto(requirement.framework()),
        memory_requirement_bytes: requirement.memory_requirement_bytes(),
        mode:                     mode_to_dto(requirement.mode()),
    }
}

pub(super) fn assignment_to_dto(
    deployment: &Deployment,
    lease: &Lease,
    replica: &Replica,
) -> WorkerAssignmentDto {
    WorkerAssignmentDto {
        artifact_ref:  deployment.artifact_ref().to_string(),
        deployment_id: deployment.id().to_string(),
        lease_id:      lease.id().to_string(),
        replica_id:    replica.id().to_string(),
        requirement:   requirement_to_dto(lease.requirement()),
        worker_id:     lease.worker_id().to_string(),
    }
}

pub(super) fn requirement_from_dto(
    dto: WorkloadRequirementDto,
) -> DomainResult<WorkloadRequirement> {
    Ok(WorkloadRequirement::new(WorkloadRequirementSpec::new(
        WorkloadProfile::new(
            framework_from_dto(dto.framework),
            mode_from_dto(dto.mode),
            device_from_dto(dto.device),
            RuntimeName::new(dto.accelerator_runtime)?,
            ArchitectureFamily::new(dto.architecture_family)?,
        ),
        dto.memory_requirement_bytes,
        dto.concurrency_requirement,
    )))
}

pub(super) fn capability_from_dto(
    dto: WorkerCapabilityDto,
) -> DomainResult<WorkerCapability> {
    Ok(WorkerCapability::new(WorkerCapabilitySpec::new(
        WorkloadProfile::new(
            framework_from_dto(dto.framework),
            mode_from_dto(dto.mode),
            device_from_dto(dto.device),
            RuntimeName::new(dto.accelerator_runtime)?,
            ArchitectureFamily::new(dto.architecture_family)?,
        ),
        dto.available_memory_bytes,
        dto.concurrency_slots,
    )))
}

pub(super) fn artifact_ref_from_string(value: String) -> DomainResult<ArtifactRef> {
    ArtifactRef::new(value)
}

pub(super) fn replica_state_from_dto(state: ReplicaStateDto) -> ReplicaState {
    match state {
        ReplicaStateDto::Assigned => ReplicaState::Assigned,
        ReplicaStateDto::Failed => ReplicaState::Failed,
        ReplicaStateDto::Pending => ReplicaState::Pending,
        ReplicaStateDto::Pulling => ReplicaState::Pulling,
        ReplicaStateDto::Ready => ReplicaState::Ready,
        ReplicaStateDto::Starting => ReplicaState::Starting,
        ReplicaStateDto::Stopped => ReplicaState::Stopped,
    }
}

pub(super) fn worker_state_from_dto(state: WorkerStateDto) -> WorkerState {
    match state {
        WorkerStateDto::Draining => WorkerState::Draining,
        WorkerStateDto::Lost => WorkerState::Lost,
        WorkerStateDto::Ready => WorkerState::Ready,
        WorkerStateDto::Starting => WorkerState::Starting,
        WorkerStateDto::Unhealthy => WorkerState::Unhealthy,
    }
}

fn replica_state_to_dto(state: &ReplicaState) -> ReplicaStateDto {
    match state {
        ReplicaState::Assigned => ReplicaStateDto::Assigned,
        ReplicaState::Failed => ReplicaStateDto::Failed,
        ReplicaState::Pending => ReplicaStateDto::Pending,
        ReplicaState::Pulling => ReplicaStateDto::Pulling,
        ReplicaState::Ready => ReplicaStateDto::Ready,
        ReplicaState::Starting => ReplicaStateDto::Starting,
        ReplicaState::Stopped => ReplicaStateDto::Stopped,
    }
}

fn worker_state_to_dto(state: &WorkerState) -> WorkerStateDto {
    match state {
        WorkerState::Draining => WorkerStateDto::Draining,
        WorkerState::Lost => WorkerStateDto::Lost,
        WorkerState::Ready => WorkerStateDto::Ready,
        WorkerState::Starting => WorkerStateDto::Starting,
        WorkerState::Unhealthy => WorkerStateDto::Unhealthy,
    }
}

fn framework_to_dto(value: &Framework) -> FrameworkDto {
    match value {
        Framework::Max => FrameworkDto::Max,
        Framework::Pytorch => FrameworkDto::Pytorch,
    }
}

fn framework_from_dto(value: FrameworkDto) -> Framework {
    match value {
        FrameworkDto::Max => Framework::Max,
        FrameworkDto::Pytorch => Framework::Pytorch,
    }
}

fn mode_to_dto(value: &WorkloadMode) -> WorkloadModeDto {
    match value {
        WorkloadMode::Inference => WorkloadModeDto::Inference,
        WorkloadMode::Training => WorkloadModeDto::Training,
    }
}

fn mode_from_dto(value: WorkloadModeDto) -> WorkloadMode {
    match value {
        WorkloadModeDto::Inference => WorkloadMode::Inference,
        WorkloadModeDto::Training => WorkloadMode::Training,
    }
}

fn device_to_dto(value: &DeviceClass) -> DeviceClassDto {
    match value {
        DeviceClass::AppleGpu => DeviceClassDto::AppleGpu,
        DeviceClass::Cpu => DeviceClassDto::Cpu,
        DeviceClass::Cuda => DeviceClassDto::Cuda,
        DeviceClass::Rocm => DeviceClassDto::Rocm,
    }
}

fn device_from_dto(value: DeviceClassDto) -> DeviceClass {
    match value {
        DeviceClassDto::AppleGpu => DeviceClass::AppleGpu,
        DeviceClassDto::Cpu => DeviceClass::Cpu,
        DeviceClassDto::Cuda => DeviceClass::Cuda,
        DeviceClassDto::Rocm => DeviceClass::Rocm,
    }
}
