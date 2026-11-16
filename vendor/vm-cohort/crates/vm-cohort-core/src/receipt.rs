use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::{
    CohortPhase, CohortState, MAX_CLEANUP_OBLIGATIONS, MAX_OBSERVATIONS, RECEIPT_NON_CLAIMS,
    ReceiptRef, ReferenceError,
};

const RECEIPT_DOMAIN: &str = "onixresearch.vm-cohort.receipt.v1";

/// Bounded lifecycle receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortReceipt {
    /// Receipt identity.
    pub receipt_ref: ReceiptRef,
    /// Cohort identity.
    pub cohort_ref: crate::CohortRef,
    /// Checkpoint identity.
    pub checkpoint_ref: crate::CheckpointRef,
    /// Resource-policy identity.
    pub policy_ref: crate::ProfileRef,
    /// Current phase.
    pub phase: CohortPhase,
    /// Successful effect count.
    pub completed_operations: u64,
    /// Activated clone count.
    pub activated_clones: u64,
    /// Cleanup obligation count.
    pub cleanup_obligations: u64,
    /// Released cleanup count.
    pub released_cleanup_operations: u64,
    /// Accepted observation identities.
    pub observation_refs: Vec<ReceiptRef>,
    /// Mandatory claim boundaries.
    pub non_claims: Vec<String>,
}

/// Receipt projection issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReceiptIssue {
    /// State exceeds an evidence bound.
    BoundExceeded,
    /// Active state is incomplete.
    PartialActivation,
    /// Cleaned state still has unresolved obligations.
    IncompleteCleanup,
    /// Receipt identity construction failed.
    Identity,
}

/// Projects a deterministic bounded receipt from one lifecycle state.
///
/// # Errors
///
/// Returns bounded receipt issues for overbound or internally inconsistent state.
// r[impl vm_cohort.receipts]
pub fn project_receipt(state: &CohortState) -> Result<CohortReceipt, Vec<ReceiptIssue>> {
    let mut issues = Vec::with_capacity(ReceiptIssue::MAX_COUNT);
    if state.observation_refs.len() > MAX_OBSERVATIONS
        || state.cleanup_obligations.len() > MAX_CLEANUP_OBLIGATIONS
    {
        issues.push(ReceiptIssue::BoundExceeded);
    }
    if state.phase == CohortPhase::Active
        && (state.activated_clones.len() != state.plan.clones.len()
            || state.completed_operations.len() != state.plan.effects.len())
    {
        issues.push(ReceiptIssue::PartialActivation);
    }
    if state.phase == CohortPhase::Cleaned
        && state.released_cleanup_operations.len() != state.cleanup_obligations.len()
    {
        issues.push(ReceiptIssue::IncompleteCleanup);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let completed_operations = count(state.completed_operations.len())?;
    let activated_clones = count(state.activated_clones.len())?;
    let cleanup_obligations = count(state.cleanup_obligations.len())?;
    let released_cleanup_operations = count(state.released_cleanup_operations.len())?;
    let non_claims = RECEIPT_NON_CLAIMS
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut receipt = CohortReceipt {
        receipt_ref: ReceiptRef::new(reference_placeholder())
            .map_err(|ReferenceError| vec![ReceiptIssue::Identity])?,
        cohort_ref: state.plan.cohort_ref.clone(),
        checkpoint_ref: state.plan.checkpoint_ref.clone(),
        policy_ref: state.plan.policy_ref.clone(),
        phase: state.phase,
        completed_operations,
        activated_clones,
        cleanup_obligations,
        released_cleanup_operations,
        observation_refs: state.observation_refs.clone(),
        non_claims,
    };
    receipt.receipt_ref =
        identify_receipt(&receipt).map_err(|ReferenceError| vec![ReceiptIssue::Identity])?;
    Ok(receipt)
}

fn identify_receipt(receipt: &CohortReceipt) -> Result<ReceiptRef, ReferenceError> {
    let mut hasher = blake3::Hasher::new_derive_key(RECEIPT_DOMAIN);
    crate::identity::update_text(&mut hasher, receipt.cohort_ref.as_str())?;
    crate::identity::update_text(&mut hasher, receipt.checkpoint_ref.as_str())?;
    crate::identity::update_text(&mut hasher, receipt.policy_ref.as_str())?;
    crate::identity::update_text(&mut hasher, phase_name(receipt.phase))?;
    crate::identity::update_u64(&mut hasher, receipt.completed_operations);
    crate::identity::update_u64(&mut hasher, receipt.activated_clones);
    crate::identity::update_u64(&mut hasher, receipt.cleanup_obligations);
    crate::identity::update_u64(&mut hasher, receipt.released_cleanup_operations);
    for observation_ref in &receipt.observation_refs {
        crate::identity::update_text(&mut hasher, observation_ref.as_str())?;
    }
    for non_claim in &receipt.non_claims {
        crate::identity::update_text(&mut hasher, non_claim)?;
    }
    ReceiptRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn count(value: usize) -> Result<u64, Vec<ReceiptIssue>> {
    u64::try_from(value).map_err(|_| vec![ReceiptIssue::BoundExceeded])
}

fn reference_placeholder() -> String {
    const ZERO_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    format!("blake3:{ZERO_HEX}")
}

const fn phase_name(phase: CohortPhase) -> &'static str {
    match phase {
        CohortPhase::Planned => "planned",
        CohortPhase::Preparing => "preparing",
        CohortPhase::Active => "active",
        CohortPhase::CleanupRequired => "cleanup-required",
        CohortPhase::CleanupUncertain => "cleanup-uncertain",
        CohortPhase::Cleaned => "cleaned",
        CohortPhase::Failed => "failed",
        CohortPhase::Abandoned => "abandoned",
    }
}

impl ReceiptIssue {
    const MAX_COUNT: usize = 4;
}
