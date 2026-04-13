//! Shared CLI and REPL entrypoint for DLP.

mod args;
mod commands;
mod config;
mod render;
mod repl;

use anyhow::Result;
use clap::Parser;
use dlp_client::transport::DlpClient;
use tokio::io::{self, AsyncWriteExt as _};

use crate::{args::Args, commands::execute_command, config::resolve_config, repl::run_repl};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let command = args.command.clone();
    let client = DlpClient::new(resolve_config(&args)?.api.base_url());

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

#[cfg(test)]
mod tests {
    use clap::Parser as _;
    use control_plane as _;
    use dlp_api::{
        deployments::{DeploymentDto, DeploymentStatusSummaryDto},
        replicas::{ReplicaDto, ReplicaState},
        shared::{DeviceClass, Framework, WorkloadMode, WorkloadRequirementDto},
    };

    use super::{
        args::{Args, Command, InteractiveCommand, ReplicasCommand, SubmitDeploymentArgs},
        render::{format_deployment, format_replica},
        repl::parse_interactive_command,
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
            Some(Command::Deployments(
                super::args::DeploymentsCommand::Submit(SubmitDeploymentArgs { .. })
            ))
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
        let deployment = DeploymentDto {
            id:               "deployment-1".to_owned(),
            name:             "trainer".to_owned(),
            artifact_ref:     "artifact://model".to_owned(),
            replicas_desired: 1,
            requirement:      WorkloadRequirementDto {
                framework:                Framework::Pytorch,
                mode:                     WorkloadMode::Training,
                device:                   DeviceClass::Cpu,
                accelerator_runtime:      "cpu".to_owned(),
                architecture_family:      "generic".to_owned(),
                memory_requirement_bytes: 1024,
                concurrency_requirement:  1,
            },
            status:           DeploymentStatusSummaryDto {
                pending_replicas:  1,
                assigned_replicas: 0,
                pulling_replicas:  0,
                starting_replicas: 0,
                ready_replicas:    0,
                failed_replicas:   0,
                stopped_replicas:  0,
            },
        };

        assert!(format_deployment(&deployment).contains("pending=1"));
    }

    #[test]
    fn formats_replica_summary() {
        let replica = ReplicaDto {
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
