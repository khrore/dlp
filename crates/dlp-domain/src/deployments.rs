//! Deployment aggregate.

use crate::{
    artifacts::ArtifactRef,
    ids::DeploymentId,
    replicas::{Replica, ReplicaState},
    requirements::WorkloadRequirement,
};

/// Replica counts grouped by state for one deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeploymentStatusCounts {
    /// Replicas waiting to be assigned.
    pub pending: u32,
    /// Replicas assigned to a worker lease.
    pub assigned: u32,
    /// Replicas pulling runtime artifacts.
    pub pulling: u32,
    /// Replicas starting the runtime.
    pub starting: u32,
    /// Replicas ready to serve work.
    pub ready: u32,
    /// Replicas that failed during startup or execution.
    pub failed: u32,
    /// Replicas stopped by the control plane.
    pub stopped: u32,
}

/// Aggregated replica status summary for a deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentStatusSummary {
    pending:  u32,
    assigned: u32,
    pulling:  u32,
    starting: u32,
    ready:    u32,
    failed:   u32,
    stopped:  u32,
}

impl DeploymentStatusSummary {
    /// Creates an empty deployment status summary.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending:  0,
            assigned: 0,
            pulling:  0,
            starting: 0,
            ready:    0,
            failed:   0,
            stopped:  0,
        }
    }

    /// Returns the number of pending replicas.
    #[must_use]
    pub const fn pending_replicas(&self) -> u32 {
        self.pending
    }

    /// Returns the number of assigned replicas.
    #[must_use]
    pub const fn assigned_replicas(&self) -> u32 {
        self.assigned
    }

    /// Returns the number of pulling replicas.
    #[must_use]
    pub const fn pulling_replicas(&self) -> u32 {
        self.pulling
    }

    /// Returns the number of starting replicas.
    #[must_use]
    pub const fn starting_replicas(&self) -> u32 {
        self.starting
    }

    /// Returns the number of ready replicas.
    #[must_use]
    pub const fn ready_replicas(&self) -> u32 {
        self.ready
    }

    /// Returns the number of failed replicas.
    #[must_use]
    pub const fn failed_replicas(&self) -> u32 {
        self.failed
    }

    /// Returns the number of stopped replicas.
    #[must_use]
    pub const fn stopped_replicas(&self) -> u32 {
        self.stopped
    }

    /// Builds a status summary by scanning the provided replicas.
    #[must_use]
    pub fn from_replicas(replicas: &[Replica]) -> Self {
        replicas
            .iter()
            .fold(Self::new(), |mut summary, replica| {
                match replica.state() {
                    ReplicaState::Pending => {
                        summary.pending = summary.pending.saturating_add(1);
                    }
                    ReplicaState::Assigned => {
                        summary.assigned = summary.assigned.saturating_add(1);
                    }
                    ReplicaState::Pulling => {
                        summary.pulling = summary.pulling.saturating_add(1);
                    }
                    ReplicaState::Starting => {
                        summary.starting = summary.starting.saturating_add(1);
                    }
                    ReplicaState::Ready => {
                        summary.ready = summary.ready.saturating_add(1);
                    }
                    ReplicaState::Failed => {
                        summary.failed = summary.failed.saturating_add(1);
                    }
                    ReplicaState::Stopped => {
                        summary.stopped = summary.stopped.saturating_add(1);
                    }
                }
                summary
            })
    }

    /// Builds a status summary from precomputed counts.
    #[must_use]
    pub const fn from_counts(counts: DeploymentStatusCounts) -> Self {
        Self {
            pending: counts.pending,
            assigned: counts.assigned,
            pulling: counts.pulling,
            starting: counts.starting,
            ready: counts.ready,
            failed: counts.failed,
            stopped: counts.stopped,
        }
    }
}

impl Default for DeploymentStatusSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// Deployment aggregate tracked by the control plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deployment {
    id:               DeploymentId,
    name:             String,
    artifact_ref:     ArtifactRef,
    replicas_desired: u32,
    requirement:      WorkloadRequirement,
    status:           DeploymentStatusSummary,
}

impl Deployment {
    /// Creates a new deployment with an empty status summary.
    #[must_use]
    pub fn new(
        id: DeploymentId,
        name: String,
        artifact_ref: ArtifactRef,
        replicas_desired: u32,
        requirement: WorkloadRequirement,
    ) -> Self {
        Self {
            id,
            name,
            artifact_ref,
            replicas_desired,
            requirement,
            status: DeploymentStatusSummary::default(),
        }
    }

    /// Returns the artifact reference to deploy.
    #[must_use]
    pub const fn artifact_ref(&self) -> &ArtifactRef {
        &self.artifact_ref
    }

    /// Returns the deployment identifier.
    #[must_use]
    pub const fn id(&self) -> &DeploymentId {
        &self.id
    }

    /// Returns the deployment display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the desired replica count.
    #[must_use]
    pub const fn replicas_desired(&self) -> u32 {
        self.replicas_desired
    }

    /// Returns the workload requirement for each replica.
    #[must_use]
    pub const fn requirement(&self) -> &WorkloadRequirement {
        &self.requirement
    }

    /// Returns the aggregated replica status summary.
    #[must_use]
    pub const fn status(&self) -> &DeploymentStatusSummary {
        &self.status
    }

    /// Recomputes the status summary from the provided replicas.
    pub fn refresh_status(&mut self, replicas: &[Replica]) {
        self.status = DeploymentStatusSummary::from_replicas(replicas);
    }

    /// Reconstructs a deployment from persisted state.
    #[must_use]
    pub const fn rehydrate(
        id: DeploymentId,
        name: String,
        artifact_ref: ArtifactRef,
        replicas_desired: u32,
        requirement: WorkloadRequirement,
        status: DeploymentStatusSummary,
    ) -> Self {
        Self {
            id,
            name,
            artifact_ref,
            replicas_desired,
            requirement,
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DeploymentStatusSummary;
    use crate::{
        artifacts::ArtifactRef,
        deployments::Deployment,
        ids::{DeploymentId, ReplicaId},
        replicas::{Replica, ReplicaState},
        requirements::{
            ArchitectureFamily, DeviceClass, Framework, RuntimeName, WorkloadMode,
            WorkloadProfile, WorkloadRequirement, WorkloadRequirementSpec,
        },
    };

    #[test]
    fn deployment_status_summary_counts_replica_states() {
        let deployment_id = DeploymentId::new("deployment-1").expect("valid");
        let mut deployment = Deployment::new(
            deployment_id,
            "trainer".to_owned(),
            ArtifactRef::new("artifact://model").expect("valid"),
            1,
            WorkloadRequirement::new(WorkloadRequirementSpec::new(
                WorkloadProfile::new(
                    Framework::Pytorch,
                    WorkloadMode::Training,
                    DeviceClass::Cpu,
                    RuntimeName::new("cpu").expect("valid"),
                    ArchitectureFamily::new("generic").expect("valid"),
                ),
                1024,
                1,
            )),
        );
        let mut ready = Replica::new_pending(
            ReplicaId::new("replica-1").expect("valid"),
            deployment.id().clone(),
        );
        ready
            .update_status(ReplicaState::Assigned, Some("assigned".to_owned()))
            .expect("valid");
        ready
            .update_status(ReplicaState::Pulling, Some("pulling".to_owned()))
            .expect("valid");
        ready
            .update_status(ReplicaState::Starting, Some("starting".to_owned()))
            .expect("valid");
        ready
            .update_status(ReplicaState::Ready, Some("ready".to_owned()))
            .expect("valid");

        deployment.refresh_status(&[ready]);

        assert_eq!(deployment.status().ready_replicas(), 1);
        assert_eq!(DeploymentStatusSummary::default().failed_replicas(), 0);
    }
}
