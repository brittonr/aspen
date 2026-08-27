//! Pure system-extension manifest, lifecycle, dispatch, and supervision laws.
//!
//! This module contains no code loading, filesystem, network, clock, process,
//! environment, or persistence effects. The outer `molten` crate owns callback
//! invocation and canonical Preserves evidence.

mod dispatch;
mod lifecycle;
mod manifest;
mod native_host;
mod supervision;

#[cfg(test)]
mod tests;

pub use dispatch::*;
pub use lifecycle::*;
pub use manifest::*;
pub use native_host::*;
pub use supervision::*;

pub const SYSTEM_EXTENSION_MANIFEST_SCHEMA: &str = "molten.system-extension.manifest.v1";
pub const SYSTEM_EXTENSION_LIFECYCLE_SCHEMA: &str = "molten.system-extension.lifecycle.v1";
pub const SYSTEM_EXTENSION_CALLBACK_SCHEMA: &str = "molten.system-extension.callback.v1";
pub const SYSTEM_EXTENSION_EXECUTION_BINDING_SCHEMA: &str = "molten.system-extension.execution-binding.v1";
pub const SYSTEM_EXTENSION_TYPED_EFFECT_SCHEMA: &str = "molten.system-extension.typed-effect.v1";
pub const SYSTEM_EXTENSION_EFFECT_COMPLETION_SCHEMA: &str = "molten.system-extension.effect-completion.v1";
pub const SYSTEM_EXTENSION_STATE_MIGRATION_SCHEMA: &str = "molten.system-extension.state-migration.v1";
pub const SYSTEM_EXTENSION_READINESS_SCHEMA: &str = "molten.system-extension.readiness.v1";
pub const SYSTEM_EXTENSION_STATUS_SCHEMA: &str = "molten.system-extension.status.v1";

pub(crate) const MAX_SYSTEM_EXTENSION_ITEMS: usize = 128;
pub(crate) const INITIAL_SYSTEM_EXTENSION_GENERATION: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallbackKind {
    Initialize,
    Start,
    Request,
    Message,
    StreamOpen,
    StreamEvent,
    Timer,
    Health,
    Checkpoint,
    Recover,
    Drain,
    Shutdown,
}

impl CallbackKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Start => "start",
            Self::Request => "request",
            Self::Message => "message",
            Self::StreamOpen => "stream-open",
            Self::StreamEvent => "stream-event",
            Self::Timer => "timer",
            Self::Health => "health",
            Self::Checkpoint => "checkpoint",
            Self::Recover => "recover",
            Self::Drain => "drain",
            Self::Shutdown => "shutdown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "initialize" => Some(Self::Initialize),
            "start" => Some(Self::Start),
            "request" => Some(Self::Request),
            "message" => Some(Self::Message),
            "stream-open" => Some(Self::StreamOpen),
            "stream-event" => Some(Self::StreamEvent),
            "timer" => Some(Self::Timer),
            "health" => Some(Self::Health),
            "checkpoint" => Some(Self::Checkpoint),
            "recover" => Some(Self::Recover),
            "drain" => Some(Self::Drain),
            "shutdown" => Some(Self::Shutdown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionProfile {
    InProcessNative,
    NativeProcess,
    SandboxedComponent,
}

impl ExecutionProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InProcessNative => "in-process-native",
            Self::NativeProcess => "native-process",
            Self::SandboxedComponent => "sandboxed-component",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemExtensionNonClaim {
    InstallationIsNotActivation,
    ArtifactIsNotAuthority,
    CallbackSuccessIsNotConsensus,
    CallbackSuccessIsNotDurability,
    CallbackSuccessIsNotProtocolCompatibility,
    CallbackSuccessIsNotSemanticCorrectness,
}

impl SystemExtensionNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InstallationIsNotActivation => "installation-is-not-activation",
            Self::ArtifactIsNotAuthority => "artifact-is-not-authority",
            Self::CallbackSuccessIsNotConsensus => "callback-success-is-not-consensus-proof",
            Self::CallbackSuccessIsNotDurability => "callback-success-is-not-durability-proof",
            Self::CallbackSuccessIsNotProtocolCompatibility => "callback-success-is-not-protocol-compatibility-proof",
            Self::CallbackSuccessIsNotSemanticCorrectness => {
                "callback-success-is-not-extension-semantic-correctness-proof"
            }
        }
    }
}

const REQUIRED_NON_CLAIM_COUNT: usize = 6;

pub const REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS: [SystemExtensionNonClaim; REQUIRED_NON_CLAIM_COUNT] = [
    SystemExtensionNonClaim::InstallationIsNotActivation,
    SystemExtensionNonClaim::ArtifactIsNotAuthority,
    SystemExtensionNonClaim::CallbackSuccessIsNotConsensus,
    SystemExtensionNonClaim::CallbackSuccessIsNotDurability,
    SystemExtensionNonClaim::CallbackSuccessIsNotProtocolCompatibility,
    SystemExtensionNonClaim::CallbackSuccessIsNotSemanticCorrectness,
];

pub(crate) fn valid_token(value: &str) -> bool {
    crate::fabric::valid_fabric_token(value)
}

pub(crate) fn valid_ref(value: &str) -> bool {
    crate::fabric::valid_blake3_ref(value)
}

pub(crate) fn duplicates<T: Ord>(values: &[T]) -> bool {
    crate::fabric::has_duplicates(values)
}
