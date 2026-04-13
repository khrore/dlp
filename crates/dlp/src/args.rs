use clap::{Args as ClapArgs, Parser, Subcommand};
use dlp_api::shared::{DeviceClass, Framework, WorkloadMode};

#[derive(Debug, Parser)]
#[command(name = "dlp", about = "DLP client with shared CLI and REPL")]
pub(crate) struct Args {
    #[arg(long, global = true)]
    pub(crate) api_host: Option<String>,

    #[arg(long, global = true)]
    pub(crate) api_port: Option<u16>,

    #[arg(long, global = true)]
    pub(crate) api_scheme: Option<String>,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum Command {
    #[command(subcommand)]
    Deployments(DeploymentsCommand),
    Health,
    #[command(subcommand)]
    Replicas(ReplicasCommand),
    #[command(subcommand)]
    Workers(WorkersCommand),
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum WorkersCommand {
    List,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum DeploymentsCommand {
    Get(GetDeploymentArgs),
    Submit(SubmitDeploymentArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum ReplicasCommand {
    List(ListReplicasArgs),
}

#[derive(Debug, Clone, ClapArgs)]
pub(crate) struct SubmitDeploymentArgs {
    #[arg(long)]
    pub(crate) accelerator_runtime: String,

    #[arg(long)]
    pub(crate) architecture_family: String,

    #[arg(long)]
    pub(crate) artifact_ref: String,

    #[arg(long)]
    pub(crate) concurrency: u32,

    #[arg(long)]
    pub(crate) device: DeviceClass,

    #[arg(long)]
    pub(crate) framework: Framework,

    #[arg(long)]
    pub(crate) memory_bytes: u64,

    #[arg(long)]
    pub(crate) mode: WorkloadMode,

    #[arg(long)]
    pub(crate) name: String,

    #[arg(long)]
    pub(crate) replicas: u32,
}

#[derive(Debug, Clone, ClapArgs)]
pub(crate) struct GetDeploymentArgs {
    pub(crate) deployment_id: String,
}

#[derive(Debug, Clone, ClapArgs)]
pub(crate) struct ListReplicasArgs {
    #[arg(long)]
    pub(crate) deployment_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum InteractiveCommand {
    Exit,
    Health,
    Help,
}
