use preserves::ValueImpl;

const RUNTIME_PREDICATE_ENGINE: &str = "trellis-bounded-local";

const TURN_COMMIT_ROLLBACK_PREDICATE: &str = "molten.trellis-runtime.turn-commit-rollback.v1";

const ASSERTION_VISIBILITY_PREDICATE: &str = "molten.trellis-runtime.assertion-visibility.v1";

const OBSERVE_DELIVERY_PREDICATE: &str = "molten.trellis-runtime.observe-delivery.v1";

const PRESERVES_PATTERN_PREDICATE: &str = "molten.trellis-runtime.preserves-pattern.v1";

const PROMISE_STATE_PREDICATE: &str = "molten.trellis-runtime.promise-state.v1";

const PROMISE_PIPELINE_PREDICATE: &str = "molten.trellis-runtime.promise-pipeline.v1";

const REVOCATION_CLEANUP_PREDICATE: &str = "molten.trellis-runtime.revocation-cleanup.v1";

const ACTORMAP_TRANSACTION_PREDICATE: &str = "molten.trellis-runtime.actormap-transaction.v1";

const NEAR_FAR_REFS_PREDICATE: &str = "molten.trellis-runtime.near-far-refs.v1";

const SNAPSHOT_AUTHORITY_PREDICATE: &str = "molten.trellis-runtime.snapshot-authority.v1";

const SERVICE_DEPENDENCIES_PREDICATE: &str = "molten.trellis-runtime.service-dependencies.v1";

pub fn validate_runtime_predicate_evidence(
    suite: &super::model::Suite,
    observations: &[super::observed::Observation],
) -> crate::error::Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "runtime predicate observation count {} does not match suite step count {}",
            observations.len(),
            suite.steps.len()
        )));
    }

    for (position, (step, observation)) in suite.steps.iter().zip(observations.iter()).enumerate() {
        let admission = observation
            .events
            .first()
            .ok_or_else(|| {
                crate::error::Failure::invalid_harness(format!("missing admission decision at observation {position}"))
            })
            .and_then(super::vetting::parse_admission_decision_event)?;
        let mut runtime_predicates = Vec::with_capacity(observation.events.as_slice().len());
        for event in observation.events.as_slice() {
            if super::tape::event_boundary(event) == super::observed::EventBoundary::RuntimePredicate {
                runtime_predicates.push(parse_runtime_predicate_receipt(event)?);
            }
        }
        let expected = expected_runtime_predicates(step, &admission.decision);
        for predicate in &runtime_predicates {
            if !expected.as_slice().iter().any(|expected_predicate| expected_predicate == &predicate.as_str()) {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "unexpected runtime predicate {predicate} at observation {position}"
                )));
            }
        }
        for expected_predicate in expected {
            let count = runtime_predicates
                .as_slice()
                .iter()
                .filter(|predicate| predicate.as_str() == expected_predicate)
                .count();
            if count != 1 {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "runtime predicate {expected_predicate} at observation {position} expected exactly one receipt, got {count}"
                )));
            }
        }
    }
    Ok(())
}

fn expected_runtime_predicates(
    step: &crate::harness::nucleus::CoreStep,
    decision: &crate::runtime::AdmissionDecision,
) -> Vec<&'static str> {
    let mut expected = Vec::with_capacity(2);
    if !decision.is_allowed()
        || matches!(
            step,
            crate::harness::nucleus::CoreStep::Send { .. }
                | crate::harness::nucleus::CoreStep::Observe { .. }
                | crate::harness::nucleus::CoreStep::Assert { .. }
                | crate::harness::nucleus::CoreStep::Retract { .. }
        )
    {
        expected.push(TURN_COMMIT_ROLLBACK_PREDICATE);
    }
    match step {
        crate::harness::nucleus::CoreStep::Observe { .. } => expected.push(OBSERVE_DELIVERY_PREDICATE),
        crate::harness::nucleus::CoreStep::Assert { .. } | crate::harness::nucleus::CoreStep::Retract { .. } => {
            expected.push(ASSERTION_VISIBILITY_PREDICATE)
        }
        crate::harness::nucleus::CoreStep::Send { .. }
        | crate::harness::nucleus::CoreStep::Clock { .. }
        | crate::harness::nucleus::CoreStep::Random { .. } => {}
    }
    expected
}

fn parse_runtime_predicate_receipt(value: &preserves::IOValue) -> crate::error::Result<String> {
    let receipt = value
        .collect_simple_record("runtime-predicate-receipt-v1", Some(8))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <runtime-predicate-receipt-v1 ...>"))?;
    let schema = super::fields::required_string(&receipt[0], "runtime predicate receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_PREDICATE_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported runtime predicate receipt schema {schema}"
        )));
    }
    let predicate = super::fields::required_string(&receipt[1], "runtime predicate name")?;
    if !matches!(
        predicate.as_str(),
        TURN_COMMIT_ROLLBACK_PREDICATE
            | ASSERTION_VISIBILITY_PREDICATE
            | OBSERVE_DELIVERY_PREDICATE
            | PRESERVES_PATTERN_PREDICATE
            | PROMISE_STATE_PREDICATE
            | PROMISE_PIPELINE_PREDICATE
            | REVOCATION_CLEANUP_PREDICATE
            | ACTORMAP_TRANSACTION_PREDICATE
            | NEAR_FAR_REFS_PREDICATE
            | SNAPSHOT_AUTHORITY_PREDICATE
            | SERVICE_DEPENDENCIES_PREDICATE
    ) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported runtime predicate receipt predicate {predicate}"
        )));
    }
    let engine = super::fields::required_string(&receipt[2], "runtime predicate engine")?;
    if engine != RUNTIME_PREDICATE_ENGINE {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported runtime predicate engine {engine}")));
    }
    super::fields::required_record_hash(&receipt[3], "input-ref", "runtime predicate input ref")?;
    let decision = super::fields::required_string(&receipt[4], "runtime predicate decision")?;
    if !matches!(decision.as_str(), "pass" | "deny") {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported runtime predicate decision {decision}"
        )));
    }
    let state_refs = super::fields::sequence_strings(&receipt[5], "runtime predicate state refs")?;
    if state_refs.is_empty() {
        return Err(crate::error::Failure::invalid_harness("runtime predicate receipt missing state refs"));
    }
    for state_ref in &state_refs {
        crate::preserves_rail::validate_content_ref(state_ref)?;
    }
    let checks = super::fields::sequence_strings(&receipt[6], "runtime predicate checks")?;
    if checks.is_empty() {
        return Err(crate::error::Failure::invalid_harness("runtime predicate receipt missing checks"));
    }
    super::fields::sequence_strings(&receipt[7], "runtime predicate diagnostics")?;
    Ok(predicate)
}
