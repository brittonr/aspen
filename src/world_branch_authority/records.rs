use molten_core::world_branch_authority::MAXIMUM_REALIZATION_EVIDENCE;
use molten_core::world_branch_authority::WORLD_BRANCH_AUTHORITY_NON_CLAIMS;
use molten_core::world_branch_authority::WorldBranchActivationDecision;
use molten_core::world_branch_authority::WorldBranchAuthorityDiagnostic;
use molten_core::world_branch_authority::WorldBranchAuthorityPlan;
use molten_core::world_branch_authority::WorldBranchRealizationObservation;
use molten_core::world_branch_authority::valid_content_ref;
use preserves::IOValue;

use super::ports::ActivationOutcome;
use crate::error::MoltenError;
use crate::error::Result;

mod vocabulary;

use vocabulary::*;

const RECEIPT_SCHEMA: &str = "molten.world-branch-authority-receipt.v1";
const RECEIPT_RECORD: &str = "molten-world-branch-authority-receipt-v1";
const RECEIPT_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-branch-authority.receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchAuthorityReceiptKind {
    Plan,
    Activation,
    ActivationOutcome,
}

impl BranchAuthorityReceiptKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Activation => "activation",
            Self::ActivationOutcome => "activation-outcome",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBranchAuthorityReceipt {
    pub schema: &'static str,
    pub kind: BranchAuthorityReceiptKind,
    pub plan_ref: String,
    pub decision_ref: Option<String>,
    pub allowed: bool,
    pub policy_ref: String,
    pub capability_ref: String,
    pub mode: Option<String>,
    pub obligations: Vec<String>,
    pub diagnostic: String,
    pub operation_ref: Option<String>,
    pub promotion_plan_ref: Option<String>,
    pub release_reservation_ref: Option<String>,
    pub activation_outcome: Option<String>,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<String>,
}

// r[impl molten.world_branch_authority.evidence]
pub fn plan_receipt(plan: &WorldBranchAuthorityPlan) -> WorldBranchAuthorityReceipt {
    WorldBranchAuthorityReceipt {
        schema: RECEIPT_SCHEMA,
        kind: BranchAuthorityReceiptKind::Plan,
        plan_ref: plan.plan_ref.clone(),
        decision_ref: None,
        allowed: plan.allowed,
        policy_ref: plan.policy_ref.clone(),
        capability_ref: plan.capability_ref.clone(),
        mode: plan.mode.map(|mode| mode.as_str().to_string()),
        obligations: plan.obligations.iter().map(|obligation| obligation.as_str().to_string()).collect(),
        diagnostic: diagnostic_text(plan.diagnostic).to_string(),
        operation_ref: None,
        promotion_plan_ref: None,
        release_reservation_ref: None,
        activation_outcome: None,
        evidence_refs: Vec::new(),
        non_claims: plan.non_claims.clone(),
    }
}

pub fn activation_receipt(
    plan: &WorldBranchAuthorityPlan,
    observation: &WorldBranchRealizationObservation,
    decision: &WorldBranchActivationDecision,
) -> WorldBranchAuthorityReceipt {
    WorldBranchAuthorityReceipt {
        schema: RECEIPT_SCHEMA,
        kind: BranchAuthorityReceiptKind::Activation,
        plan_ref: plan.plan_ref.clone(),
        decision_ref: Some(decision.decision_ref.clone()),
        allowed: decision.allowed,
        policy_ref: plan.policy_ref.clone(),
        capability_ref: plan.capability_ref.clone(),
        mode: plan.mode.map(|mode| mode.as_str().to_string()),
        obligations: plan.obligations.iter().map(|obligation| obligation.as_str().to_string()).collect(),
        diagnostic: diagnostic_text(decision.diagnostic).to_string(),
        operation_ref: Some(observation.operation_ref.clone()),
        promotion_plan_ref: observation
            .promotion_admission
            .as_ref()
            .map(|admission| admission.promotion_plan_ref.clone()),
        release_reservation_ref: observation.release_reservation_ref.clone(),
        activation_outcome: None,
        evidence_refs: observation.evidence_refs.clone(),
        non_claims: decision.non_claims.clone(),
    }
}

pub fn activation_outcome_receipt(
    plan: &WorldBranchAuthorityPlan,
    observation: &WorldBranchRealizationObservation,
    decision: &WorldBranchActivationDecision,
    outcome: ActivationOutcome,
) -> WorldBranchAuthorityReceipt {
    let diagnostic = match outcome {
        ActivationOutcome::Activated => decision.diagnostic,
        ActivationOutcome::Denied => WorldBranchAuthorityDiagnostic::ActivationDenied,
        ActivationOutcome::Unknown => WorldBranchAuthorityDiagnostic::ActivationOutcomeUnknown,
    };
    WorldBranchAuthorityReceipt {
        schema: RECEIPT_SCHEMA,
        kind: BranchAuthorityReceiptKind::ActivationOutcome,
        plan_ref: plan.plan_ref.clone(),
        decision_ref: Some(decision.decision_ref.clone()),
        allowed: outcome == ActivationOutcome::Activated,
        policy_ref: plan.policy_ref.clone(),
        capability_ref: plan.capability_ref.clone(),
        mode: plan.mode.map(|mode| mode.as_str().to_string()),
        obligations: plan.obligations.iter().map(|obligation| obligation.as_str().to_string()).collect(),
        diagnostic: diagnostic_text(diagnostic).to_string(),
        operation_ref: Some(observation.operation_ref.clone()),
        promotion_plan_ref: observation
            .promotion_admission
            .as_ref()
            .map(|admission| admission.promotion_plan_ref.clone()),
        release_reservation_ref: observation.release_reservation_ref.clone(),
        activation_outcome: Some(outcome.as_str().to_string()),
        evidence_refs: observation.evidence_refs.clone(),
        non_claims: decision.non_claims.clone(),
    }
}

pub fn encode_receipt(receipt: &WorldBranchAuthorityReceipt) -> Result<(String, Vec<u8>)> {
    validate_receipt(receipt)?;
    let value = crate::preserves_rail::record(RECEIPT_RECORD, vec![
        string(receipt.schema),
        field("kind", string(receipt.kind.as_str())),
        field("plan-ref", string(&receipt.plan_ref)),
        field("decision-ref", optional_text(receipt.decision_ref.as_deref())),
        field("allowed", boolean(receipt.allowed)),
        field("policy-ref", optional_text(non_empty(&receipt.policy_ref))),
        field("capability-ref", optional_text(non_empty(&receipt.capability_ref))),
        field("mode", optional_text(receipt.mode.as_deref())),
        field("obligations", sequence(receipt.obligations.iter().map(string).collect())),
        field("diagnostic", string(&receipt.diagnostic)),
        field("operation-ref", optional_text(receipt.operation_ref.as_deref())),
        field("promotion-plan-ref", optional_text(receipt.promotion_plan_ref.as_deref())),
        field("release-reservation-ref", optional_text(receipt.release_reservation_ref.as_deref())),
        field("activation-outcome", optional_text(receipt.activation_outcome.as_deref())),
        field("evidence-refs", sequence(receipt.evidence_refs.iter().map(string).collect())),
        field("non-claims", sequence(receipt.non_claims.iter().map(string).collect())),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let mut hasher = blake3::Hasher::new_derive_key(RECEIPT_IDENTITY_DOMAIN);
    let length =
        u64::try_from(bytes.len()).map_err(|_| MoltenError::invalid_harness("branch-authority receipt exceeds u64"))?;
    hasher.update(&length.to_le_bytes());
    hasher.update(&bytes);
    Ok((format!("blake3:{}", hasher.finalize().to_hex()), bytes))
}

fn validate_receipt(receipt: &WorldBranchAuthorityReceipt) -> Result<()> {
    if receipt.schema != RECEIPT_SCHEMA || !valid_content_ref(&receipt.plan_ref) {
        return Err(MoltenError::invalid_harness("world branch authority receipt schema or plan ref is invalid"));
    }
    for reference in [
        receipt.decision_ref.as_deref(),
        non_empty(&receipt.policy_ref),
        non_empty(&receipt.capability_ref),
        receipt.operation_ref.as_deref(),
        receipt.promotion_plan_ref.as_deref(),
        receipt.release_reservation_ref.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !valid_content_ref(reference) {
            return Err(MoltenError::invalid_harness("world branch authority receipt contains an invalid reference"));
        }
    }
    if receipt.evidence_refs.len() > MAXIMUM_REALIZATION_EVIDENCE
        || receipt.evidence_refs.iter().any(|reference| !valid_content_ref(reference))
    {
        return Err(MoltenError::invalid_harness("world branch authority receipt evidence is invalid"));
    }
    if receipt.obligations.is_empty()
        || receipt.obligations.len() > MAXIMUM_REALIZATION_EVIDENCE
        || receipt.obligations.iter().any(|obligation| !valid_obligation(obligation))
        || !receipt.mode.as_deref().is_none_or(valid_mode)
        || !valid_diagnostic(&receipt.diagnostic)
        || !receipt.activation_outcome.as_deref().is_none_or(valid_activation_outcome)
    {
        return Err(MoltenError::invalid_harness("world branch authority receipt contains non-closed vocabulary"));
    }
    let expected = WORLD_BRANCH_AUTHORITY_NON_CLAIMS.iter().map(ToString::to_string).collect::<Vec<_>>();
    if receipt.non_claims != expected {
        return Err(MoltenError::invalid_harness("world branch authority receipt non-claims are incomplete"));
    }
    Ok(())
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn optional_text(value: Option<&str>) -> IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |text| crate::preserves_rail::record("some", vec![string(text)]),
    )
}

fn boolean(value: bool) -> IOValue {
    crate::preserves_rail::record(if value { "true" } else { "false" }, Vec::new())
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}
