mod dispatch;
mod plan;
mod reference;

pub use dispatch::*;
pub use plan::*;
pub use reference::*;

pub const TRANSACTIONAL_RECONCILIATION_REVISION: &str = "eb2bd3441753af97bfcb247cef7cc22d72675b62";
pub const TRANSACTIONAL_RECONCILIATION_RID: &str = "rad:z4Tky6zvC8w4Y6c4YBzNxVbq5n752";
pub const TRANSACTIONAL_RECONCILIATION_SOURCE: &str = "https://seed.radicle.garden/z4Tky6zvC8w4Y6c4YBzNxVbq5n752.git";
pub const MAX_WORLD_PROMOTION_INTENTS: usize = 256;
pub const MAX_WORLD_PROMOTION_PREREQUISITES: u32 = 8;
pub const MAX_WORLD_PROMOTION_TRANSACTION_OPERATIONS: u32 = 257;
pub const MAX_WORLD_PROMOTION_DIAGNOSTICS: usize = 128;
pub const WORLD_PROMOTION_NON_CLAIMS: &[&str] = &[
    "promotion commits local eligibility, not external effect completion",
    "a reservation does not prove dispatch or acknowledgment",
    "an attempt does not prove an external effect occurred",
    "unknown outcomes do not imply success or failure",
    "retry identity does not prove exactly-once execution",
    "promotion evidence does not grant capability, policy, adapter, or release authority",
];

pub fn promotion_non_claims() -> Vec<String> {
    WORLD_PROMOTION_NON_CLAIMS.iter().map(ToString::to_string).collect()
}
