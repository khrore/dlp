use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Runtime framework requested by a deployment or offered by a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum Framework {
    /// Apple MAX runtime support.
    Max,
    /// `PyTorch` runtime support.
    Pytorch,
}

/// Workload mode requested by a deployment or offered by a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum WorkloadMode {
    /// Inference-oriented workloads.
    Inference,
    /// Training-oriented workloads.
    Training,
}

/// Device class requested by a deployment or offered by a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "kebab-case", ascii_case_insensitive)]
pub enum DeviceClass {
    /// Apple GPU-backed execution.
    AppleGpu,
    /// CPU-only execution.
    Cpu,
    /// NVIDIA CUDA execution.
    Cuda,
    /// AMD `ROCm` execution.
    Rocm,
}

/// Scheduling requirements for a workload or worker capability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkloadRequirementDto {
    /// Accelerator runtime name, such as `cpu` or `cuda`.
    pub accelerator_runtime:      String,
    /// Architecture family identifier.
    pub architecture_family:      String,
    /// Required parallelism or slot count.
    pub concurrency_requirement:  u32,
    /// Requested device class.
    pub device:                   DeviceClass,
    /// Requested framework.
    pub framework:                Framework,
    /// Required memory in bytes.
    pub memory_requirement_bytes: u64,
    /// Requested workload mode.
    pub mode:                     WorkloadMode,
}
