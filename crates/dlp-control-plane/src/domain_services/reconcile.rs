use std::time::Duration;

/// Default amount of time before a worker is considered lost.
pub const DEFAULT_WORKER_LOST_TIMEOUT: Duration = Duration::from_secs(15);
/// Default interval between reconcile loop iterations.
pub const DEFAULT_RECONCILE_INTERVAL: Duration = Duration::from_secs(1);
