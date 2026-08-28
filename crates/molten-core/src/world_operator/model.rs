mod plan;
mod receipt;
mod request;

pub use plan::*;
pub use receipt::*;
pub use request::*;

pub const WORLD_WORKFLOW_REQUEST_SCHEMA: &str = "molten.world-workflow-request.v1";
pub const WORLD_WORKFLOW_PLAN_SCHEMA: &str = "molten.world-workflow-plan.v1";
pub const WORLD_WORKFLOW_RECEIPT_SCHEMA: &str = "molten.world-workflow-receipt.v1";
pub const WORLD_WORKFLOW_SUMMARY_SCHEMA: &str = "molten.world-workflow-summary.v1";

pub const MAX_WORLD_OPERATOR_OPERATIONS: usize = 32;
pub const MAX_WORLD_OPERATOR_DEPENDENCIES: usize = 32;
pub const MAX_WORLD_OPERATOR_PROFILES: usize = 16;
pub const MAX_WORLD_OPERATOR_OBSERVATIONS: usize = 32;
pub const MAX_WORLD_OPERATOR_RECEIPT_LINKS: usize = 64;
pub const MAX_WORLD_OPERATOR_CANONICAL_BYTES: usize = 262_144;
pub const MAX_WORLD_OPERATOR_TEXT_BYTES: usize = 512;
pub const MAX_WORLD_OPERATOR_DIAGNOSTICS: usize = 16;

pub const WORLD_OPERATOR_NON_CLAIMS: &[&str] = &[
    "workflow plans do not execute component operations",
    "aggregate receipts do not replace component receipts",
    "aggregate completion does not prove component correctness",
    "plans and receipts do not grant branch or effect authority",
    "promotion planning does not authorize effect dispatch",
    "garbage-collection planning does not grant deletion authority",
    "opaque replay does not imply logical merge or semantic equivalence",
    "unavailable stronger profiles never fall back to weaker profiles",
    "dogfood evidence does not prove whole-stack correctness or release eligibility",
];

pub fn world_operator_non_claims() -> Vec<String> {
    WORLD_OPERATOR_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect()
}
