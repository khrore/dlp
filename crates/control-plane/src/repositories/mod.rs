mod memory;
mod migration;
mod postgres;

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use dlp_api::workers::WorkerAssignmentDto;
use dlp_domain::{
    Deployment, DeploymentId, DomainError, Lease, LeaseId, Replica, ReplicaId, ReplicaState,
    Worker, WorkerId, WorkerState,
};

/// Internal storage adapters used by the control plane.
pub use self::{memory::MemoryStorage, migration::Migrator, postgres::PostgresStorage};

/// Result of attempting to apply a replica status transition in storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateReplicaStatusResult {
    /// The lease in the request conflicts with the stored lease.
    LeaseConflict(String),
    /// The status update succeeded and returned the updated replica.
    Success(Replica),
    /// The target replica does not exist.
    UnknownReplica,
}

/// Internal storage abstraction implemented by in-memory and `PostgreSQL`
/// backends.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Allocates the next deployment identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot allocate an identifier.
    async fn next_deployment_id(&self) -> Result<String>;
    /// Allocates the next replica identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot allocate an identifier.
    async fn next_replica_id(&self) -> Result<String>;
    /// Allocates the next lease identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot allocate an identifier.
    async fn next_lease_id(&self) -> Result<String>;

    /// Persists a deployment and its initial replica set atomically.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot save the deployment and
    /// replicas.
    async fn create_deployment_with_replicas(
        &self,
        deployment: Deployment,
        replicas: Vec<Replica>,
    ) -> Result<()>;

    /// Loads one deployment by identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn deployment(&self, deployment_id: &DeploymentId) -> Result<Option<Deployment>>;
    /// Lists all deployment identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn deployment_ids(&self) -> Result<Vec<DeploymentId>>;
    /// Saves a deployment snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot persist the deployment.
    async fn save_deployment(&self, deployment: Deployment) -> Result<()>;

    /// Loads one replica by identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn replica(&self, replica_id: &ReplicaId) -> Result<Option<Replica>>;
    /// Saves a replica snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot persist the replica.
    async fn save_replica(&self, replica: Replica) -> Result<()>;
    /// Lists all replicas for a deployment.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn replicas_for_deployment(&self, deployment_id: &DeploymentId) -> Result<Vec<Replica>>;
    /// Lists replicas, optionally filtered by deployment.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn list_replicas(&self, deployment_id: Option<&DeploymentId>) -> Result<Vec<Replica>>;
    /// Lists replicas still waiting for assignment.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn pending_replica_ids(&self) -> Result<Vec<ReplicaId>>;

    /// Loads one lease by identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn lease(&self, lease_id: &LeaseId) -> Result<Option<Lease>>;
    /// Saves a lease snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot persist the lease.
    async fn save_lease(&self, lease: Lease) -> Result<()>;
    /// Lists active leases for one worker.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn active_leases_for_worker(&self, worker_id: &WorkerId) -> Result<Vec<Lease>>;
    /// Lists active lease identifiers for one worker.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn active_lease_ids_for_worker(&self, worker_id: &WorkerId) -> Result<Vec<LeaseId>>;

    /// Loads one worker by identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn worker(&self, worker_id: &WorkerId) -> Result<Option<Worker>>;
    /// Lists all worker identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn worker_ids(&self) -> Result<Vec<WorkerId>>;
    /// Lists workers eligible for scheduling consideration.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn ready_workers(&self) -> Result<Vec<Worker>>;

    /// Registers or replaces a worker record.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot persist the worker record.
    async fn register_worker(&self, worker: Worker, restart_message: &str) -> Result<()>;

    /// Applies a worker heartbeat and returns any queued assignments.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot update the worker record.
    async fn heartbeat_worker(
        &self,
        worker_id: &WorkerId,
        worker_state: WorkerState,
    ) -> Result<Option<(Worker, Vec<WorkerAssignmentDto>)>>;

    /// Marks a worker as expired and updates any affected replicas.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot apply the expiration.
    async fn expire_worker(&self, worker_id: &WorkerId, message: &str) -> Result<()>;

    /// Assigns a replica to a worker and enqueues its assignment payload.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot persist the assignment.
    async fn assign_replica(
        &self,
        replica_id: &ReplicaId,
        worker_id: &WorkerId,
        lease: Lease,
        assignment: WorkerAssignmentDto,
    ) -> Result<bool>;

    /// Applies a replica status update guarded by lease validation.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query or update fails.
    async fn update_replica_status(
        &self,
        replica_id: &ReplicaId,
        lease_id: &LeaseId,
        state: ReplicaState,
        status_message: Option<String>,
    ) -> Result<UpdateReplicaStatusResult>;

    /// Updates the cached worker capacity summary.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot persist the worker summary.
    async fn touch_worker_capacity(
        &self,
        worker_id: &WorkerId,
        assigned_replicas: u32,
        available_slots: u32,
    ) -> Result<()>;

    /// Lists workers whose heartbeat age exceeds the provided timeout.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend query fails.
    async fn lost_worker_ids(&self, lost_timeout: Duration) -> Result<Vec<WorkerId>>;
    /// Forces a worker heartbeat age for tests.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend cannot update the worker heartbeat.
    async fn force_last_heartbeat_age(
        &self,
        worker_id: &WorkerId,
        elapsed: Duration,
    ) -> Result<bool>;
}

#[expect(
    dead_code,
    reason = "Reserved helper for future repository transition validation."
)]
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
