use clap::{Args as ClapArgs, Parser, Subcommand};
use dlp_api::shared::{DeviceClass, Framework, WorkloadMode};

/// Top-level command-line arguments for the DLP client.
#[derive(Debug, Parser)]
#[command(name = "dlp", about = "DLP client with shared CLI and REPL")]
pub struct Args {
    /// Overrides the configured API host.
    #[arg(long, global = true)]
    pub api_host: Option<String>,

    /// Overrides the configured API port.
    #[arg(long, global = true)]
    pub api_port: Option<u16>,

    /// Overrides the configured API scheme.
    #[arg(long, global = true)]
    pub api_scheme: Option<String>,

    /// Selects the non-interactive command to execute.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Supported top-level DLP commands.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Manages deployments.
    #[command(subcommand)]
    Deployments(DeploymentsCommand),
    /// Calls the health endpoint.
    Health,
    /// Lists replicas.
    #[command(subcommand)]
    Replicas(ReplicasCommand),
    /// Manages workers.
    #[command(subcommand)]
    Workers(WorkersCommand),
}

/// Worker-related commands.
#[derive(Debug, Clone, Subcommand)]
pub enum WorkersCommand {
    /// Lists known workers.
    List,
}

/// Deployment-related commands.
#[derive(Debug, Clone, Subcommand)]
pub enum DeploymentsCommand {
    /// Fetches one deployment by id.
    Get(GetDeploymentArgs),
    /// Creates a deployment.
    Submit(SubmitDeploymentArgs),
}

/// Replica-related commands.
#[derive(Debug, Clone, Subcommand)]
pub enum ReplicasCommand {
    /// Lists replicas, optionally filtered by deployment.
    List(ListReplicasArgs),
}

/// Arguments for the `deployments submit` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct SubmitDeploymentArgs {
    /// Accelerator runtime name.
    #[arg(long)]
    pub accelerator_runtime: String,

    /// Architecture family name.
    #[arg(long)]
    pub architecture_family: String,

    /// Artifact reference for the deployment payload.
    #[arg(long)]
    pub artifact_ref: String,

    /// Required concurrency slots per replica.
    #[arg(long)]
    pub concurrency: u32,

    /// Required device class.
    #[arg(long)]
    pub device: DeviceClass,

    /// Required framework.
    #[arg(long)]
    pub framework: Framework,

    /// Required memory in bytes per replica.
    #[arg(long)]
    pub memory_bytes: u64,

    /// Required workload mode.
    #[arg(long)]
    pub mode: WorkloadMode,

    /// Deployment display name.
    #[arg(long)]
    pub name: String,

    /// Desired replica count.
    #[arg(long)]
    pub replicas: u32,
}

/// Arguments for the `deployments get` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct GetDeploymentArgs {
    /// Deployment identifier.
    pub deployment_id: String,
}

/// Arguments for the `replicas list` command.
#[derive(Debug, Clone, ClapArgs)]
pub struct ListReplicasArgs {
    /// Optional deployment identifier filter.
    #[arg(long)]
    pub deployment_id: Option<String>,
}

/// Supported REPL commands.
#[derive(Debug, Clone)]
pub enum InteractiveCommand {
    /// Exits the REPL.
    Exit,
    /// Calls the health endpoint.
    Health,
    /// Prints the help summary.
    Help,
}
