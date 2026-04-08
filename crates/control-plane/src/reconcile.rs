use tokio::time::{MissedTickBehavior, interval};

use crate::state::{DEFAULT_RECONCILE_INTERVAL, DEFAULT_WORKER_LOST_TIMEOUT, SharedState};

pub(crate) async fn reconcile_once(state: &SharedState) {
    let mut guard = state.lock().await;
    guard.reconcile(DEFAULT_WORKER_LOST_TIMEOUT);
}

pub fn spawn_reconcile_loop(state: SharedState) {
    tokio::spawn(async move {
        let mut ticker = interval(DEFAULT_RECONCILE_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            reconcile_once(&state).await;
        }
    });
}
