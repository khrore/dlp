//! Deployment aggregate.

use crate::{
    artifacts::ArtifactRef,
    ids::DeploymentId,
    replicas::{Replica, ReplicaState},
    requirements::WorkloadRequirement,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentStatusSummary {
    pending_replicas:  u32,
    assigned_replicas: u32,
    pulling_replicas:  u32,
    starting_replicas: u32,
    ready_replicas:    u32,
    failed_replicas:   u32,
    stopped_replicas:  u32,
}

impl DeploymentStatusSummary {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending_replicas:  0,
            assigned_replicas: 0,
            pulling_replicas:  0,
            starting_replicas: 0,
            ready_replicas:    0,
            failed_replicas:   0,
            stopped_replicas:  0,
        }
    }

    #[must_use]
    pub const fn pending_replicas(&self) -> u32 {
        self.pending_replicas
    }

    #[must_use]
    pub const fn assigned_replicas(&self) -> u32 {
        self.assigned_replicas
    }

    #[must_use]
    pub const fn pulling_replicas(&self) -> u32 {
        self.pulling_replicas
    }

    #[must_use]
    pub const fn starting_replicas(&self) -> u32 {
        self.starting_replicas
    }

    #[must_use]
    pub const fn ready_replicas(&self) -> u32 {
        self.ready_replicas
    }

    #[must_use]
    pub const fn failed_replicas(&self) -> u32 {
        self.failed_replicas
    }

    #[must_use]
    pub const fn stopped_replicas(&self) -> u32 {
        self.stopped_replicas
    }

    #[must_use]
    pub fn from_replicas<'a>(replicas: impl IntoIterator<Item = &'a Replica>) -> Self {
        replicas
            .into_iter()
            .fold(Self::new(), |mut summary, replica| {
                match replica.state() {
                    ReplicaState::Pending => summary.pending_replicas += 1,
                    ReplicaState::Assigned => summary.assigned_replicas += 1,
                    ReplicaState::Pulling => summary.pulling_replicas += 1,
                    ReplicaState::Starting => summary.starting_replicas += 1,
                    ReplicaState::Ready => summary.ready_replicas += 1,
                    ReplicaState::Failed => summary.failed_replicas += 1,
                    ReplicaState::Stopped => summary.stopped_replicas += 1,
                }
                summary
            })
    }
}

impl Default for DeploymentStatusSummary {
    fn default() -> Self {
        Self::new()
    }
}

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

    #[must_use]
    pub fn artifact_ref(&self) -> &ArtifactRef {
        &self.artifact_ref
    }

    #[must_use]
    pub fn id(&self) -> &DeploymentId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn replicas_desired(&self) -> u32 {
        self.replicas_desired
    }

    #[must_use]
    pub fn requirement(&self) -> &WorkloadRequirement {
        &self.requirement
    }

    #[must_use]
    pub fn status(&self) -> &DeploymentStatusSummary {
        &self.status
    }

    pub fn refresh_status<'a>(&mut self, replicas: impl IntoIterator<Item = &'a Replica>) {
        self.status = DeploymentStatusSummary::from_replicas(replicas);
    }
}

#[cfg(test)]
mod tests {
    use super::DeploymentStatusSummary;
    use crate::{
        ArtifactRef, Deployment, DeploymentId, Framework, Replica, ReplicaId, ReplicaState,
        RuntimeName, WorkloadMode, WorkloadRequirement,
    };

    #[test]
    fn deployment_status_summary_counts_replica_states() {
        let deployment_id = DeploymentId::new("deployment-1").expect("valid");
        let mut deployment = Deployment::new(
            deployment_id.clone(),
            "trainer".to_owned(),
            ArtifactRef::new("artifact://model").expect("valid"),
            1,
            WorkloadRequirement::new(
                Framework::Pytorch,
                WorkloadMode::Training,
                crate::DeviceClass::Cpu,
                RuntimeName::new("cpu").expect("valid"),
                crate::ArchitectureFamily::new("generic").expect("valid"),
                1024,
                1,
            ),
        );
        let mut ready = Replica::new_pending(
            ReplicaId::new("replica-1").expect("valid"),
            deployment_id.clone(),
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

        deployment.refresh_status([&ready]);

        assert_eq!(deployment.status().ready_replicas(), 1);
        assert_eq!(DeploymentStatusSummary::default().failed_replicas(), 0);
    }
}
