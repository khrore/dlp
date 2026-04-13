use std::time::Duration;

pub(crate) const DEFAULT_WORKER_LOST_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const DEFAULT_RECONCILE_INTERVAL: Duration = Duration::from_secs(1);
