use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum Framework {
    Max,
    Pytorch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum WorkloadMode {
    Inference,
    Training,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "kebab-case", ascii_case_insensitive)]
pub enum DeviceClass {
    AppleGpu,
    Cpu,
    Cuda,
    Rocm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkloadRequirementDto {
    pub accelerator_runtime:      String,
    pub architecture_family:      String,
    pub concurrency_requirement:  u32,
    pub device:                   DeviceClass,
    pub framework:                Framework,
    pub memory_requirement_bytes: u64,
    pub mode:                     WorkloadMode,
}
