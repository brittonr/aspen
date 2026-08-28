use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActorPhase {
    Dormant,
    Starting,
    Running,
    Draining,
    Stopped,
    Degraded,
    Recovering,
}

impl ActorPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dormant => "dormant",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Draining => "draining",
            Self::Stopped => "stopped",
            Self::Degraded => "degraded",
            Self::Recovering => "recovering",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedActorOperation {
    pub request_ref: String,
    pub operation_ref: String,
    pub operation_kind: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorState {
    pub schema: String,
    pub actor_key_ref: String,
    pub profile_ref: String,
    pub system_extension_manifest_ref: String,
    pub placement_ref: String,
    pub extension_generation: u64,
    pub lifecycle_sequence: u64,
    pub revision: u64,
    pub phase: ActorPhase,
    pub checkpoint_ref: Option<String>,
    pub durable_state_ref: Option<String>,
    pub active_wake_ref: Option<String>,
    pub unknown_effect_ref: Option<String>,
    pub mailbox_revision: u64,
    pub last_activity_tick: u64,
    pub completed_event_refs: Vec<String>,
    pub applied_operations: BTreeMap<String, AppliedActorOperation>,
}

impl ActorState {
    #[must_use]
    pub fn dormant(
        actor_key_ref: impl Into<String>,
        profile_ref: impl Into<String>,
        system_extension_manifest_ref: impl Into<String>,
        placement_ref: impl Into<String>,
        extension_generation: u64,
    ) -> Self {
        Self {
            schema: ACTOR_STATE_SCHEMA.to_string(),
            actor_key_ref: actor_key_ref.into(),
            profile_ref: profile_ref.into(),
            system_extension_manifest_ref: system_extension_manifest_ref.into(),
            placement_ref: placement_ref.into(),
            extension_generation,
            lifecycle_sequence: ADDRESSABLE_ACTOR_INITIAL_SEQUENCE,
            revision: ADDRESSABLE_ACTOR_INITIAL_REVISION,
            phase: ActorPhase::Dormant,
            checkpoint_ref: None,
            durable_state_ref: None,
            active_wake_ref: None,
            unknown_effect_ref: None,
            mailbox_revision: ADDRESSABLE_ACTOR_INITIAL_REVISION,
            last_activity_tick: ADDRESSABLE_ACTOR_INITIAL_SEQUENCE,
            completed_event_refs: Vec::new(),
            applied_operations: BTreeMap::new(),
        }
    }
}
