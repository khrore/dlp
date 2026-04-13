#![expect(
    clippy::redundant_pub_crate,
    reason = "Reconcile constants are shared between sibling modules through a private parent module."
)]
#![expect(
    unreachable_pub,
    reason = "These constants stay public within a private module tree for sibling access."
)]

use std::time::Duration;

pub const DEFAULT_WORKER_LOST_TIMEOUT: Duration = Duration::from_secs(15);
pub const DEFAULT_RECONCILE_INTERVAL: Duration = Duration::from_secs(1);
