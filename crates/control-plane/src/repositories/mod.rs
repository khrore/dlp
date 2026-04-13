#![expect(
    clippy::redundant_pub_crate,
    reason = "Repository interfaces are shared between sibling modules through a private parent module."
)]
#![expect(
    unreachable_pub,
    reason = "Repository types are re-exported within a private module tree for sibling access."
)]
#![expect(
    dead_code,
    reason = "Several repository hooks are kept for parity between adapters even if not all are exercised yet."
)]

mod memory;
mod migration;
mod postgres;

pub use self::{memory::MemoryStorage, migration::Migrator, postgres::PostgresStorage};

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use dlp_api::workers::WorkerAssignmentDto;
use dlp_domain::{
    Deployment, DeploymentId, DomainError, Lease, LeaseId, Replica, ReplicaId, ReplicaState,
    Worker, WorkerId, WorkerState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UpdateReplicaStatusResult {
    LeaseConflict(String),
    Success(Replica),
    UnknownReplica,
}

#[async_trait]
pub(super) trait StorageBackend: Send + Sync {
    async fn next_deployment_id(&self) -> Result<String>;
    async fn next_replica_id(&self) -> Result<String>;
    async fn next_lease_id(&self) -> Result<String>;

    async fn create_deployment_with_replicas(
        &self,
        deployment: Deployment,
        replicas: Vec<Replica>,
    ) -> Result<()>;

    async fn deployment(&self, deployment_id: &DeploymentId) -> Result<Option<Deployment>>;
    async fn deployment_ids(&self) -> Result<Vec<DeploymentId>>;
    async fn save_deployment(&self, deployment: Deployment) -> Result<()>;

    async fn replica(&self, replica_id: &ReplicaId) -> Result<Option<Replica>>;
    async fn save_replica(&self, replica: Replica) -> Result<()>;
    async fn replicas_for_deployment(&self, deployment_id: &DeploymentId) -> Result<Vec<Replica>>;
    async fn list_replicas(&self, deployment_id: Option<&DeploymentId>) -> Result<Vec<Replica>>;
    async fn pending_replica_ids(&self) -> Result<Vec<ReplicaId>>;

    async fn lease(&self, lease_id: &LeaseId) -> Result<Option<Lease>>;
    async fn save_lease(&self, lease: Lease) -> Result<()>;
    async fn active_leases_for_worker(&self, worker_id: &WorkerId) -> Result<Vec<Lease>>;
    async fn active_lease_ids_for_worker(&self, worker_id: &WorkerId) -> Result<Vec<LeaseId>>;

    async fn worker(&self, worker_id: &WorkerId) -> Result<Option<Worker>>;
    async fn worker_ids(&self) -> Result<Vec<WorkerId>>;
    async fn ready_workers(&self) -> Result<Vec<Worker>>;

    async fn register_worker(&self, worker: Worker, restart_message: &str) -> Result<()>;

    async fn heartbeat_worker(
        &self,
        worker_id: &WorkerId,
        worker_state: WorkerState,
    ) -> Result<Option<(Worker, Vec<WorkerAssignmentDto>)>>;

    async fn expire_worker(&self, worker_id: &WorkerId, message: &str) -> Result<()>;

    async fn assign_replica(
        &self,
        replica_id: &ReplicaId,
        worker_id: &WorkerId,
        lease: Lease,
        assignment: WorkerAssignmentDto,
    ) -> Result<bool>;

    async fn update_replica_status(
        &self,
        replica_id: &ReplicaId,
        lease_id: &LeaseId,
        state: ReplicaState,
        status_message: Option<String>,
    ) -> Result<UpdateReplicaStatusResult>;

    async fn touch_worker_capacity(
        &self,
        worker_id: &WorkerId,
        assigned_replicas: u32,
        available_slots: u32,
    ) -> Result<()>;

    async fn lost_worker_ids(&self, lost_timeout: Duration) -> Result<Vec<WorkerId>>;
    async fn force_last_heartbeat_age(
        &self,
        worker_id: &WorkerId,
        elapsed: Duration,
    ) -> Result<bool>;
}

#[expect(dead_code, reason = "Reserved helper for future repository transition validation.")]
fn invalid_state_transition(
    entity: &'static str,
    from: &impl ToString,
    to: &impl ToString,
) -> DomainError {
    DomainError::InvalidStateTransition {
        entity,
        from: from.to_string(),
        to: to.to_string(),
    }
}
