//! Workload requirements and worker capabilities.

use std::fmt;

use strum::{Display, EnumString};

use crate::errors::{DomainError, DomainResult};

macro_rules! domain_string {
    ($name:ident, $field:literal) => {
        #[doc = concat!("Validated string value for `", $field, "`.")]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Validates and stores a `", $field, "` value.")]
            ///
            /// # Errors
            ///
            /// Returns [`DomainError::EmptyValue`] when the provided value is blank.
            pub fn new<Value>(value: Value) -> DomainResult<Self>
            where
                Value: Into<String>,
            {
                let validated_value = value.into();
                if validated_value.trim().is_empty() {
                    return Err(DomainError::EmptyValue($field));
                }

                Ok(Self(validated_value))
            }

            #[doc = concat!("Returns the raw `", $field, "` string.")]
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
/// Deep-learning framework supported by the scheduler.
pub enum Framework {
    /// Max-specific workloads.
    Max,
    /// `PyTorch` workloads.
    Pytorch,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
/// Execution mode for a workload.
pub enum WorkloadMode {
    /// Latency-oriented inference workloads.
    Inference,
    /// Training or fine-tuning workloads.
    Training,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case", ascii_case_insensitive)]
/// Hardware device family required by a workload or provided by a worker.
pub enum DeviceClass {
    /// Apple GPU execution.
    AppleGpu,
    /// CPU execution.
    Cpu,
    /// NVIDIA CUDA execution.
    Cuda,
    /// AMD `ROCm` execution.
    Rocm,
}

domain_string!(RuntimeName, "accelerator_runtime");
domain_string!(ArchitectureFamily, "architecture_family");

/// Shared workload dimensions that must match between a worker and a workload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadProfile {
    accelerator_runtime: RuntimeName,
    architecture_family: ArchitectureFamily,
    device: DeviceClass,
    framework: Framework,
    mode: WorkloadMode,
}

impl WorkloadProfile {
    /// Creates a workload profile describing the runtime, architecture, and framework.
    #[must_use]
    pub const fn new(
        framework: Framework,
        mode: WorkloadMode,
        device: DeviceClass,
        accelerator_runtime: RuntimeName,
        architecture_family: ArchitectureFamily,
    ) -> Self {
        Self {
            accelerator_runtime,
            architecture_family,
            device,
            framework,
            mode,
        }
    }

    /// Returns the accelerator runtime name.
    #[must_use]
    pub const fn accelerator_runtime(&self) -> &RuntimeName {
        &self.accelerator_runtime
    }

    /// Returns the architecture family name.
    #[must_use]
    pub const fn architecture_family(&self) -> &ArchitectureFamily {
        &self.architecture_family
    }

    /// Returns the required device class.
    #[must_use]
    pub const fn device(&self) -> &DeviceClass {
        &self.device
    }

    /// Returns the workload framework.
    #[must_use]
    pub const fn framework(&self) -> &Framework {
        &self.framework
    }

    /// Returns the workload mode.
    #[must_use]
    pub const fn mode(&self) -> &WorkloadMode {
        &self.mode
    }
}

/// Constructor input for a [`WorkerCapability`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCapabilitySpec {
    available_memory_bytes: u64,
    concurrency_slots: u32,
    profile: WorkloadProfile,
}

impl WorkerCapabilitySpec {
    /// Creates a worker capability specification.
    #[must_use]
    pub const fn new(
        profile: WorkloadProfile,
        available_memory_bytes: u64,
        concurrency_slots: u32,
    ) -> Self {
        Self {
            available_memory_bytes,
            concurrency_slots,
            profile,
        }
    }

    /// Returns the worker's schedulable memory.
    #[must_use]
    pub const fn available_memory_bytes(&self) -> u64 {
        self.available_memory_bytes
    }

    /// Returns the worker's concurrency capacity.
    #[must_use]
    pub const fn concurrency_slots(&self) -> u32 {
        self.concurrency_slots
    }

    /// Returns the common workload profile.
    #[must_use]
    pub const fn profile(&self) -> &WorkloadProfile {
        &self.profile
    }
}

/// Constructor input for a [`WorkloadRequirement`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadRequirementSpec {
    concurrency_requirement: u32,
    memory_requirement_bytes: u64,
    profile: WorkloadProfile,
}

impl WorkloadRequirementSpec {
    /// Creates a workload requirement specification.
    #[must_use]
    pub const fn new(
        profile: WorkloadProfile,
        memory_requirement_bytes: u64,
        concurrency_requirement: u32,
    ) -> Self {
        Self {
            concurrency_requirement,
            memory_requirement_bytes,
            profile,
        }
    }

    /// Returns the required concurrency slots.
    #[must_use]
    pub const fn concurrency_requirement(&self) -> u32 {
        self.concurrency_requirement
    }

    /// Returns the required memory budget.
    #[must_use]
    pub const fn memory_requirement_bytes(&self) -> u64 {
        self.memory_requirement_bytes
    }

    /// Returns the common workload profile.
    #[must_use]
    pub const fn profile(&self) -> &WorkloadProfile {
        &self.profile
    }
}

/// Capability advertised by a worker for a specific workload profile.
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
    /// Creates a worker capability from an explicit specification.
    #[must_use]
    pub fn new(spec: WorkerCapabilitySpec) -> Self {
        Self {
            accelerator_runtime: spec.profile.accelerator_runtime,
            architecture_family: spec.profile.architecture_family,
            available_memory_bytes: spec.available_memory_bytes,
            concurrency_slots: spec.concurrency_slots,
            device: spec.profile.device,
            framework: spec.profile.framework,
            mode: spec.profile.mode,
        }
    }

    /// Returns the accelerator runtime name.
    #[must_use]
    pub const fn accelerator_runtime(&self) -> &RuntimeName {
        &self.accelerator_runtime
    }

    /// Returns the architecture family.
    #[must_use]
    pub const fn architecture_family(&self) -> &ArchitectureFamily {
        &self.architecture_family
    }

    /// Returns the schedulable memory budget.
    #[must_use]
    pub const fn available_memory_bytes(&self) -> u64 {
        self.available_memory_bytes
    }

    /// Returns the total concurrency slots exposed by the worker.
    #[must_use]
    pub const fn concurrency_slots(&self) -> u32 {
        self.concurrency_slots
    }

    /// Returns the device class.
    #[must_use]
    pub const fn device(&self) -> &DeviceClass {
        &self.device
    }

    /// Returns the supported framework.
    #[must_use]
    pub const fn framework(&self) -> &Framework {
        &self.framework
    }

    /// Returns the supported workload mode.
    #[must_use]
    pub const fn mode(&self) -> &WorkloadMode {
        &self.mode
    }
}

/// Resource requirement for a deployment workload.
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
    /// Creates a workload requirement from an explicit specification.
    #[must_use]
    pub fn new(spec: WorkloadRequirementSpec) -> Self {
        Self {
            accelerator_runtime: spec.profile.accelerator_runtime,
            architecture_family: spec.profile.architecture_family,
            concurrency_requirement: spec.concurrency_requirement,
            device: spec.profile.device,
            framework: spec.profile.framework,
            memory_requirement_bytes: spec.memory_requirement_bytes,
            mode: spec.profile.mode,
        }
    }

    /// Returns the accelerator runtime name.
    #[must_use]
    pub const fn accelerator_runtime(&self) -> &RuntimeName {
        &self.accelerator_runtime
    }

    /// Returns the architecture family.
    #[must_use]
    pub const fn architecture_family(&self) -> &ArchitectureFamily {
        &self.architecture_family
    }

    /// Returns the required concurrency slots.
    #[must_use]
    pub const fn concurrency_requirement(&self) -> u32 {
        self.concurrency_requirement
    }

    /// Returns the required device class.
    #[must_use]
    pub const fn device(&self) -> &DeviceClass {
        &self.device
    }

    /// Returns the workload framework.
    #[must_use]
    pub const fn framework(&self) -> &Framework {
        &self.framework
    }

    /// Returns the required memory budget.
    #[must_use]
    pub const fn memory_requirement_bytes(&self) -> u64 {
        self.memory_requirement_bytes
    }

    /// Returns the workload mode.
    #[must_use]
    pub const fn mode(&self) -> &WorkloadMode {
        &self.mode
    }
}
