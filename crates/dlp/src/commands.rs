use dlp_api::{deployments::CreateDeploymentRequest, shared::WorkloadRequirementDto};
use dlp_client::{DeploymentsClient as _, DlpClient, ReplicasClient as _, WorkersClient as _};

use crate::{
    Result,
    args::{Command, DeploymentsCommand, ReplicasCommand, WorkersCommand},
    render::{format_deployment, format_replica, format_worker},
};

/// Executes a parsed CLI command and returns formatted terminal output.
///
/// # Errors
///
/// Returns an error when the underlying client request fails.
pub async fn execute_command(command: Command, client: &DlpClient) -> Result<String> {
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
                .map(format_worker)
                .collect::<Vec<_>>()
                .join("\n"))
        }
        Command::Deployments(DeploymentsCommand::Submit(args)) => {
            let response = client
                .create_deployment(&CreateDeploymentRequest {
                    name:             args.name,
                    artifact_ref:     args.artifact_ref,
                    replicas_desired: args.replicas,
                    requirement:      WorkloadRequirementDto {
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
