use molten_core::world_replay::*;
use preserves::IOValue;

use super::CanonicalWorldReplayRecord;
use super::WORLD_REPLAY_IMPORT_RECEIPT_RECORD;
use super::WORLD_REPLAY_RECEIPT_RECORD;
use super::support::*;
use crate::error::Failure;
use crate::error::Result;

const WORLD_REPLAY_RECEIPT_IDENTITY_CONTEXT: &str = "onixresearch.molten.world-replay.receipt.v1";
const WORLD_REPLAY_IMPORT_RECEIPT_IDENTITY_CONTEXT: &str = "onixresearch.molten.world-replay.import-receipt.v1";

// r[impl molten.world_replay.receipts]
pub fn canonicalize_world_replay_receipt(
    mut receipt: WorldReplayReceipt,
) -> Result<(WorldReplayReceipt, CanonicalWorldReplayRecord)> {
    validate_receipt_common(&ReceiptValidationInput {
        trace_ref: &receipt.trace_ref,
        capsule_ref: &receipt.capsule_ref,
        profile_ref: &receipt.profile_ref,
        dependencies: &receipt.dependency_refs,
        diagnostics: &receipt.diagnostics,
        non_claims: &receipt.non_claims,
    })?;
    if receipt.schema != WORLD_REPLAY_RECEIPT_SCHEMA {
        return Err(Failure::invalid_harness("world replay receipt schema is invalid"));
    }
    for reference in &receipt.actual_transition_refs {
        validate_ref(reference, "actual transition observation")?;
    }
    if let Some(reference) = &receipt.divergence_ref {
        validate_ref(reference, "replay divergence")?;
    }
    if let Some(reference) = &receipt.current_admission_ref {
        validate_ref(reference, "replay current admission")?;
    }
    let claim_blockers = validate_replay_boundary_claims(&receipt.boundary);
    if !claim_blockers.is_empty() {
        return Err(Failure::invalid_harness(format!("world replay receipt overclaims: {claim_blockers:?}")));
    }
    let identity_value = replay_receipt_value(&receipt, false);
    receipt.receipt_ref = domain_identity(WORLD_REPLAY_RECEIPT_IDENTITY_CONTEXT, &identity_value)?;
    let value = replay_receipt_value(&receipt, true);
    let record = canonical("receipt", WORLD_REPLAY_RECEIPT_RECORD, value)?;
    Ok((receipt, record))
}

// r[impl molten.world_replay.import]
pub fn canonicalize_world_replay_import_receipt(
    mut receipt: WorldReplayImportReceipt,
) -> Result<(WorldReplayImportReceipt, CanonicalWorldReplayRecord)> {
    if receipt.schema != WORLD_REPLAY_IMPORT_RECEIPT_SCHEMA {
        return Err(Failure::invalid_harness("world replay import receipt schema is invalid"));
    }
    validate_ref(&receipt.capsule_ref, "import capsule")?;
    if let Some(reference) = &receipt.availability_ref {
        validate_ref(reference, "capsule availability")?;
    }
    validate_diagnostics(&receipt.diagnostics)?;
    require_non_claims(&receipt.non_claims)?;
    if receipt.branch_moved || receipt.runtime_activated || receipt.authority_granted {
        return Err(Failure::invalid_harness("world replay import receipt claims forbidden mutation or authority"));
    }
    let identity_value = import_receipt_value(&receipt, false);
    receipt.receipt_ref = domain_identity(WORLD_REPLAY_IMPORT_RECEIPT_IDENTITY_CONTEXT, &identity_value)?;
    let value = import_receipt_value(&receipt, true);
    let record = canonical("import-receipt", WORLD_REPLAY_IMPORT_RECEIPT_RECORD, value)?;
    Ok((receipt, record))
}

fn replay_receipt_value(receipt: &WorldReplayReceipt, include_ref: bool) -> IOValue {
    let mut values = Vec::with_capacity(WORLD_REPLAY_RECEIPT_FIELD_CAPACITY);
    values.push(string(&receipt.schema));
    if include_ref {
        values.push(field("receipt-ref", string(&receipt.receipt_ref)));
    }
    values.extend([
        field("decision", string(receipt.decision.as_str())),
        field("trace-ref", string(&receipt.trace_ref)),
        field("capsule-ref", string(&receipt.capsule_ref)),
        field("profile-ref", string(&receipt.profile_ref)),
        field("horizon", usize_value(receipt.horizon)),
        field("actual-transitions", sequence(receipt.actual_transition_refs.iter().map(string).collect())),
        field("divergence-ref", optional_ref(receipt.divergence_ref.as_deref())),
        field("current-admission-ref", optional_ref(receipt.current_admission_ref.as_deref())),
        field("dependencies", sequence(receipt.dependency_refs.iter().map(string).collect())),
        field("diagnostics", sequence(receipt.diagnostics.iter().map(string).collect())),
        boundary_value(&receipt.boundary),
        field("universal-determinism-proved", boolean(false)),
        field("semantic-equivalence-proved", boolean(false)),
        field("capability-transferred", boolean(false)),
        field("effects-completed", boolean(false)),
        field("release-authorized", boolean(false)),
        non_claims_value(&receipt.non_claims),
    ]);
    record(WORLD_REPLAY_RECEIPT_RECORD, values)
}

/// The boundary facts of a replay receipt: inventory, handling classes,
/// horizon, cohorts, effect sealing, opaque diagnostics, redaction, claims,
/// and non-claims.
// r[impl molten.world_replay_boundary.claims]
fn boundary_value(boundary: &ReplayBoundarySummary) -> IOValue {
    let horizon = match boundary.horizon {
        Some((step, position)) => sequence(vec![number(step), number(position)]),
        None => sequence(Vec::new()),
    };
    field(
        "observation-boundary",
        record("replay-observation-boundary", vec![
            field("inventory-ref", string(&boundary.inventory_ref)),
            field("ledger-ref", string(&boundary.ledger_ref)),
            field("deterministic-sources", number(u64::from(boundary.deterministic_sources))),
            field("simulated-sources", number(u64::from(boundary.simulated_sources))),
            field("recorded-sources", number(u64::from(boundary.recorded_sources))),
            field("recorded-observations", number(u64::from(boundary.recorded_observations))),
            field("horizon", horizon),
            field("adapter-cohort-ref", string(&boundary.adapter_cohort_ref)),
            field("runtime-cohort-ref", optional_ref(boundary.runtime_cohort_ref.as_deref())),
            field("sealed-effects", number(u64::from(boundary.sealed_effects))),
            field("repeated-effect-attempts", number(u64::from(boundary.repeated_effect_attempts))),
            field("opaque-diagnostics", sequence(boundary.opaque_diagnostic_refs.iter().map(string).collect())),
            field("redaction", string(redaction_label(boundary.redaction))),
            field("claims", sequence(boundary.claims.iter().map(|claim| string(claim.as_str())).collect())),
            field("non-claims", sequence(boundary.non_claims.iter().map(string).collect())),
        ]),
    )
}

const fn redaction_label(redaction: ReplayRedaction) -> &'static str {
    match redaction {
        ReplayRedaction::NoSensitiveSources => "no-sensitive-sources",
        ReplayRedaction::SensitiveProtected => "sensitive-protected",
    }
}

fn import_receipt_value(receipt: &WorldReplayImportReceipt, include_ref: bool) -> IOValue {
    let mut values = Vec::with_capacity(WORLD_REPLAY_IMPORT_RECEIPT_FIELD_CAPACITY);
    values.push(string(&receipt.schema));
    if include_ref {
        values.push(field("receipt-ref", string(&receipt.receipt_ref)));
    }
    values.extend([
        field("decision", string(receipt.decision.as_str())),
        field("capsule-ref", string(&receipt.capsule_ref)),
        field("verified-members", usize_value(receipt.verified_members)),
        field("availability-ref", optional_ref(receipt.availability_ref.as_deref())),
        field("diagnostics", sequence(receipt.diagnostics.iter().map(string).collect())),
        field("branch-moved", boolean(receipt.branch_moved)),
        field("runtime-activated", boolean(receipt.runtime_activated)),
        field("authority-granted", boolean(receipt.authority_granted)),
        non_claims_value(&receipt.non_claims),
    ]);
    record(WORLD_REPLAY_IMPORT_RECEIPT_RECORD, values)
}

const WORLD_REPLAY_RECEIPT_FIELD_CAPACITY: usize = 19;
const WORLD_REPLAY_IMPORT_RECEIPT_FIELD_CAPACITY: usize = 11;
