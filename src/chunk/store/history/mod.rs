//! Per-manifest receipt lineage built on `evidence_chain` links and predicate receipts. Lineage is
//! evidence-only: it binds a manifest and root to its pass receipts and never names a global head.

pub(super) mod assemble;
mod attest;
pub(super) mod audit;
mod binding;
mod scan;
