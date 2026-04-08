use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use client_sdk::{
    DeviceClass, DlpClient, Framework, RegisterWorkerRequest, ReplicaState,
    UpdateReplicaStatusRequest, WorkerAssignment, WorkerCapability, WorkerHeartbeatRequest,
    WorkerState, WorkloadMode,
};
#[cfg(test)]
use control_plane as _;
use tokio::{
    sync::Mutex,
    time::{MissedTickBehavior, interval, sleep},
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const LIFECYCLE_STEP_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Parser)]
#[command(name = "pytorch-worker", about = "Stub PyTorch worker for DLP")]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    api_base_url: String,

    #[arg(long, default_value = "worker-1")]
    worker_id: String,

    #[arg(long, default_value = "PyTorch Worker")]
    display_name: String,

    #[arg(long, default_value_t = WorkloadMode::Training)]
    mode: WorkloadMode,

    #[arg(long, default_value_t = DeviceClass::Cpu)]
    device: DeviceClass,

    #[arg(long, default_value = "cpu")]
    accelerator_runtime: String,

    #[arg(long, default_value = "generic")]
    architecture_family: String,

    #[arg(long, default_value_t = 8192)]
    memory_bytes: u64,

    #[arg(long, default_value_t = 1)]
    concurrency_slots: u32,

    #[arg(long, value_enum, default_value_t = FailureMode::None)]
    failure_mode: FailureMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum FailureMode {
    None,
    BeforeReady,
    AfterReady,
    ExitAfterReady,
}

trait RuntimeProvider {
    fn planned_updates(&self) -> Vec<LifecycleStep>;
}

#[derive(Debug, Clone)]
struct SimulatedProvider {
    failure_mode: FailureMode,
}

impl RuntimeProvider for SimulatedProvider {
    fn planned_updates(&self) -> Vec<LifecycleStep> {
        match self.failure_mode {
            FailureMode::None | FailureMode::ExitAfterReady => vec![
                LifecycleStep::new(ReplicaState::Pulling, "pulling artifacts"),
                LifecycleStep::new(ReplicaState::Starting, "starting runtime"),
                LifecycleStep::new(ReplicaState::Ready, "ready"),
            ],
            FailureMode::BeforeReady => vec![
                LifecycleStep::new(ReplicaState::Pulling, "pulling artifacts"),
                LifecycleStep::new(ReplicaState::Starting, "starting runtime"),
                LifecycleStep::new(ReplicaState::Failed, "simulated failure before ready"),
            ],
            FailureMode::AfterReady => vec![
                LifecycleStep::new(ReplicaState::Pulling, "pulling artifacts"),
                LifecycleStep::new(ReplicaState::Starting, "starting runtime"),
                LifecycleStep::new(ReplicaState::Ready, "ready"),
                LifecycleStep::new(ReplicaState::Failed, "simulated failure after ready"),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LifecycleStep {
    state:   ReplicaState,
    message: String,
}

impl LifecycleStep {
    fn new(state: ReplicaState, message: impl Into<String>) -> Self {
        Self {
            state,
            message: message.into(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = DlpClient::new(args.api_base_url);
    let provider = SimulatedProvider {
        failure_mode: args.failure_mode,
    };
    let active_replicas = Arc::new(Mutex::new(HashMap::<String, ReplicaState>::new()));
    let stop_heartbeats = Arc::new(AtomicBool::new(false));

    client
        .register_worker(&RegisterWorkerRequest {
            worker_id:     args.worker_id.clone(),
            display_name:  args.display_name,
            capabilities:  vec![WorkerCapability {
                framework:              Framework::Pytorch,
                mode:                   args.mode,
                device:                 args.device,
                accelerator_runtime:    args.accelerator_runtime,
                architecture_family:    args.architecture_family,
                available_memory_bytes: args.memory_bytes,
                concurrency_slots:      args.concurrency_slots,
            }],
            heartbeat_ttl: None,
        })
        .await?;

    let mut ticker = interval(HEARTBEAT_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        if stop_heartbeats.load(Ordering::Relaxed) {
            break;
        }

        ticker.tick().await;
        let response = client
            .heartbeat_worker(&args.worker_id, &WorkerHeartbeatRequest {
                state: WorkerState::Ready,
            })
            .await?;

        for assignment in response.assignments {
            let client = client.clone();
            let provider = provider.clone();
            let active_replicas = Arc::clone(&active_replicas);
            let stop_heartbeats = Arc::clone(&stop_heartbeats);
            tokio::spawn(async move {
                let _ = process_assignment(
                    client,
                    provider,
                    assignment,
                    active_replicas,
                    stop_heartbeats,
                )
                .await;
            });
        }
    }

    Ok(())
}

async fn process_assignment(
    client: DlpClient,
    provider: SimulatedProvider,
    assignment: WorkerAssignment,
    active_replicas: Arc<Mutex<HashMap<String, ReplicaState>>>,
    stop_heartbeats: Arc<AtomicBool>,
) -> Result<()> {
    {
        let mut guard = active_replicas.lock().await;
        guard.insert(assignment.replica_id.clone(), ReplicaState::Assigned);
    }

    for step in provider.planned_updates() {
        sleep(LIFECYCLE_STEP_DELAY).await;
        client
            .update_replica_status(&assignment.replica_id, &UpdateReplicaStatusRequest {
                state:          step.state.clone(),
                status_message: Some(step.message.clone()),
            })
            .await?;

        let mut guard = active_replicas.lock().await;
        match step.state {
            ReplicaState::Ready => {
                guard.insert(assignment.replica_id.clone(), ReplicaState::Ready);
                if provider.failure_mode == FailureMode::ExitAfterReady {
                    stop_heartbeats.store(true, Ordering::Relaxed);
                }
            }
            ReplicaState::Failed | ReplicaState::Stopped => {
                guard.remove(&assignment.replica_id);
            }
            other => {
                guard.insert(assignment.replica_id.clone(), other);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use client_sdk::ReplicaState;

    use super::{FailureMode, LifecycleStep, RuntimeProvider, SimulatedProvider};

    #[test]
    fn provider_plans_ready_lifecycle_by_default() {
        let provider = SimulatedProvider {
            failure_mode: FailureMode::None,
        };

        assert_eq!(provider.planned_updates(), vec![
            LifecycleStep::new(ReplicaState::Pulling, "pulling artifacts"),
            LifecycleStep::new(ReplicaState::Starting, "starting runtime"),
            LifecycleStep::new(ReplicaState::Ready, "ready"),
        ]);
    }

    #[test]
    fn provider_can_fail_before_ready() {
        let provider = SimulatedProvider {
            failure_mode: FailureMode::BeforeReady,
        };

        assert_eq!(
            provider
                .planned_updates()
                .last()
                .map(|step| step.state.clone()),
            Some(ReplicaState::Failed)
        );
    }
}
