use crate::fabric::FabricNonClaim;

// r[impl molten.fabric_execution.simulation]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionProfileKind {
    LiveBoundedProcess,
    DeterministicSimulation,
}

impl ExecutionProfileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LiveBoundedProcess => "live-bounded-process",
            Self::DeterministicSimulation => "deterministic-simulation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionPlatform {
    UnixProcessGroup,
    DirectChildOnly,
}

impl ExecutionPlatform {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnixProcessGroup => "unix-process-group",
            Self::DirectChildOnly => "direct-child-only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionEnvironmentMode {
    Clear,
    InheritRequested,
}

impl ExecutionEnvironmentMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::InheritRequested => "inherit-requested",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionInvocationMode {
    Direct,
    ShellExpansion,
}

impl ExecutionInvocationMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::ShellExpansion => "shell-expansion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutableResolutionMode {
    ExactArtifact,
    PathSearch,
}

impl ExecutableResolutionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExactArtifact => "exact-artifact",
            Self::PathSearch => "path-search",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkspaceMode {
    CapabilityRoot,
    ImplicitCurrentDirectory,
}

impl WorkspaceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityRoot => "capability-root",
            Self::ImplicitCurrentDirectory => "implicit-current-directory",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnvironmentValueClass {
    Public,
    Secret,
}

impl EnvironmentValueClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Secret => "secret",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionTerminationScope {
    DirectChild,
    ProcessGroup,
}

impl ExecutionTerminationScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DirectChild => "direct-child",
            Self::ProcessGroup => "process-group",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionNonClaim {
    Sandboxing,
    Hermeticity,
    ExecutableTrust,
    ChildCorrectness,
    NetworkIsolation,
    PlatformEquivalence,
    ApplicationSuccess,
    ReleaseReadiness,
}

impl ExecutionNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sandboxing => "does-not-prove-sandboxing",
            Self::Hermeticity => "does-not-prove-hermeticity",
            Self::ExecutableTrust => "does-not-prove-executable-trust",
            Self::ChildCorrectness => "does-not-prove-child-correctness",
            Self::NetworkIsolation => "does-not-prove-network-isolation",
            Self::PlatformEquivalence => "does-not-prove-platform-equivalence",
            Self::ApplicationSuccess => "does-not-prove-application-success",
            Self::ReleaseReadiness => "does-not-prove-release-readiness",
        }
    }
}

const REQUIRED_EXECUTION_NON_CLAIM_COUNT: usize = 8;

pub const REQUIRED_EXECUTION_NON_CLAIMS: [ExecutionNonClaim; REQUIRED_EXECUTION_NON_CLAIM_COUNT] = [
    ExecutionNonClaim::Sandboxing,
    ExecutionNonClaim::Hermeticity,
    ExecutionNonClaim::ExecutableTrust,
    ExecutionNonClaim::ChildCorrectness,
    ExecutionNonClaim::NetworkIsolation,
    ExecutionNonClaim::PlatformEquivalence,
    ExecutionNonClaim::ApplicationSuccess,
    ExecutionNonClaim::ReleaseReadiness,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionProfileDescriptor {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub kind: ExecutionProfileKind,
    pub platform: ExecutionPlatform,
    pub supported_termination_scopes: Vec<ExecutionTerminationScope>,
    pub max_timeout_ms: u64,
    pub max_stdin_bytes: u64,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
    pub max_poll_interval_ms: u64,
    pub max_teardown_timeout_ms: u64,
    pub max_arguments: usize,
    pub max_argument_bytes: usize,
    pub max_environment_entries: usize,
    pub max_environment_name_bytes: usize,
    pub max_environment_value_bytes: usize,
    pub max_concurrency_units: u64,
    pub max_queue_units: u64,
    pub component_repository: String,
    pub component_revision: String,
    pub component_license: String,
    pub component_package: String,
    pub conformance_refs: Vec<String>,
    pub fabric_non_claims: Vec<FabricNonClaim>,
    pub non_claims: Vec<ExecutionNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedExecutionProfile {
    pub descriptor: ExecutionProfileDescriptor,
}
