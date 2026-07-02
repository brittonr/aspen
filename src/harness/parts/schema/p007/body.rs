
fn nickel_budget_source(budget: &Budget, budget_ref: &str) -> String {
    format!(
        "{{\n  schema_version = {},\n  budget_schema = {},\n  budget_ref = {},\n  limits = {{\n    max_steps = {},\n    max_effects = {},\n    max_events = {},\n    max_report_bytes = {},\n  }},\n}}",
        nickel_string(crate::preserves_rail::HARNESS_BUDGET_NICKEL_STATIC_SCHEMA),
        nickel_string(crate::preserves_rail::HARNESS_BUDGET_SCHEMA),
        nickel_string(budget_ref),
        budget.max_steps,
        budget.max_effects,
        budget.max_events,
        budget.max_report_bytes,
    )
}

fn budget_gate_checks_value() -> IoValue {
    record("checks", vec![sequence(
        [
            "budget-schema",
            "canonical-budget-snapshot",
            "explicit-budget-fixture",
            "no-default-resource-policy",
            "resource-policy-preflight",
            "nickel-resource-policy",
            "nickel-resource-export",
            "basalt-resource-preflight",
            "basalt-resource-receipt",
            "budget-usage-binding",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn parse_budget_gate_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "budget gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(&check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "budget gate check name")?;
        let status = required_string(&check[1], "budget gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("budget gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_budget_gate_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("budget gate missing {expected} check")))
    }
}

pub fn validate_actor_registry_evidence(suite: &Suite, observations: &[Observation]) -> Result<()> {
    if !suite.actors_explicit {
        return Err(MoltenError::invalid_harness(
            "missing explicit actor registry fixture; inferred actors cannot satisfy evidence gates",
        ));
    }
    let actor_ids = suite.actors.iter().map(|actor| actor.id.as_str()).collect::<OrderedSet<_>>();
    for step in &suite.steps {
        for actor in actor_ids_for_step(step) {
            require_declared_actor(&actor_ids, actor, "suite step", None)?;
        }
    }
    for (position, observation) in observations.iter().enumerate() {
        for event in &observation.events {
            for actor in actor_ids_for_event(event)? {
                require_declared_actor(&actor_ids, &actor, "observation event", Some(position))?;
            }
        }
    }
    Ok(())
}

pub fn validate_admission_evidence(
    suite: &Suite,
    observations: &[Observation],
    capability_gate: &CapabilityGateEvidence,
) -> Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(MoltenError::invalid_harness(format!(
            "admission evidence observation count {} does not match suite step count {}",
            observations.len(),
            suite.steps.len()
        )));
    }

    for (position, (step, observation)) in suite.steps.iter().zip(observations.iter()).enumerate() {
        if observation.events.is_empty() {
            return Err(MoltenError::invalid_harness(format!("missing admission decision at observation {position}")));
        }
        if event_boundary(&observation.events[0]) != EventBoundary::PolicyDecision {
            return Err(MoltenError::invalid_harness(format!(
                "missing admission decision at observation {position}; first event is not admission-decision-v1"
            )));
        }
        let mut decision_count = 0usize;
        for event in &observation.events {
            if event_boundary(event) == EventBoundary::PolicyDecision {
                decision_count += 1;
            }
        }
        if decision_count != 1 {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate admission decision at observation {position}: got {decision_count} decisions"
            )));
        }

        let recorded = parse_admission_decision_event(&observation.events[0])?;
        let expected_request = super::core::AdmissionRequest::from_step(step);
        if recorded.request != expected_request {
            return Err(MoltenError::invalid_harness(format!("admission request mismatch at observation {position}")));
        }
        let expected_authority = admission_authority_evidence(&suite.capabilities, &expected_request)?;
        let recorded_authority = recorded.authority.as_ref().ok_or_else(|| {
            MoltenError::invalid_harness(format!("missing capability authority evidence at observation {position}"))
        })?;
        if recorded_authority != &expected_authority {
            return Err(MoltenError::invalid_harness(format!(
                "capability authority mismatch at observation {position}"
            )));
        }
        if recorded_authority.capability_ref != capability_gate.capability_ref {
            return Err(MoltenError::invalid_harness(format!(
                "capability authority preflight ref mismatch at observation {position}"
            )));
        }
        let preflight_grant_refs: &[String] = capability_gate.grant_refs.as_slice();
        if let Some(grant_ref) = recorded_authority.grant_ref.as_deref()
            && !preflight_grant_refs.iter().any(|preflight_ref| preflight_ref.as_str() == grant_ref)
        {
            return Err(MoltenError::invalid_harness(format!(
                "capability grant ref at observation {position} is not bound by authority preflight"
            )));
        }
        let expected_decision = suite.policy.decide_with_capabilities(&suite.capabilities, &expected_request);
        if recorded.decision != expected_decision {
            return Err(MoltenError::invalid_harness(format!("admission decision mismatch at observation {position}")));
        }
        if !recorded.decision.is_allowed() {
            validate_denied_observation_events(position, &observation.events[1..])?;
        }
    }
    Ok(())
}

pub fn validate_runtime_predicate_evidence(suite: &Suite, observations: &[Observation]) -> Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(MoltenError::invalid_harness(format!(
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
                MoltenError::invalid_harness(format!("missing admission decision at observation {position}"))
            })
            .and_then(parse_admission_decision_event)?;
        let mut runtime_predicates = Vec::with_capacity(observation.events.as_slice().len());
        for event in observation.events.as_slice() {
            if event_boundary(event) == EventBoundary::RuntimePredicate {
                runtime_predicates.push(parse_runtime_predicate_receipt(event)?);
            }
        }
        let expected = expected_runtime_predicates(step, &admission.decision);
        for predicate in &runtime_predicates {
            if !expected.as_slice().iter().any(|expected_predicate| expected_predicate == &predicate.as_str()) {
                return Err(MoltenError::invalid_harness(format!(
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
                return Err(MoltenError::invalid_harness(format!(
                    "runtime predicate {expected_predicate} at observation {position} expected exactly one receipt, got {count}"
                )));
            }
        }
    }
    Ok(())
}

fn expected_runtime_predicates(
    step: &super::core::CoreStep,
    decision: &crate::runtime::AdmissionDecision,
) -> Vec<&'static str> {
    let mut expected = Vec::with_capacity(2);
    if !decision.is_allowed()
        || matches!(
            step,
            super::core::CoreStep::Send { .. }
                | super::core::CoreStep::Observe { .. }
                | super::core::CoreStep::Assert { .. }
                | super::core::CoreStep::Retract { .. }
        )
    {
        expected.push(TURN_COMMIT_ROLLBACK_PREDICATE);
    }
    match step {
        super::core::CoreStep::Observe { .. } => expected.push(OBSERVE_DELIVERY_PREDICATE),
        super::core::CoreStep::Assert { .. } | super::core::CoreStep::Retract { .. } => {
            expected.push(ASSERTION_VISIBILITY_PREDICATE)
        }
        super::core::CoreStep::Send { .. }
        | super::core::CoreStep::Clock { .. }
        | super::core::CoreStep::Random { .. } => {}
    }
    expected
}

fn parse_runtime_predicate_receipt(value: &IoValue) -> Result<String> {
    let receipt = value
        .collect_simple_record("runtime-predicate-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <runtime-predicate-receipt-v1 ...>"))?;
    let schema = required_string(&receipt[0], "runtime predicate receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_PREDICATE_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate receipt schema {schema}")));
    }
    let predicate = required_string(&receipt[1], "runtime predicate name")?;
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
        return Err(MoltenError::invalid_harness(format!(
            "unsupported runtime predicate receipt predicate {predicate}"
        )));
    }
    let engine = required_string(&receipt[2], "runtime predicate engine")?;
    if engine != RUNTIME_PREDICATE_ENGINE {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate engine {engine}")));
    }
    required_record_hash(&receipt[3], "input-ref", "runtime predicate input ref")?;
    let decision = required_string(&receipt[4], "runtime predicate decision")?;
    if !matches!(decision.as_str(), "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!("unsupported runtime predicate decision {decision}")));
    }
    let state_refs = sequence_strings(&receipt[5], "runtime predicate state refs")?;
    if state_refs.is_empty() {
        return Err(MoltenError::invalid_harness("runtime predicate receipt missing state refs"));
    }
    for state_ref in &state_refs {
        validate_content_ref(state_ref)?;
    }
    let checks = sequence_strings(&receipt[6], "runtime predicate checks")?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("runtime predicate receipt missing checks"));
    }
    sequence_strings(&receipt[7], "runtime predicate diagnostics")?;
    Ok(predicate)
}

fn sequence_strings(value: &Value<IoValue>, field: &str) -> Result<Vec<String>> {
    let values = required_sequence(value, field)?;
    values.iter().map(|value| required_string(&value, field)).collect()
}

#[derive(Clone, Copy)]
struct BoundaryEvidence<'a> {
    suite: &'a Suite,
    policy_gate: &'a PolicyGateEvidence,
    capability_gate: &'a CapabilityGateEvidence,
    budget_gate: &'a BudgetGateEvidence,
}
