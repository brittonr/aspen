use molten_core::world_commit::WorldCommitRef;
use molten_core::world_promotion::*;
use preserves::IOValue;
use transactional_reconciliation_core::PersistenceState;
use transactional_reconciliation_core::QuarantineReason;
use transactional_reconciliation_core::QuarantineStatus;

use crate::error::MoltenError;
use crate::error::Result;

pub const WORLD_PROMOTION_PLAN_SCHEMA: &str = "molten.world-promotion-plan.v1";
pub const WORLD_RELEASE_RESERVATION_SCHEMA: &str = "molten.world-release-reservation.v1";
pub const WORLD_RELEASE_ATTEMPT_SCHEMA: &str = "molten.world-release-attempt.v1";
pub const WORLD_RELEASE_OBSERVATION_SCHEMA: &str = "molten.world-release-observation.v1";
pub const WORLD_PROMOTION_RECONCILIATION_SCHEMA: &str = "molten.world-promotion-reconciliation.v1";

const PROMOTION_PLAN_RECORD: &str = "world-promotion-plan";
const RESERVATION_RECORD: &str = "world-release-reservation";
const ATTEMPT_RECORD: &str = "world-release-attempt";
const OBSERVATION_RECORD: &str = "world-release-observation";
const RECONCILIATION_RECORD: &str = "world-promotion-reconciliation";
const RESERVATION_FIELD_COUNT: usize = 11;
const ATTEMPT_FIELD_COUNT: usize = 7;

#[derive(Debug, Clone)]
pub struct CanonicalWorldPromotionRecord {
    pub record_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_promotion_plan(plan: &WorldPromotionPlan) -> Result<CanonicalWorldPromotionRecord> {
    require_non_claims(&plan.non_claims)?;
    canonical(crate::preserves_rail::record(PROMOTION_PLAN_RECORD, vec![
        crate::preserves_rail::string(WORLD_PROMOTION_PLAN_SCHEMA),
        named("plan-ref", string(plan.plan_ref.as_str())),
        named("operation-ref", string(plan.operation_ref.as_str())),
        named("branch-id", string(plan.after.branch_id.as_str())),
        named("before-head", string(plan.before.head.as_str())),
        named("before-generation", number(plan.before.generation)),
        named("after-head", string(plan.after.head.as_str())),
        named("after-generation", number(plan.after.generation)),
        named(
            "reservations",
            sequence(
                plan.reservations.iter().map(|reservation| string(reservation.reservation_ref.as_str())).collect(),
            ),
        ),
        named("external-effects-completed", boolean(plan.external_effects_completed)),
        non_claims(),
    ]))
}

pub fn canonical_reservation(reservation: &WorldReleaseReservation) -> Result<CanonicalWorldPromotionRecord> {
    canonical(crate::preserves_rail::record(RESERVATION_RECORD, vec![
        crate::preserves_rail::string(WORLD_RELEASE_RESERVATION_SCHEMA),
        named("reservation-ref", string(reservation.reservation_ref.as_str())),
        named("promotion-ref", string(reservation.promotion_ref.as_str())),
        named("operation-ref", string(reservation.operation_ref.as_str())),
        named("candidate-head", string(reservation.candidate_head.as_str())),
        named("intent-ref", string(reservation.intent_ref.as_str())),
        named("semantic-ref", string(reservation.semantic_ref.as_str())),
        named("handler-ref", string(reservation.handler_ref.as_str())),
        named("adapter-ref", string(reservation.adapter_ref.as_str())),
        named("generation", number(reservation.generation)),
        named("state", string(reservation.state.as_str())),
    ]))
}

pub fn parse_reservation(bytes: &[u8]) -> Result<WorldReleaseReservation> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields =
        crate::preserves_rail::simple_record_fields(&decoded.value, RESERVATION_RECORD, RESERVATION_FIELD_COUNT)?;
    require_schema(&fields[0], WORLD_RELEASE_RESERVATION_SCHEMA)?;
    let reservation = WorldReleaseReservation {
        reservation_ref: WorldReleaseReservationRef::new(content_ref(&fields[1], "reservation-ref")?)
            .map_err(reference_error)?,
        promotion_ref: WorldPromotionPlanRef::new(content_ref(&fields[2], "promotion-ref")?)
            .map_err(reference_error)?,
        operation_ref: WorldPromotionOperationRef::new(content_ref(&fields[3], "operation-ref")?)
            .map_err(reference_error)?,
        candidate_head: WorldCommitRef::new(content_ref(&fields[4], "candidate-head")?).map_err(world_commit_error)?,
        intent_ref: WorldEffectIntentRef::new(content_ref(&fields[5], "intent-ref")?).map_err(reference_error)?,
        semantic_ref: WorldSemanticIntentRef::new(content_ref(&fields[6], "semantic-ref")?).map_err(reference_error)?,
        handler_ref: WorldPromotionHandlerRef::new(content_ref(&fields[7], "handler-ref")?).map_err(reference_error)?,
        adapter_ref: WorldPromotionAdapterRef::new(content_ref(&fields[8], "adapter-ref")?).map_err(reference_error)?,
        generation: u64_field(&fields[9], "generation")?,
        state: WorldReleaseState::parse(&string_field(&fields[10], "state")?)
            .ok_or_else(|| MoltenError::invalid_harness("unsupported world release state"))?,
    };
    let canonical = canonical_reservation(&reservation)?;
    if canonical.bytes != decoded.canonical_bytes {
        return Err(MoltenError::invalid_harness("world reservation bytes are not canonical"));
    }
    Ok(reservation)
}

pub fn canonical_attempt(record: &WorldAttemptRecord) -> Result<CanonicalWorldPromotionRecord> {
    canonical_attempt_record(ATTEMPT_RECORD, WORLD_RELEASE_ATTEMPT_SCHEMA, record)
}

pub fn canonical_observation(record: &WorldAttemptRecord) -> Result<CanonicalWorldPromotionRecord> {
    canonical_attempt_record(OBSERVATION_RECORD, WORLD_RELEASE_OBSERVATION_SCHEMA, record)
}

fn canonical_attempt_record(
    label: &'static str,
    schema: &'static str,
    record: &WorldAttemptRecord,
) -> Result<CanonicalWorldPromotionRecord> {
    canonical(crate::preserves_rail::record(label, vec![
        crate::preserves_rail::string(schema),
        named("reservation-ref", string(record.reservation_ref.as_str())),
        named("attempt-ref", string(record.attempt_ref.as_str())),
        named("state", string(record.state.as_str())),
        named("observation-ref", optional(record.observation_ref.as_ref().map(WorldReleaseObservationRef::as_str))),
        named("external-completion-proven", boolean(record.external_completion_proven)),
        non_claims(),
    ]))
}

pub fn parse_attempt(bytes: &[u8]) -> Result<WorldAttemptRecord> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(&decoded.value, ATTEMPT_RECORD, ATTEMPT_FIELD_COUNT)?;
    require_schema(&fields[0], WORLD_RELEASE_ATTEMPT_SCHEMA)?;
    let non_claim_field = named_value(&fields[6], "non-claims")?;
    let non_claim_values =
        crate::preserves_rail::required_sequence_field(&non_claim_field, "world promotion non-claims")?;
    let parsed_non_claims = non_claim_values
        .iter()
        .map(|value| crate::preserves_rail::required_string_field(value, "world promotion non-claim"))
        .collect::<Result<Vec<_>>>()?;
    require_non_claims(&parsed_non_claims)?;
    let record = WorldAttemptRecord {
        reservation_ref: WorldReleaseReservationRef::new(content_ref(&fields[1], "reservation-ref")?)
            .map_err(reference_error)?,
        attempt_ref: WorldReleaseAttemptRef::new(content_ref(&fields[2], "attempt-ref")?).map_err(reference_error)?,
        state: WorldReleaseState::parse(&string_field(&fields[3], "state")?)
            .ok_or_else(|| MoltenError::invalid_harness("unsupported world attempt state"))?,
        observation_ref: crate::preserves_rail::optional_content_ref_string(
            &named_value(&fields[4], "observation-ref")?,
            "world attempt observation ref",
        )?
        .map(WorldReleaseObservationRef::new)
        .transpose()
        .map_err(reference_error)?,
        external_completion_proven: bool_field(&fields[5], "external-completion-proven")?,
    };
    let canonical = canonical_attempt(&record)?;
    if canonical.bytes != decoded.canonical_bytes {
        return Err(MoltenError::invalid_harness("world attempt bytes are not canonical"));
    }
    Ok(record)
}

pub fn canonical_persistence(
    plan: &WorldPromotionPlan,
    persistence: &WorldPromotionPersistence,
) -> Result<CanonicalWorldPromotionRecord> {
    require_non_claims(&persistence.non_claims)?;
    canonical(crate::preserves_rail::record(RECONCILIATION_RECORD, vec![
        crate::preserves_rail::string(WORLD_PROMOTION_RECONCILIATION_SCHEMA),
        named("plan-ref", string(plan.plan_ref.as_str())),
        named("state", string(persistence_state(persistence.shared.state()))),
        named("quarantine", string(quarantine(persistence.shared.quarantine()))),
        named("dispatch-eligible", boolean(persistence.dispatch_eligible)),
        named("mutation-authorized-by-evidence", boolean(persistence.mutation_authorized_by_evidence)),
        non_claims(),
    ]))
}

fn canonical(value: IOValue) -> Result<CanonicalWorldPromotionRecord> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldPromotionRecord {
        record_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

fn require_non_claims(non_claims: &[String]) -> Result<()> {
    if non_claims != promotion_non_claims() {
        return Err(MoltenError::invalid_harness("world promotion non-claims are incomplete"));
    }
    Ok(())
}

fn require_schema(value: &preserves::Value<IOValue>, expected: &str) -> Result<()> {
    let actual = crate::preserves_rail::required_string_field(value, "world promotion schema")?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness("unsupported world promotion schema"))
    }
}

fn content_ref(value: &preserves::Value<IOValue>, label: &str) -> Result<String> {
    crate::preserves_rail::required_content_ref_string(&named_value(value, label)?, label)
}

fn string_field(value: &preserves::Value<IOValue>, label: &str) -> Result<String> {
    crate::preserves_rail::required_string_field(&named_value(value, label)?, label)
}

fn u64_field(value: &preserves::Value<IOValue>, label: &str) -> Result<u64> {
    named_value(value, label)?
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|_| MoltenError::invalid_harness(format!("u64 out of range for {label}")))
}

fn bool_field(value: &preserves::Value<IOValue>, label: &str) -> Result<bool> {
    let value = named_value(value, label)?;
    if value.collect_simple_record("true", Some(0)).is_some() {
        Ok(true)
    } else if value.collect_simple_record("false", Some(0)).is_some() {
        Ok(false)
    } else {
        Err(MoltenError::invalid_harness(format!("expected boolean for {label}")))
    }
}

fn named_value(value: &preserves::Value<IOValue>, label: &str) -> Result<preserves::Value<IOValue>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} VALUE>")))?;
    Ok(fields[0].clone())
}

fn persistence_state(state: PersistenceState) -> &'static str {
    match state {
        PersistenceState::NotPublished => "not-published",
        PersistenceState::Published => "published",
        PersistenceState::PublicationUnknown => "publication-unknown",
        PersistenceState::Conflicting => "conflicting",
    }
}

fn quarantine(status: QuarantineStatus) -> &'static str {
    match status {
        QuarantineStatus::Clear => "clear",
        QuarantineStatus::Quarantined(QuarantineReason::CommitOutcomeUnknown) => "commit-outcome-unknown",
        QuarantineStatus::Quarantined(QuarantineReason::RepairReported) => "repair-reported",
        QuarantineStatus::Quarantined(QuarantineReason::Corrupt) => "corrupt",
        QuarantineStatus::Quarantined(QuarantineReason::Missing) => "missing",
        QuarantineStatus::Quarantined(QuarantineReason::Inconsistent) => "inconsistent",
    }
}

fn named(label: &'static str, value: IOValue) -> IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn boolean(value: bool) -> IOValue {
    crate::preserves_rail::record(if value { "true" } else { "false" }, Vec::new())
}

fn optional(value: Option<&str>) -> IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![string(value)]),
    )
}

fn non_claims() -> IOValue {
    named("non-claims", sequence(WORLD_PROMOTION_NON_CLAIMS.iter().map(string).collect()))
}

fn reference_error(error: WorldPromotionReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world promotion reference: {error:?}"))
}

fn world_commit_error(error: molten_core::world_commit::WorldCommitReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world commit reference: {error:?}"))
}
