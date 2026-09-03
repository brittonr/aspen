use serde::Deserialize;
use serde::Serialize;

use super::*;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorKey {
    pub schema: String,
    pub namespace_ref: String,
    pub actor_type: String,
    pub key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorReferenceSource {
    pub repository: String,
    pub revision: String,
    pub license: String,
    pub selected_concepts: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurvivalClass {
    DurableState,
    MailboxEntries,
    CompletedSemanticEvents,
    Checkpoints,
    Processes,
    Streams,
    Sessions,
    PartialCallbacks,
    InFlightDeltas,
}

impl SurvivalClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DurableState => "durable-state",
            Self::MailboxEntries => "mailbox-entries",
            Self::CompletedSemanticEvents => "completed-semantic-events",
            Self::Checkpoints => "checkpoints",
            Self::Processes => "processes",
            Self::Streams => "streams",
            Self::Sessions => "sessions",
            Self::PartialCallbacks => "partial-callbacks",
            Self::InFlightDeltas => "in-flight-deltas",
        }
    }

    pub const fn all() -> [Self; MAX_ACTOR_RESTORE_CLASSES] {
        [
            Self::DurableState,
            Self::MailboxEntries,
            Self::CompletedSemanticEvents,
            Self::Checkpoints,
            Self::Processes,
            Self::Streams,
            Self::Sessions,
            Self::PartialCallbacks,
            Self::InFlightDeltas,
        ]
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurvivalDisposition {
    Durable,
    RuntimeOnly,
    Unsupported,
}

impl SurvivalDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::RuntimeOnly => "runtime-only",
            Self::Unsupported => "unsupported",
        }
    }

    pub const fn permits_restore(self) -> bool {
        matches!(self, Self::Durable)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurvivalRule {
    pub class: SurvivalClass,
    pub disposition: SurvivalDisposition,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSurvivalMatrix {
    pub schema: String,
    pub profile_version: u32,
    pub rules: Vec<SurvivalRule>,
}

impl ActorSurvivalMatrix {
    #[must_use]
    pub fn disposition(&self, class: SurvivalClass) -> Option<SurvivalDisposition> {
        self.rules.iter().find(|rule| rule.class == class).map(|rule| rule.disposition)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AddressableActorProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_version: u32,
    pub reference_source: ActorReferenceSource,
    pub system_extension_profile_ref: String,
    pub placement_profile_ref: String,
    pub delivery_profile_ref: String,
    pub durable_state_profile_ref: String,
    pub time_profile_ref: String,
    pub resource_profile_ref: String,
    pub supervision_profile_ref: String,
    pub authority_profile_ref: String,
    pub evidence_profile_ref: String,
    pub idle_after_ticks: u64,
    pub maximum_drain_items: u32,
    pub survival: ActorSurvivalMatrix,
    pub non_claims: Vec<String>,
}

#[must_use]
pub fn standard_actor_survival_matrix() -> ActorSurvivalMatrix {
    ActorSurvivalMatrix {
        schema: ACTOR_SURVIVAL_MATRIX_SCHEMA.to_string(),
        profile_version: ADDRESSABLE_ACTOR_PROFILE_VERSION,
        rules: vec![
            SurvivalRule {
                class: SurvivalClass::DurableState,
                disposition: SurvivalDisposition::Durable,
            },
            SurvivalRule {
                class: SurvivalClass::MailboxEntries,
                disposition: SurvivalDisposition::Durable,
            },
            SurvivalRule {
                class: SurvivalClass::CompletedSemanticEvents,
                disposition: SurvivalDisposition::Durable,
            },
            SurvivalRule {
                class: SurvivalClass::Checkpoints,
                disposition: SurvivalDisposition::Durable,
            },
            SurvivalRule {
                class: SurvivalClass::Processes,
                disposition: SurvivalDisposition::RuntimeOnly,
            },
            SurvivalRule {
                class: SurvivalClass::Streams,
                disposition: SurvivalDisposition::RuntimeOnly,
            },
            SurvivalRule {
                class: SurvivalClass::Sessions,
                disposition: SurvivalDisposition::RuntimeOnly,
            },
            SurvivalRule {
                class: SurvivalClass::PartialCallbacks,
                disposition: SurvivalDisposition::Unsupported,
            },
            SurvivalRule {
                class: SurvivalClass::InFlightDeltas,
                disposition: SurvivalDisposition::Unsupported,
            },
        ],
    }
}
