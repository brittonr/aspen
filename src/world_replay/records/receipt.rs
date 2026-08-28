use molten_core::world_replay::*;
use preserves::IOValue;

use super::CanonicalWorldReplayRecord;
use super::WORLD_REPLAY_IMPORT_RECEIPT_RECORD;
use super::WORLD_REPLAY_RECEIPT_RECORD;
use super::support::*;
use crate::error::MoltenError;
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
        return Err(MoltenError::invalid_harness("world replay receipt schema is invalid"));
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
        return Err(MoltenError::invalid_harness("world replay import receipt schema is invalid"));
    }
    validate_ref(&receipt.capsule_ref, "import capsule")?;
    if let Some(reference) = &receipt.availability_ref {
        validate_ref(reference, "capsule availability")?;
    }
    validate_diagnostics(&receipt.diagnostics)?;
    require_non_claims(&receipt.non_claims)?;
    if receipt.branch_moved || receipt.runtime_activated || receipt.authority_granted {
        return Err(MoltenError::invalid_harness("world replay import receipt claims forbidden mutation or authority"));
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
        field("universal-determinism-proved", boolean(false)),
        field("semantic-equivalence-proved", boolean(false)),
        field("capability-transferred", boolean(false)),
        field("effects-completed", boolean(false)),
        field("release-authorized", boolean(false)),
        non_claims_value(&receipt.non_claims),
    ]);
    record(WORLD_REPLAY_RECEIPT_RECORD, values)
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

const WORLD_REPLAY_RECEIPT_FIELD_CAPACITY: usize = 18;
const WORLD_REPLAY_IMPORT_RECEIPT_FIELD_CAPACITY: usize = 11;
