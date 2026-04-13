//! Shared library API for the DLP CLI and REPL client.

mod args;
mod commands;
mod config;
mod render;
mod repl;

pub use self::{
    args::{
        Args, Command, DeploymentsCommand, GetDeploymentArgs, InteractiveCommand, ListReplicasArgs,
        ReplicasCommand, SubmitDeploymentArgs, WorkersCommand,
    },
    commands::execute_command,
    config::resolve_config,
    render::{format_deployment, format_replica, format_worker},
    repl::{parse_interactive_command, run_repl},
};

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
        Args, Command, DeploymentsCommand, InteractiveCommand, ReplicasCommand,
        SubmitDeploymentArgs, format_deployment, format_replica, parse_interactive_command,
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
        parse_interactive_command("workers").unwrap_err();
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
            Some(Command::Deployments(DeploymentsCommand::Submit(
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
