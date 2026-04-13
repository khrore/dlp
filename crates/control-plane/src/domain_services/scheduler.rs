use dlp_domain::{
    leases::Lease,
    requirements::{WorkerCapability, WorkloadRequirement},
    workers::Worker,
};

pub(crate) fn capability_matches(
    capability: &WorkerCapability,
    requirement: &WorkloadRequirement,
) -> bool {
    capability.framework() == requirement.framework()
        && capability.mode() == requirement.mode()
        && capability.device() == requirement.device()
        && capability.accelerator_runtime() == requirement.accelerator_runtime()
        && capability.architecture_family() == requirement.architecture_family()
}

pub(crate) fn available_capacity_for_requirement(
    worker: &Worker,
    requirement: &WorkloadRequirement,
    leases: &[Lease],
) -> Option<(u32, u64)> {
    worker
        .capabilities()
        .iter()
        .find(|capability| capability_matches(capability, requirement))
        .map(|capability| {
            let used_slots = leases
                .iter()
                .filter(|lease| capability_matches(capability, lease.requirement()))
                .fold(0, |total: u32, lease| {
                    total.saturating_add(lease.requirement().concurrency_requirement())
                });
            let used_memory = leases
                .iter()
                .filter(|lease| capability_matches(capability, lease.requirement()))
                .fold(0, |total: u64, lease| {
                    total.saturating_add(lease.requirement().memory_requirement_bytes())
                });

            (
                capability.concurrency_slots().saturating_sub(used_slots),
                capability
                    .available_memory_bytes()
                    .saturating_sub(used_memory),
            )
        })
}

pub(crate) fn worker_is_eligible(
    worker: &Worker,
    requirement: &WorkloadRequirement,
    leases: &[Lease],
) -> bool {
    available_capacity_for_requirement(worker, requirement, leases).is_some_and(
        |(slots, memory)| {
            slots >= requirement.concurrency_requirement()
                && memory >= requirement.memory_requirement_bytes()
        },
    )
}
