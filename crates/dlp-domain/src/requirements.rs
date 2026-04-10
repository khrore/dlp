//! Workload requirements and worker capabilities.

use std::fmt;

use strum::{Display, EnumString};

use crate::errors::{DomainError, DomainResult};

macro_rules! domain_string {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> DomainResult<Self> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(DomainError::EmptyValue($field));
                }

                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum Framework {
    Max,
    Pytorch,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum WorkloadMode {
    Inference,
    Training,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case", ascii_case_insensitive)]
pub enum DeviceClass {
    AppleGpu,
    Cpu,
    Cuda,
    Rocm,
}

domain_string!(RuntimeName, "accelerator_runtime");
domain_string!(ArchitectureFamily, "architecture_family");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCapability {
    accelerator_runtime:    RuntimeName,
    architecture_family:    ArchitectureFamily,
    available_memory_bytes: u64,
    concurrency_slots:      u32,
    device:                 DeviceClass,
    framework:              Framework,
    mode:                   WorkloadMode,
}

impl WorkerCapability {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        framework: Framework,
        mode: WorkloadMode,
        device: DeviceClass,
        accelerator_runtime: RuntimeName,
        architecture_family: ArchitectureFamily,
        available_memory_bytes: u64,
        concurrency_slots: u32,
    ) -> Self {
        Self {
            accelerator_runtime,
            architecture_family,
            available_memory_bytes,
            concurrency_slots,
            device,
            framework,
            mode,
        }
    }

    #[must_use]
    pub fn accelerator_runtime(&self) -> &RuntimeName {
        &self.accelerator_runtime
    }

    #[must_use]
    pub fn architecture_family(&self) -> &ArchitectureFamily {
        &self.architecture_family
    }

    #[must_use]
    pub const fn available_memory_bytes(&self) -> u64 {
        self.available_memory_bytes
    }

    #[must_use]
    pub const fn concurrency_slots(&self) -> u32 {
        self.concurrency_slots
    }

    #[must_use]
    pub fn device(&self) -> &DeviceClass {
        &self.device
    }

    #[must_use]
    pub fn framework(&self) -> &Framework {
        &self.framework
    }

    #[must_use]
    pub fn mode(&self) -> &WorkloadMode {
        &self.mode
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadRequirement {
    accelerator_runtime:      RuntimeName,
    architecture_family:      ArchitectureFamily,
    concurrency_requirement:  u32,
    device:                   DeviceClass,
    framework:                Framework,
    memory_requirement_bytes: u64,
    mode:                     WorkloadMode,
}

impl WorkloadRequirement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        framework: Framework,
        mode: WorkloadMode,
        device: DeviceClass,
        accelerator_runtime: RuntimeName,
        architecture_family: ArchitectureFamily,
        memory_requirement_bytes: u64,
        concurrency_requirement: u32,
    ) -> Self {
        Self {
            accelerator_runtime,
            architecture_family,
            concurrency_requirement,
            device,
            framework,
            memory_requirement_bytes,
            mode,
        }
    }

    #[must_use]
    pub fn accelerator_runtime(&self) -> &RuntimeName {
        &self.accelerator_runtime
    }

    #[must_use]
    pub fn architecture_family(&self) -> &ArchitectureFamily {
        &self.architecture_family
    }

    #[must_use]
    pub const fn concurrency_requirement(&self) -> u32 {
        self.concurrency_requirement
    }

    #[must_use]
    pub fn device(&self) -> &DeviceClass {
        &self.device
    }

    #[must_use]
    pub fn framework(&self) -> &Framework {
        &self.framework
    }

    #[must_use]
    pub const fn memory_requirement_bytes(&self) -> u64 {
        self.memory_requirement_bytes
    }

    #[must_use]
    pub fn mode(&self) -> &WorkloadMode {
        &self.mode
    }
}
