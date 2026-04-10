use dlp_api::{DeploymentDto, ReplicaDto, WorkerDto};

pub fn format_deployment(deployment: &DeploymentDto) -> String {
    format!(
        "{} ({}) artifact={} desired={} pending={} assigned={} pulling={} starting={} ready={} \
         failed={} stopped={}",
        deployment.name,
        deployment.id,
        deployment.artifact_ref,
        deployment.replicas_desired,
        deployment.status.pending_replicas,
        deployment.status.assigned_replicas,
        deployment.status.pulling_replicas,
        deployment.status.starting_replicas,
        deployment.status.ready_replicas,
        deployment.status.failed_replicas,
        deployment.status.stopped_replicas
    )
}

pub fn format_replica(replica: ReplicaDto) -> String {
    let worker = replica.worker_id.unwrap_or_else(|| "unassigned".to_owned());
    let lease = replica.lease_id.unwrap_or_else(|| "none".to_owned());
    let message = replica
        .status_message
        .unwrap_or_else(|| "no status".to_owned());

    format!(
        "{} deployment={} state={} worker={} lease={} message={}",
        replica.id, replica.deployment_id, replica.state, worker, lease, message
    )
}

pub fn format_worker(worker: WorkerDto) -> String {
    let capabilities = worker
        .capabilities
        .into_iter()
        .map(|capability| {
            format!(
                "{}/{}/{} runtime={} arch={} mem={} slots={}",
                capability.framework,
                capability.mode,
                capability.device,
                capability.accelerator_runtime,
                capability.architecture_family,
                capability.available_memory_bytes,
                capability.concurrency_slots
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "{} ({}) state={} assigned={} available_slots={} capabilities=[{}]",
        worker.display_name,
        worker.id,
        worker.state,
        worker.assigned_replicas,
        worker.available_slots,
        capabilities
    )
}
