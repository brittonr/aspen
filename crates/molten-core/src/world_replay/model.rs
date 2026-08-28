mod capsule;
mod evidence;
mod plan;
mod trace;

pub use capsule::*;
pub use evidence::*;
pub use plan::*;
pub use trace::*;

pub const WORLD_TRANSITION_TRACE_SCHEMA: &str = "molten.world-replay.transition-trace.v1";
pub const WORLD_REPLAY_CAPSULE_SCHEMA: &str = "molten.world-replay.capsule.v1";
pub const WORLD_REPLAY_PLAN_SCHEMA: &str = "molten.world-replay.plan.v1";
pub const WORLD_REPLAY_DIVERGENCE_SCHEMA: &str = "molten.world-replay.divergence.v1";
pub const WORLD_REPLAY_RECEIPT_SCHEMA: &str = "molten.world-replay.receipt.v1";
pub const WORLD_REPLAY_IMPORT_RECEIPT_SCHEMA: &str = "molten.world-replay.import-receipt.v1";

pub const WORLD_TRANSITION_TRACE_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-replay.trace.v1";
pub const WORLD_REPLAY_CAPSULE_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-replay.capsule.v1";
pub const WORLD_REPLAY_PLAN_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-replay.plan.v1";
pub const WORLD_REPLAY_DIVERGENCE_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-replay.divergence.v1";

pub const MAX_WORLD_REPLAY_STEPS: usize = 256;
pub const MAX_WORLD_REPLAY_MEMBERS: usize = 4_096;
pub const MAX_WORLD_REPLAY_MEMBER_BYTES: u64 = 1_073_741_824;
pub const MAX_WORLD_REPLAY_TOTAL_BYTES: u64 = 17_179_869_184;
pub const MAX_WORLD_REPLAY_FIELD_PATH_SEGMENTS: usize = 32;
pub const MAX_WORLD_REPLAY_FIELD_SEGMENT_BYTES: usize = 128;
pub const MAX_WORLD_REPLAY_DIAGNOSTICS: usize = 64;
pub const MAX_WORLD_REPLAY_ROLES_PER_MEMBER: usize = 8;
pub const MAX_WORLD_REPLAY_DEPENDENCY_REFS: usize = 64;
pub const MAX_WORLD_REPLAY_CANONICAL_BYTES: usize = 4_194_304;
pub const MAX_WORLD_REPLAY_TEXT_BYTES: usize = 256;
pub const WORLD_REPLAY_OPERATIONS_PER_STEP: usize = 3;
pub const WORLD_REPLAY_FIXED_OPERATIONS: usize = 3;

pub const WORLD_REPLAY_NON_CLAIMS: &[&str] = &[
    "replay does not prove universal determinism",
    "logical and opaque profiles are not semantically equivalent",
    "capsule possession does not transfer capability or authority",
    "replay does not prove external effect completion",
    "import does not move a branch or activate a runtime",
    "replay and import receipts do not grant release eligibility",
];
