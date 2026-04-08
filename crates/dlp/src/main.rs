//! Shared CLI and REPL entrypoint for DLP.

use anyhow::{Result, bail};
use app_config::{DlpConfig, load_dlp_config};
use clap::{Args as ClapArgs, Parser, Subcommand};
use client_sdk::{
    CreateDeploymentRequest, DeviceClass, DlpClient, Framework, ModelDeployment, ModelReplica,
    WorkloadMode, WorkloadRequirement,
};
#[cfg(test)]
use control_plane as _;
use tokio::io::{self, AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

#[derive(Debug, Parser)]
#[command(name = "dlp", about = "DLP client with shared CLI and REPL")]
struct Args {
    #[arg(long, global = true)]
    api_host: Option<String>,

    #[arg(long, global = true)]
    api_port: Option<u16>,

    #[arg(long, global = true)]
    api_scheme: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    #[command(subcommand)]
    Deployments(DeploymentsCommand),
    Health,
    #[command(subcommand)]
    Replicas(ReplicasCommand),
    #[command(subcommand)]
    Workers(WorkersCommand),
}

#[derive(Debug, Clone, Subcommand)]
enum WorkersCommand {
    List,
}

#[derive(Debug, Clone, Subcommand)]
enum DeploymentsCommand {
    Get(GetDeploymentArgs),
    Submit(SubmitDeploymentArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum ReplicasCommand {
    List(ListReplicasArgs),
}

#[derive(Debug, Clone, ClapArgs)]
struct SubmitDeploymentArgs {
    #[arg(long)]
    accelerator_runtime: String,

    #[arg(long)]
    architecture_family: String,

    #[arg(long)]
    artifact_ref: String,

    #[arg(long)]
    concurrency: u32,

    #[arg(long)]
    device: DeviceClass,

    #[arg(long)]
    framework: Framework,

    #[arg(long)]
    memory_bytes: u64,

    #[arg(long)]
    mode: WorkloadMode,

    #[arg(long)]
    name: String,

    #[arg(long)]
    replicas: u32,
}

#[derive(Debug, Clone, ClapArgs)]
struct GetDeploymentArgs {
    deployment_id: String,
}

#[derive(Debug, Clone, ClapArgs)]
struct ListReplicasArgs {
    #[arg(long)]
    deployment_id: Option<String>,
}

#[derive(Debug, Clone)]
enum InteractiveCommand {
    Exit,
    Health,
    Help,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let command = args.command.clone();
    let client = DlpClient::new(resolve_config(args)?.api.base_url());

    match command {
        Some(parsed_command) => {
            let output = execute_command(parsed_command, &client).await?;
            let mut stdout = io::stdout();
            stdout.write_all(output.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
        }
        None => run_repl(client).await?,
    }

    Ok(())
}

fn resolve_config(args: Args) -> Result<DlpConfig> {
    let mut config = load_dlp_config()?;

    if let Some(api_scheme) = args.api_scheme {
        config.api.scheme = api_scheme;
    }
    if let Some(api_host) = args.api_host {
        config.api.host = api_host;
    }
    if let Some(api_port) = args.api_port {
        config.api.port = api_port;
    }

    Ok(config)
}

async fn execute_command(command: Command, client: &DlpClient) -> Result<String> {
    match command {
        Command::Health => {
            let health = client.health_check().await?;
            Ok(format!("{}: {}", health.service, health.status))
        }
        Command::Workers(WorkersCommand::List) => {
            let response = client.list_workers().await?;
            if response.workers.is_empty() {
                return Ok("No workers registered.".to_owned());
            }

            Ok(response
                .workers
                .into_iter()
                .map(|worker| {
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
                        worker.state.to_string().to_ascii_lowercase(),
                        worker.assigned_replicas,
                        worker.available_slots,
                        capabilities
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        Command::Deployments(DeploymentsCommand::Submit(args)) => {
            let response = client
                .create_deployment(&CreateDeploymentRequest {
                    name:             args.name,
                    artifact_ref:     args.artifact_ref,
                    replicas_desired: args.replicas,
                    requirement:      WorkloadRequirement {
                        framework:                args.framework,
                        mode:                     args.mode,
                        device:                   args.device,
                        accelerator_runtime:      args.accelerator_runtime,
                        architecture_family:      args.architecture_family,
                        memory_requirement_bytes: args.memory_bytes,
                        concurrency_requirement:  args.concurrency,
                    },
                })
                .await?;

            Ok(format_deployment(&response.deployment))
        }
        Command::Deployments(DeploymentsCommand::Get(args)) => {
            let response = client.get_deployment(&args.deployment_id).await?;
            let mut lines = vec![format_deployment(&response.deployment)];
            if response.replicas.is_empty() {
                lines.push("Replicas: none".to_owned());
            } else {
                lines.push("Replicas:".to_owned());
                lines.extend(response.replicas.into_iter().map(format_replica));
            }

            Ok(lines.join("\n"))
        }
        Command::Replicas(ReplicasCommand::List(args)) => {
            let response = client.list_replicas(args.deployment_id.as_deref()).await?;
            if response.replicas.is_empty() {
                return Ok("No replicas found.".to_owned());
            }

            Ok(response
                .replicas
                .into_iter()
                .map(format_replica)
                .collect::<Vec<_>>()
                .join("\n"))
        }
    }
}

fn format_deployment(deployment: &ModelDeployment) -> String {
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

fn format_replica(replica: ModelReplica) -> String {
    let worker = replica
        .worker_id
        .unwrap_or_else(|| "unassigned".to_owned());
    let lease = replica.lease_id.unwrap_or_else(|| "none".to_owned());
    let message = replica
        .status_message
        .unwrap_or_else(|| "no status".to_owned());

    format!(
        "{} deployment={} state={} worker={} lease={} message={}",
        replica.id,
        replica.deployment_id,
        replica.state.to_string().to_ascii_lowercase(),
        worker,
        lease,
        message
    )
}

async fn run_repl(client: DlpClient) -> Result<()> {
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = io::stdout();

    stdout
        .write_all(b"DLP REPL. Type `help` for commands.\n")
        .await?;

    loop {
        stdout.write_all(b"dlp> ").await?;
        stdout.flush().await?;

        let Some(line) = lines.next_line().await? else {
            break;
        };

        match parse_interactive_command(&line) {
            Ok(InteractiveCommand::Health) => {
                let output = execute_command(Command::Health, &client).await?;
                stdout.write_all(output.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
            }
            Ok(InteractiveCommand::Help) => {
                stdout
                    .write_all(b"Commands: health, help, exit, quit\n")
                    .await?;
            }
            Ok(InteractiveCommand::Exit) => break,
            Err(error) => {
                stdout.write_all(error.to_string().as_bytes()).await?;
                stdout.write_all(b"\n").await?;
            }
        }
    }

    Ok(())
}

fn parse_interactive_command(input: &str) -> Result<InteractiveCommand> {
    match input.trim() {
        "" => bail!("enter a command"),
        "health" => Ok(InteractiveCommand::Health),
        "help" => Ok(InteractiveCommand::Help),
        "exit" | "quit" => Ok(InteractiveCommand::Exit),
        other => bail!("unknown command: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;
    use client_sdk::{
        DeploymentStatusSummary, Framework, ReplicaState, WorkloadMode, WorkloadRequirement,
    };

    use super::{
        Args, Command, DeploymentStatusSummary, DeviceClass, InteractiveCommand, ModelDeployment,
        ModelReplica, ReplicasCommand, SubmitDeploymentArgs, format_deployment, format_replica,
        parse_interactive_command,
    };

    #[test]
    fn parses_known_interactive_commands() {
        assert!(matches!(
            parse_interactive_command("health").ok(),
            Some(InteractiveCommand::Health)
        ));
        assert!(matches!(
            parse_interactive_command("help").ok(),
            Some(InteractiveCommand::Help)
        ));
        assert!(matches!(
            parse_interactive_command("quit").ok(),
            Some(InteractiveCommand::Exit)
        ));
    }

    #[test]
    fn rejects_unknown_interactive_commands() {
        let error = parse_interactive_command("workers");
        assert!(error.is_err());
        assert!(error
            .err()
            .is_some_and(|value| value.to_string().contains("unknown command")));
    }

    #[test]
    fn parses_deployment_submit_command() {
        let args = Args::try_parse_from([
            "dlp",
            "deployments",
            "submit",
            "--name",
            "trainer",
            "--artifact-ref",
            "artifact://model",
            "--replicas",
            "2",
            "--framework",
            "pytorch",
            "--mode",
            "training",
            "--device",
            "cpu",
            "--accelerator-runtime",
            "cpu",
            "--architecture-family",
            "generic",
            "--memory-bytes",
            "1024",
            "--concurrency",
            "1",
        ]);

        assert!(args.is_ok());
        let command = args.ok().and_then(|parsed| parsed.command);
        assert!(matches!(
            command,
            Some(Command::Deployments(super::DeploymentsCommand::Submit(
                SubmitDeploymentArgs { .. }
            )))
        ));
    }

    #[test]
    fn parses_replica_list_command_with_filter() {
        let args =
            Args::try_parse_from(["dlp", "replicas", "list", "--deployment-id", "deployment-1"]);

        assert!(args.is_ok());
        let command = args.ok().and_then(|parsed| parsed.command);
        assert!(matches!(
            command,
            Some(Command::Replicas(ReplicasCommand::List(_)))
        ));
    }

    #[test]
    fn formats_deployment_summary() {
        let deployment = ModelDeployment {
            id:               "deployment-1".to_owned(),
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      WorkloadRequirement {
                framework:                Framework::Pytorch,
                mode:                     WorkloadMode::Training,
                device:                   DeviceClass::Cpu,
                accelerator_runtime:      "cpu".to_owned(),
                architecture_family:      "generic".to_owned(),
                memory_requirement_bytes: 1024,
                concurrency_requirement:  1,
            },
            status:           DeploymentStatusSummary {
                pending_replicas: 1,
                ..DeploymentStatusSummary::default()
            },
        };

        assert!(format_deployment(&deployment).contains("pending=1"));
    }

    #[test]
    fn formats_replica_summary() {
        let replica = ModelReplica {
            id:             "replica-1".to_owned(),
            deployment_id:  "deployment-1".to_owned(),
            worker_id:      Some("worker-1".to_owned()),
            lease_id:       Some("lease-1".to_owned()),
            state:          ReplicaState::Ready,
            status_message: Some("ready".to_owned()),
        };

        let formatted = format_replica(replica);
        assert!(formatted.contains("state=ready"));
        assert!(formatted.contains("worker=worker-1"));
    }
}
