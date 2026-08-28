use molten_core::world_faults::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const WORLD_FAULT_RECEIPT_RECORD: &str = "molten-world-fault-conformance-receipt-v1";

const WORLD_FAULT_RECEIPT_CONTEXT: &str = "onixresearch.molten.world-fault-conformance.record.v1";
const MAX_WORLD_FAULT_RECEIPT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone)]
pub struct CanonicalWorldFaultReceipt {
    pub record_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

// r[impl molten.world_faults.receipt]
pub fn canonical_world_fault_receipt(
    receipt: &WorldFaultConformanceReceipt,
    profile: &WorldFaultProfile,
) -> Result<CanonicalWorldFaultReceipt> {
    let issues = validate_world_fault_receipt(receipt, profile);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("world fault receipt denied: {issues:?}")));
    }
    let value = record(WORLD_FAULT_RECEIPT_RECORD, vec![
        string(receipt.schema),
        field("source-revision", string(&receipt.source_revision)),
        field("inventory-ref", string(&receipt.inventory_ref)),
        field("profile-ref", string(&receipt.profile_ref)),
        field("adapter-refs", sequence(receipt.adapter_refs.iter().map(string).collect())),
        field("schedule-refs", sequence(receipt.schedule_refs.iter().map(string).collect())),
        limits_value(receipt.limits),
        field("results", sequence(receipt.results.iter().map(result_value).collect())),
        field("schedules", sequence(receipt.schedules.iter().map(schedule_result_value).collect())),
        field("unsupported-rows", sequence(receipt.unsupported_rows.iter().map(unsupported_value).collect())),
        field("decision", string(receipt.decision.as_str())),
        field("mutation-authorized-by-evidence", boolean(receipt.mutation_authorized_by_evidence)),
        field("cleanup-authorized-by-evidence", boolean(receipt.cleanup_authorized_by_evidence)),
        field(
            "non-claims",
            sequence(receipt.non_claims.iter().map(|non_claim| string(non_claim.as_str())).collect()),
        ),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_WORLD_FAULT_RECEIPT_BYTES {
        return Err(MoltenError::invalid_harness("world fault receipt exceeds its canonical byte bound"));
    }
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_FAULT_RECEIPT_CONTEXT);
    update(&mut hasher, WORLD_FAULT_RECEIPT_RECORD)?;
    let byte_length = u64::try_from(bytes.len())
        .map_err(|_| MoltenError::invalid_harness("world fault receipt length exceeds u64"))?;
    hasher.update(&byte_length.to_be_bytes());
    hasher.update(&bytes);
    Ok(CanonicalWorldFaultReceipt {
        record_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        value,
        bytes,
    })
}

fn limits_value(limits: WorldFaultLimits) -> IOValue {
    record("limits", vec![
        field("max-cases", usize_value(limits.max_cases)),
        field("max-schedules", usize_value(limits.max_schedules)),
        field("max-schedule-steps", usize_value(limits.max_schedule_steps)),
        field("max-adapters", usize_value(limits.max_adapters)),
        field("max-observations", usize_value(limits.max_observations)),
        field("max-unsupported-rows", usize_value(limits.max_unsupported_rows)),
        field("max-restarts", number(u64::from(limits.max_restarts))),
    ])
}

fn result_value(result: &WorldFaultConformanceResult) -> IOValue {
    record("case-result", vec![
        field("case-id", string(&result.case_id)),
        field("mutation", string(result.mutation.as_str())),
        field("phase", string(result.phase.as_str())),
        field("expected-decision", string(result.expected_decision.as_str())),
        field("observed-decision", string(result.observed_decision.as_str())),
        observation_value(&result.observation),
        field("disposition", string(result.disposition.as_str())),
        field("diagnostics", sequence(result.diagnostics.iter().map(|issue| string(issue_code(issue))).collect())),
    ])
}

fn observation_value(observation: &WorldOperationObservation) -> IOValue {
    record("observation", vec![
        field("operation-id", string(&observation.operation_id)),
        field("submission", string(submission_name(observation.submission))),
        field("response", string(response_name(observation.response))),
        field("read-back", read_back_value(&observation.read_back)),
        field("owner-decision", string(observation.owner_decision.as_str())),
        field("whole-store-rollback", boolean(observation.whole_store_rollback)),
        field("cleanup-authorized", boolean(observation.cleanup_authorized)),
    ])
}

fn read_back_value(read_back: &DurableReadBack) -> IOValue {
    record("durable-read-back", vec![
        field("status", string(read_back_name(read_back.status))),
        field("state-ref", optional_string(read_back.state_ref.as_deref())),
        field("record-ref", optional_string(read_back.record_ref.as_deref())),
        field("observed-generation", optional_number(read_back.observed_generation)),
        field("independent-witness", boolean(read_back.independent_witness)),
    ])
}

fn schedule_result_value(result: &ConcurrentScheduleResult) -> IOValue {
    record("schedule-result", vec![
        field("schedule-id", string(&result.schedule_id)),
        field("observations", sequence(result.observations.iter().map(concurrent_observation_value).collect())),
        field("scheduler-choices", sequence(result.scheduler_choices.iter().map(scheduler_choice_value).collect())),
        field("disposition", string(result.disposition.as_str())),
        field("diagnostics", sequence(result.diagnostics.iter().map(|issue| string(issue_code(issue))).collect())),
    ])
}

fn concurrent_observation_value(observation: &ConcurrentOperationObservation) -> IOValue {
    record("concurrent-observation", vec![
        field("operation-id", string(&observation.operation_id)),
        field("mutation", string(observation.mutation.as_str())),
        field("expected-generation", number(observation.expected_generation)),
        field("pre-state-ref", string(&observation.pre_state_ref)),
        field("outcome", string(concurrent_outcome_name(observation.outcome))),
        field("effect-release-count", number(u64::from(observation.effect_release_count))),
    ])
}

fn scheduler_choice_value(choice: &molten_core::fabric_simulation::EligibleChoice) -> IOValue {
    record("scheduler-choice", vec![
        field("kind", string(choice.kind.as_str())),
        field("choice-id", string(&choice.choice_id)),
        field("node-id", string(&choice.node_id)),
        field("generation", number(choice.generation)),
        field("ready-at-tick", number(choice.ready_at_tick)),
    ])
}

fn unsupported_value(row: &UnsupportedConformanceRow) -> IOValue {
    record("unsupported-row", vec![
        field("mutation", string(row.mutation.as_str())),
        field("case-id", string(&row.case_id)),
        field("reason", string(row.reason.as_str())),
    ])
}

fn submission_name(observation: SubmissionObservation) -> &'static str {
    match observation {
        SubmissionObservation::NotSubmitted => "not-submitted",
        SubmissionObservation::PossiblySubmitted => "possibly-submitted",
        SubmissionObservation::DurablySubmitted => "durably-submitted",
    }
}

fn response_name(observation: ResponseObservation) -> &'static str {
    match observation {
        ResponseObservation::NotExpected => "not-expected",
        ResponseObservation::Received => "received",
        ResponseObservation::Lost => "lost",
    }
}

fn read_back_name(status: DurableReadBackStatus) -> &'static str {
    match status {
        DurableReadBackStatus::Prior => "prior",
        DurableReadBackStatus::Applied => "applied",
        DurableReadBackStatus::Missing => "missing",
        DurableReadBackStatus::Corrupt => "corrupt",
        DurableReadBackStatus::Contradictory => "contradictory",
    }
}

fn concurrent_outcome_name(outcome: ConcurrentOutcome) -> &'static str {
    match outcome {
        ConcurrentOutcome::Applied => "applied",
        ConcurrentOutcome::AlreadyComplete => "already-complete",
        ConcurrentOutcome::Stale => "stale",
        ConcurrentOutcome::Superseded => "superseded",
        ConcurrentOutcome::Conflict => "conflict",
        ConcurrentOutcome::Uncertain => "uncertain",
        ConcurrentOutcome::Denied => "denied",
    }
}

fn issue_code(issue: &WorldFaultIssue) -> &'static str {
    match issue {
        WorldFaultIssue::SchemaMismatch(_) => "schema-mismatch",
        WorldFaultIssue::InventoryVersionMismatch => "inventory-version-mismatch",
        WorldFaultIssue::InventoryRowCount { .. } => "inventory-row-count",
        WorldFaultIssue::MissingMutation(_) => "missing-mutation",
        WorldFaultIssue::DuplicateMutation(_) => "duplicate-mutation",
        WorldFaultIssue::UnknownProductMutation(_) => "unknown-product-mutation",
        WorldFaultIssue::ProductMutationMissing(_) => "product-mutation-missing",
        WorldFaultIssue::InventoryContractMismatch(_) => "inventory-contract-mismatch",
        WorldFaultIssue::MissingRequiredPhase { .. } => "missing-required-phase",
        WorldFaultIssue::MissingRequiredFailureCase { .. } => "missing-required-failure-case",
        WorldFaultIssue::WitnessSupportOverclaim => "witness-support-overclaim",
        WorldFaultIssue::InvalidLimit(_) => "invalid-limit",
        WorldFaultIssue::BoundExceeded { .. } => "bound-exceeded",
        WorldFaultIssue::EmptyIdentifier(_) => "empty-identifier",
        WorldFaultIssue::MalformedReference { .. } => "malformed-reference",
        WorldFaultIssue::IdentityLengthOverflow(_) => "identity-length-overflow",
        WorldFaultIssue::DuplicateAdapter(_) => "duplicate-adapter",
        WorldFaultIssue::AdapterOwnerMissing(_) => "adapter-owner-missing",
        WorldFaultIssue::DuplicateCase(_) => "duplicate-case",
        WorldFaultIssue::CaseMutationMissing(_) => "case-mutation-missing",
        WorldFaultIssue::CaseAdapterMissing(_) => "case-adapter-missing",
        WorldFaultIssue::CasePhaseNotRequired { .. } => "case-phase-not-required",
        WorldFaultIssue::CaseExpectedDecisionMismatch { .. } => "case-expected-decision-mismatch",
        WorldFaultIssue::DuplicateSchedule(_) => "duplicate-schedule",
        WorldFaultIssue::ScheduleMutationMismatch(_) => "schedule-mutation-mismatch",
        WorldFaultIssue::SchedulePositionGap { .. } => "schedule-position-gap",
        WorldFaultIssue::ScheduleOperationMissing(_) => "schedule-operation-missing",
        WorldFaultIssue::ScheduleStepBoundExceeded { .. } => "schedule-step-bound-exceeded",
        WorldFaultIssue::ScheduleNodeGenerationZero(_) => "schedule-node-generation-zero",
        WorldFaultIssue::ObservationMissing(_) => "observation-missing",
        WorldFaultIssue::DuplicateObservation(_) => "duplicate-observation",
        WorldFaultIssue::ObservationOperationMismatch(_) => "observation-operation-mismatch",
        WorldFaultIssue::ObservationPhaseMismatch(_) => "observation-phase-mismatch",
        WorldFaultIssue::OwnerDecisionMismatch { .. } => "owner-decision-mismatch",
        WorldFaultIssue::SuccessWithoutDurableReadBack(_) => "success-without-durable-read-back",
        WorldFaultIssue::UnsafeRetryAfterPossibleSubmit(_) => "unsafe-retry-after-possible-submit",
        WorldFaultIssue::MissingStateBecameSuccess(_) => "missing-state-became-success",
        WorldFaultIssue::CorruptStateMisclassified(_) => "corrupt-state-misclassified",
        WorldFaultIssue::ContradictoryStateMisclassified(_) => "contradictory-state-misclassified",
        WorldFaultIssue::LocalRollbackDetectionOverclaim(_) => "local-rollback-detection-overclaim",
        WorldFaultIssue::UnsafeCleanupAuthority(_) => "unsafe-cleanup-authority",
        WorldFaultIssue::ConcurrentObservationMissing(_) => "concurrent-observation-missing",
        WorldFaultIssue::ConcurrentObservationUnexpected(_) => "concurrent-observation-unexpected",
        WorldFaultIssue::ConcurrentBindingMismatch(_) => "concurrent-binding-mismatch",
        WorldFaultIssue::MultipleLinearizations { .. } => "multiple-linearizations",
        WorldFaultIssue::DuplicateEffectRelease { .. } => "duplicate-effect-release",
        WorldFaultIssue::UnsupportedRowDropped(_) => "unsupported-row-dropped",
        WorldFaultIssue::ReceiptNonClaimMissing(_) => "receipt-non-claim-missing",
        WorldFaultIssue::EvidenceAuthorityOverclaim => "evidence-authority-overclaim",
    }
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| MoltenError::invalid_harness("world fault receipt identity field exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

fn optional_string(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_number(value: Option<u64>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![number(value)]))
}

fn usize_value(value: usize) -> IOValue {
    u64::try_from(value).map_or_else(|_| number(u64::MAX), number)
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}
