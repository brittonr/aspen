pub const ACTOR_KEY_SCHEMA: &str = "molten.addressable-actor.key.v1";
pub const ACTOR_PROFILE_SCHEMA: &str = "molten.addressable-actor.profile.v1";
pub const ACTOR_SURVIVAL_MATRIX_SCHEMA: &str = "molten.addressable-actor.survival-matrix.v1";
pub const ACTOR_STATE_SCHEMA: &str = "molten.addressable-actor.state.v1";
pub const ACTOR_REQUEST_SCHEMA: &str = "molten.addressable-actor.request.v1";
pub const ACTOR_TRANSITION_SCHEMA: &str = "molten.addressable-actor.transition.v1";
pub const ACTOR_EFFECT_INTENT_SCHEMA: &str = "molten.addressable-actor.effect-intent.v1";
pub const ACTOR_STATUS_SCHEMA: &str = "molten.addressable-actor.status.v1";
pub const ACTOR_HOST_BINDING_SCHEMA: &str = "molten.addressable-actor.host-binding.v1";

pub const ADDRESSABLE_ACTOR_PROFILE_VERSION: u32 = 1;
pub const ADDRESSABLE_ACTOR_INITIAL_GENERATION: u64 = 1;
pub const ADDRESSABLE_ACTOR_INITIAL_SEQUENCE: u64 = 0;
pub const ADDRESSABLE_ACTOR_INITIAL_REVISION: u64 = 0;
pub const ADDRESSABLE_ACTOR_SEQUENCE_INCREMENT: u64 = 1;
pub const ADDRESSABLE_ACTOR_REVISION_INCREMENT: u64 = 1;
pub const MAX_ACTOR_KEY_BYTES: usize = 256;
pub const MAX_ACTOR_OPERATION_ID_BYTES: usize = 192;
pub const MAX_ACTOR_OPERATIONS: usize = 128;
pub const MAX_ACTOR_COMPLETED_EVENTS: usize = 128;
pub const MAX_ACTOR_RESTORE_CLASSES: usize = 9;
pub const MAX_ACTOR_EFFECTS_PER_TRANSITION: usize = 8;

pub const RIVET_ACTORS_REPOSITORY: &str = "https://github.com/rivet-dev/actors";
pub const RIVET_ACTORS_REVISION: &str = "71f371ba4eab1234d8b6b6c419e6748cc6fc9911";
pub const RIVET_ACTORS_LICENSE: &str = "Apache-2.0";

const REQUIRED_NON_CLAIM_COUNT: usize = 6;

pub const REQUIRED_ADDRESSABLE_ACTOR_NON_CLAIMS: [&str; REQUIRED_NON_CLAIM_COUNT] = [
    "actor-identity-does-not-grant-authority",
    "wake-success-does-not-prove-transport-delivery",
    "checkpoint-possession-does-not-prove-survival",
    "runtime-processes-streams-sessions-callbacks-and-deltas-do-not-survive",
    "unknown-external-effects-are-not-retried-automatically",
    "actor-receipts-do-not-authorize-mutation-activation-release-or-retry",
];

#[must_use]
pub fn required_addressable_actor_non_claims() -> Vec<String> {
    REQUIRED_ADDRESSABLE_ACTOR_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect()
}
