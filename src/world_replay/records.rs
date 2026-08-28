mod receipt;
mod support;
mod value;

use preserves::IOValue;
pub use receipt::*;
pub use value::*;

pub const WORLD_TRANSITION_TRACE_RECORD: &str = "molten-world-transition-trace-v1";
pub const WORLD_REPLAY_CAPSULE_RECORD: &str = "molten-world-replay-capsule-v1";
pub const WORLD_REPLAY_PLAN_RECORD: &str = "molten-world-replay-plan-v1";
pub const WORLD_REPLAY_DIVERGENCE_RECORD: &str = "molten-world-replay-divergence-v1";
pub const WORLD_REPLAY_RECEIPT_RECORD: &str = "molten-world-replay-receipt-v1";
pub const WORLD_REPLAY_IMPORT_RECEIPT_RECORD: &str = "molten-world-replay-import-receipt-v1";

#[derive(Debug, Clone)]
pub struct CanonicalWorldReplayRecord {
    pub kind: &'static str,
    pub record_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}
