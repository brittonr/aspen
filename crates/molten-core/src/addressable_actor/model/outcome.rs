use serde::Deserialize;
use serde::Serialize;

use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActorDecision {
    Applied,
    DuplicateReplay,
    Denied,
    Unknown,
}

impl ActorDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::DuplicateReplay => "duplicate-replay",
            Self::Denied => "denied",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActorTransitionKind {
    WakeStart,
    WakeDispatch,
    StartComplete,
    Sleep,
    DrainBegin,
    DrainComplete,
    Stop,
    Degrade,
    RecoveryBegin,
    RecoveryComplete,
    RecoveryFailed,
    DeliveryComplete,
    UnknownEffect,
    UnknownEffectResolved,
    DeniedPreserve,
    DuplicatePreserve,
}

impl ActorTransitionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WakeStart => "wake-start",
            Self::WakeDispatch => "wake-dispatch",
            Self::StartComplete => "start-complete",
            Self::Sleep => "sleep",
            Self::DrainBegin => "drain-begin",
            Self::DrainComplete => "drain-complete",
            Self::Stop => "stop",
            Self::Degrade => "degrade",
            Self::RecoveryBegin => "recovery-begin",
            Self::RecoveryComplete => "recovery-complete",
            Self::RecoveryFailed => "recovery-failed",
            Self::DeliveryComplete => "delivery-complete",
            Self::UnknownEffect => "unknown-effect",
            Self::UnknownEffectResolved => "unknown-effect-resolved",
            Self::DeniedPreserve => "denied-preserve",
            Self::DuplicatePreserve => "duplicate-preserve",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActorEffectIntentKind {
    RestoreCheckpoint,
    StartRuntime,
    DeliverMessage,
    InvokeTimer,
    AcceptConnection,
    NotifyOperator,
    PersistCheckpoint,
    StopRuntime,
    AcknowledgeDelivery,
    PublishStatus,
}

impl ActorEffectIntentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RestoreCheckpoint => "restore-checkpoint",
            Self::StartRuntime => "start-runtime",
            Self::DeliverMessage => "deliver-message",
            Self::InvokeTimer => "invoke-timer",
            Self::AcceptConnection => "accept-connection",
            Self::NotifyOperator => "notify-operator",
            Self::PersistCheckpoint => "persist-checkpoint",
            Self::StopRuntime => "stop-runtime",
            Self::AcknowledgeDelivery => "acknowledge-delivery",
            Self::PublishStatus => "publish-status",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorEffectIntent {
    pub schema: String,
    pub effect_ref: String,
    pub request_ref: String,
    pub kind: ActorEffectIntentKind,
    pub actor_key_ref: String,
    pub profile_ref: String,
    pub system_extension_manifest_ref: String,
    pub placement_ref: String,
    pub extension_generation: u64,
    pub lifecycle_sequence: u64,
    pub wake_ref: Option<String>,
    pub subject_ref: Option<String>,
    pub requires_fresh_admission: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorTransition {
    pub schema: String,
    pub decision: ActorDecision,
    pub kind: ActorTransitionKind,
    pub request_ref: String,
    pub operation_ref: String,
    pub before_state_ref: String,
    pub after_state_ref: String,
    pub next_state: ActorState,
    pub effects: Vec<ActorEffectIntent>,
    pub restored_classes: Vec<SurvivalClass>,
    pub issue: Option<ActorIssue>,
    pub effects_require_fresh_admission: bool,
    pub external_effect_retry_authorized: bool,
    pub receipt_authority: bool,
}
