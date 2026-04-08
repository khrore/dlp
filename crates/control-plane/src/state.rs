use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use client_sdk::{
    CreateDeploymentRequest, CreateDeploymentResponse, DeploymentStatusSummary,
    GetDeploymentResponse, LeaseState, ListReplicasResponse, ListWorkersResponse, ModelDeployment,
    ModelReplica, RegisterWorkerRequest, RegisterWorkerResponse, ReplicaState,
    UpdateReplicaStatusRequest, Worker, WorkerAssignment, WorkerHeartbeatResponse, WorkerLease,
    WorkerState, WorkloadRequirement,
};
use tokio::sync::Mutex;

use crate::scheduler::{available_capacity_for_requirement, worker_is_eligible};

pub(crate) const DEFAULT_WORKER_LOST_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const DEFAULT_RECONCILE_INTERVAL: Duration = Duration::from_secs(1);

pub type SharedState = Arc<Mutex<AppState>>;

#[derive(Debug)]
pub struct AppState {
    next_id:     u64,
    workers:     BTreeMap<String, WorkerRecord>,
    deployments: BTreeMap<String, ModelDeployment>,
    replicas:    BTreeMap<String, ModelReplica>,
    leases:      BTreeMap<String, WorkerLease>,
}

#[derive(Debug)]
struct WorkerRecord {
    worker:            Worker,
    last_heartbeat_at: Instant,
    assignment_queue:  VecDeque<WorkerAssignment>,
}

pub fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(AppState::new()))
}

impl AppState {
    pub(crate) fn new() -> Self {
        Self {
            next_id:     0,
            workers:     BTreeMap::new(),
            deployments: BTreeMap::new(),
            replicas:    BTreeMap::new(),
            leases:      BTreeMap::new(),
        }
    }

    pub(crate) fn register_worker(
        &mut self,
        request: RegisterWorkerRequest,
    ) -> RegisterWorkerResponse {
        let worker = Worker {
            id:                request.worker_id.clone(),
            display_name:      request.display_name,
            state:             WorkerState::Starting,
            capabilities:      request.capabilities,
            assigned_replicas: 0,
            available_slots:   0,
        };
        let worker_id = worker.id.clone();

        self.workers.insert(worker_id, WorkerRecord {
            worker:            worker.clone(),
            last_heartbeat_at: Instant::now(),
            assignment_queue:  VecDeque::new(),
        });
        self.refresh_worker_summaries();

        RegisterWorkerResponse { worker }
    }

    pub(crate) fn heartbeat_worker(
        &mut self,
        worker_id: &str,
        worker_state: WorkerState,
    ) -> Option<WorkerHeartbeatResponse> {
        let record = self.workers.get_mut(worker_id)?;
        let assignments = record.assignment_queue.drain(..).collect::<Vec<_>>();
        record.last_heartbeat_at = Instant::now();
        record.worker.state = worker_state;
        self.refresh_worker_summaries();

        Some(WorkerHeartbeatResponse {
            worker: self.workers.get(worker_id)?.worker.clone(),
            assignments,
            acknowledged: true,
        })
    }

    pub(crate) fn list_workers(&mut self) -> ListWorkersResponse {
        self.refresh_worker_summaries();
        ListWorkersResponse {
            workers: self
                .workers
                .values()
                .map(|record| record.worker.clone())
                .collect(),
        }
    }

    pub(crate) fn create_deployment(
        &mut self,
        request: CreateDeploymentRequest,
    ) -> CreateDeploymentResponse {
        let deployment_id = self.next_id("deployment");
        let deployment = ModelDeployment {
            id:               deployment_id.clone(),
            name:             request.name,
            artifact_ref:     request.artifact_ref,
            replicas_desired: request.replicas_desired,
            requirement:      request.requirement,
            status:           DeploymentStatusSummary::default(),
        };

        self.deployments
            .insert(deployment_id.clone(), deployment.clone());
        self.ensure_deployment_capacity(&deployment_id);
        self.refresh_deployment_status(&deployment_id);

        let deployment = self
            .deployments
            .get(&deployment_id)
            .cloned()
            .unwrap_or(deployment);

        CreateDeploymentResponse { deployment }
    }

    pub(crate) fn get_deployment(&mut self, deployment_id: &str) -> Option<GetDeploymentResponse> {
        self.refresh_deployment_status(deployment_id);
        let deployment = self.deployments.get(deployment_id)?.clone();
        let replicas = self
            .replicas
            .values()
            .filter(|replica| replica.deployment_id == deployment_id)
            .cloned()
            .collect();

        Some(GetDeploymentResponse {
            deployment,
            replicas,
        })
    }

    pub(crate) fn list_replicas(&self, deployment_id: Option<&str>) -> ListReplicasResponse {
        let replicas = self
            .replicas
            .values()
            .filter(|replica| {
                deployment_id.is_none_or(|requested_id| replica.deployment_id == requested_id)
            })
            .cloned()
            .collect();

        ListReplicasResponse { replicas }
    }

    pub(crate) fn update_replica_status(
        &mut self,
        replica_id: &str,
        request: UpdateReplicaStatusRequest,
    ) -> Option<ModelReplica> {
        let (deployment_id, lease_id, updated_replica) = {
            let replica = self.replicas.get_mut(replica_id)?;
            replica.state = request.state;
            replica.status_message = request.status_message;

            (
                replica.deployment_id.clone(),
                replica.lease_id.clone(),
                replica.clone(),
            )
        };

        if matches!(
            updated_replica.state,
            ReplicaState::Failed | ReplicaState::Stopped
        ) {
            if let Some(lease_id) = lease_id.as_deref() {
                self.release_lease(lease_id);
            }
        }

        self.refresh_deployment_status(&deployment_id);

        Some(updated_replica)
    }

    pub(crate) fn reconcile(&mut self, lost_timeout: Duration) {
        self.expire_lost_workers(lost_timeout);

        let deployment_ids = self.deployments.keys().cloned().collect::<Vec<_>>();
        for deployment_id in deployment_ids {
            self.ensure_deployment_capacity(&deployment_id);
            self.refresh_deployment_status(&deployment_id);
        }

        let pending_replica_ids = self
            .replicas
            .values()
            .filter(|replica| replica.state == ReplicaState::Pending)
            .map(|replica| replica.id.clone())
            .collect::<Vec<_>>();

        for replica_id in pending_replica_ids {
            let Some((worker_id, lease, assignment)) = self.build_assignment(&replica_id) else {
                continue;
            };

            self.leases.insert(lease.id.clone(), lease.clone());

            if let Some(replica) = self.replicas.get_mut(&replica_id) {
                replica.state = ReplicaState::Assigned;
                replica.worker_id = Some(worker_id.clone());
                replica.lease_id = Some(lease.id.clone());
                replica.status_message = Some(format!("assigned to {worker_id}"));
            }

            if let Some(record) = self.workers.get_mut(&worker_id) {
                record.assignment_queue.push_back(assignment);
            }
        }

        self.refresh_worker_summaries();
        let deployment_ids = self.deployments.keys().cloned().collect::<Vec<_>>();
        for deployment_id in deployment_ids {
            self.refresh_deployment_status(&deployment_id);
        }
    }

    fn build_assignment(
        &mut self,
        replica_id: &str,
    ) -> Option<(String, WorkerLease, WorkerAssignment)> {
        let replica = self.replicas.get(replica_id)?.clone();
        let deployment = self.deployments.get(&replica.deployment_id)?.clone();
        let candidate_worker_id = self.select_worker(&deployment.requirement)?;
        let lease_id = self.next_id("lease");
        let lease = WorkerLease {
            id:            lease_id.clone(),
            worker_id:     candidate_worker_id.clone(),
            deployment_id: deployment.id.clone(),
            replica_id:    replica.id.clone(),
            state:         LeaseState::Active,
            requirement:   deployment.requirement.clone(),
        };
        let assignment = WorkerAssignment {
            worker_id: candidate_worker_id.clone(),
            deployment_id: deployment.id,
            replica_id: replica.id,
            lease_id,
            artifact_ref: deployment.artifact_ref,
            requirement: deployment.requirement,
        };

        Some((candidate_worker_id, lease, assignment))
    }

    fn select_worker(&self, requirement: &WorkloadRequirement) -> Option<String> {
        self.workers
            .iter()
            .filter(|(_, record)| record.worker.state == WorkerState::Ready)
            .find_map(|(worker_id, record)| {
                let leases = self
                    .leases
                    .values()
                    .filter(|lease| {
                        lease.worker_id == *worker_id && lease.state == LeaseState::Active
                    })
                    .collect::<Vec<_>>();

                worker_is_eligible(&record.worker, requirement, &leases).then(|| worker_id.clone())
            })
    }

    fn ensure_deployment_capacity(&mut self, deployment_id: &str) {
        let Some(deployment) = self.deployments.get(deployment_id).cloned() else {
            return;
        };

        let active_replicas = self
            .replicas
            .values()
            .filter(|replica| {
                replica.deployment_id == deployment_id
                    && matches!(
                        replica.state,
                        ReplicaState::Pending
                            | ReplicaState::Assigned
                            | ReplicaState::Pulling
                            | ReplicaState::Starting
                            | ReplicaState::Ready
                    )
            })
            .count() as u32;

        if active_replicas >= deployment.replicas_desired {
            return;
        }

        for _ in active_replicas..deployment.replicas_desired {
            let replica_id = self.next_id("replica");
            self.replicas.insert(replica_id.clone(), ModelReplica {
                id:             replica_id,
                deployment_id:  deployment_id.to_string(),
                worker_id:      None,
                lease_id:       None,
                state:          ReplicaState::Pending,
                status_message: Some("pending scheduling".to_string()),
            });
        }
    }

    fn refresh_worker_summaries(&mut self) {
        let worker_ids = self.workers.keys().cloned().collect::<Vec<_>>();
        for worker_id in worker_ids {
            let active_leases = self
                .leases
                .values()
                .filter(|lease| lease.worker_id == worker_id && lease.state == LeaseState::Active)
                .cloned()
                .collect::<Vec<_>>();

            if let Some(record) = self.workers.get_mut(&worker_id) {
                let lease_refs = active_leases.iter().collect::<Vec<_>>();
                let available_slots = record
                    .worker
                    .capabilities
                    .iter()
                    .filter_map(|capability| {
                        available_capacity_for_requirement(
                            &record.worker,
                            &WorkloadRequirement {
                                framework:                capability.framework.clone(),
                                mode:                     capability.mode.clone(),
                                device:                   capability.device.clone(),
                                accelerator_runtime:      capability.accelerator_runtime.clone(),
                                architecture_family:      capability.architecture_family.clone(),
                                memory_requirement_bytes: 0,
                                concurrency_requirement:  0,
                            },
                            &lease_refs,
                        )
                        .map(|(slots, _)| slots)
                    })
                    .fold(0_u32, u32::saturating_add);

                record.worker.assigned_replicas = active_leases.len() as u32;
                record.worker.available_slots = available_slots;
            }
        }
    }

    fn refresh_deployment_status(&mut self, deployment_id: &str) {
        let Some(deployment) = self.deployments.get_mut(deployment_id) else {
            return;
        };

        let summary = self
            .replicas
            .values()
            .filter(|replica| replica.deployment_id == deployment_id)
            .fold(
                DeploymentStatusSummary::default(),
                |mut summary, replica| {
                    match replica.state {
                        ReplicaState::Pending => {
                            summary.pending_replicas = summary.pending_replicas.saturating_add(1);
                        }
                        ReplicaState::Assigned => {
                            summary.assigned_replicas = summary.assigned_replicas.saturating_add(1);
                        }
                        ReplicaState::Pulling => {
                            summary.pulling_replicas = summary.pulling_replicas.saturating_add(1);
                        }
                        ReplicaState::Starting => {
                            summary.starting_replicas = summary.starting_replicas.saturating_add(1);
                        }
                        ReplicaState::Ready => {
                            summary.ready_replicas = summary.ready_replicas.saturating_add(1);
                        }
                        ReplicaState::Failed => {
                            summary.failed_replicas = summary.failed_replicas.saturating_add(1);
                        }
                        ReplicaState::Stopped => {
                            summary.stopped_replicas = summary.stopped_replicas.saturating_add(1);
                        }
                    }
                    summary
                },
            );

        deployment.status = summary;
    }

    fn expire_lost_workers(&mut self, lost_timeout: Duration) {
        let lost_worker_ids = self
            .workers
            .iter()
            .filter(|(_, record)| {
                record.worker.state != WorkerState::Lost
                    && record.last_heartbeat_at.elapsed() >= lost_timeout
            })
            .map(|(worker_id, _)| worker_id.clone())
            .collect::<Vec<_>>();

        for worker_id in lost_worker_ids {
            if let Some(record) = self.workers.get_mut(&worker_id) {
                record.worker.state = WorkerState::Lost;
                record.assignment_queue.clear();
            }

            let affected_leases = self
                .leases
                .values()
                .filter(|lease| lease.worker_id == worker_id && lease.state == LeaseState::Active)
                .map(|lease| lease.id.clone())
                .collect::<Vec<_>>();

            for lease_id in affected_leases {
                if let Some(lease) = self.leases.get_mut(&lease_id) {
                    lease.state = LeaseState::Expired;
                    if let Some(replica) = self.replicas.get_mut(&lease.replica_id) {
                        replica.state = ReplicaState::Stopped;
                        replica.status_message =
                            Some("worker lost before replica completed".to_string());
                    }
                }
            }
        }
    }

    fn release_lease(&mut self, lease_id: &str) {
        if let Some(lease) = self.leases.get_mut(lease_id) {
            lease.state = LeaseState::Released;
        }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.next_id = self.next_id.saturating_add(1);
        format!("{prefix}-{}", self.next_id)
    }

    #[cfg(test)]
    pub(crate) fn force_last_heartbeat_age(&mut self, worker_id: &str, elapsed: Duration) -> bool {
        if let Some(record) = self.workers.get_mut(worker_id) {
            record.last_heartbeat_at = Instant::now() - elapsed;
            return true;
        }

        false
    }
}
