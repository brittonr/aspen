#[derive(Clone, Copy)]
struct BoundaryEvidence<'a> {
    suite: &'a super::model::Suite,
    policy_gate: &'a super::observed::PolicyGateEvidence,
    capability_gate: &'a super::observed::CapabilityGateEvidence,
    budget_gate: &'a super::observed::BudgetGateEvidence,
}

struct ExpectedBoundary<'a> {
    suite: &'a super::model::Suite,
    step: &'a crate::harness::nucleus::CoreStep,
    position: usize,
    observation: &'a super::observed::Observation,
    context: super::observed::HostcallEvidenceContext<'a>,
    admission: &'a super::observed::AdmissionDecisionEvent,
    authority: &'a super::observed::AdmissionAuthorityEvidence,
}

struct RuntimeBoundary<'a> {
    suite: &'a super::model::Suite,
    step: &'a crate::harness::nucleus::CoreStep,
    position: usize,
    observation: &'a super::observed::Observation,
    decision: &'a crate::runtime::AdmissionDecision,
    runtime_events: &'a [preserves::IOValue],
}

pub fn validate_hostcall_evidence(
    suite: &super::model::Suite,
    observations: &[super::observed::Observation],
    policy_gate: &super::observed::PolicyGateEvidence,
    capability_gate: &super::observed::CapabilityGateEvidence,
    budget_gate: &super::observed::BudgetGateEvidence,
) -> crate::error::Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "hostcall evidence observation count {} does not match suite step count {}",
            observations.len(),
            suite.steps.len()
        )));
    }
    let evidence = BoundaryEvidence {
        suite,
        policy_gate,
        capability_gate,
        budget_gate,
    };
    for (position, (step, observation)) in suite.steps.iter().zip(observations.iter()).enumerate() {
        validate_boundary_observation(evidence, position, step, observation)?;
    }
    Ok(())
}

fn validate_boundary_observation(
    evidence: BoundaryEvidence<'_>,
    position: usize,
    step: &crate::harness::nucleus::CoreStep,
    observation: &super::observed::Observation,
) -> crate::error::Result<()> {
    let step_ref = validated_step_ref(position, step, observation)?;
    let (actor_output_index, actor_output_event) = actor_output_slot(position, observation)?;
    boundary_order(position, observation, actor_output_event)?;
    let admission = super::vetting::parse_admission_decision_event(&observation.events[0])?;
    let authority = admission.authority.as_ref().ok_or_else(|| {
        crate::error::Failure::invalid_harness(format!(
            "missing capability authority evidence for hostcall decision at observation {position}"
        ))
    })?;
    let suite_ref = super::intake::suite_ref(evidence.suite)?;
    let context = super::observed::HostcallEvidenceContext {
        sequence: position as u64,
        suite_ref: &suite_ref,
        step_ref: &step_ref,
        policy_ref: &evidence.policy_gate.policy_ref,
        capability_ref: &evidence.capability_gate.capability_ref,
        budget_ref: &evidence.budget_gate.budget_ref,
    };
    validate_expected_boundary(&ExpectedBoundary {
        suite: evidence.suite,
        step,
        position,
        observation,
        context,
        admission: &admission,
        authority,
    })?;
    let runtime_events = &observation.events[4..actor_output_index];
    validate_runtime_boundary(&RuntimeBoundary {
        suite: evidence.suite,
        step,
        position,
        observation,
        decision: &admission.decision,
        runtime_events,
    })?;
    let expected_output = super::calls::actor_output_value(step, context, &admission.decision, runtime_events)?;
    require_hostcall_event(position, "actor-output", actor_output_event, &expected_output)
}

fn validated_step_ref(
    position: usize,
    step: &crate::harness::nucleus::CoreStep,
    observation: &super::observed::Observation,
) -> crate::error::Result<String> {
    let step_ref = crate::preserves_rail::canonical_hash(&super::events::step_value(step))?;
    if observation.step_ref != step_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "hostcall step ref mismatch at observation {position}"
        )));
    }
    Ok(step_ref)
}

fn actor_output_slot(
    position: usize,
    observation: &super::observed::Observation,
) -> crate::error::Result<(usize, &preserves::IOValue)> {
    if observation.events.len() < 5 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "missing executor hostcall boundary evidence at observation {position}"
        )));
    }
    let actor_output_index = if observation.events.last().is_some_and(super::vetting::is_turn_journal) {
        observation.events.len() - 2
    } else {
        observation.events.len() - 1
    };
    let Some(actor_output_event) = observation.events.as_slice().get(actor_output_index) else {
        return Err(crate::error::Failure::invalid_harness(format!(
            "missing final actor output at observation {position}"
        )));
    };
    Ok((actor_output_index, actor_output_event))
}

fn boundary_order(
    position: usize,
    observation: &super::observed::Observation,
    actor_output_event: &preserves::IOValue,
) -> crate::error::Result<()> {
    if super::tape::event_boundary(&observation.events[1]) != super::observed::EventBoundary::ActorInput
        || super::tape::event_boundary(&observation.events[2]) != super::observed::EventBoundary::HostcallRequest
        || super::tape::event_boundary(&observation.events[3]) != super::observed::EventBoundary::HostcallDecision
        || super::tape::event_boundary(actor_output_event) != super::observed::EventBoundary::ActorOutput
    {
        return Err(crate::error::Failure::invalid_harness(format!(
            "executor hostcall boundary order mismatch at observation {position}"
        )));
    }
    Ok(())
}

fn validate_expected_boundary(input: &ExpectedBoundary<'_>) -> crate::error::Result<()> {
    let expected_input = super::calls::actor_input_value(input.suite, input.step, input.context)?;
    require_hostcall_event(input.position, "actor-input", &input.observation.events[1], &expected_input)?;
    let expected_request =
        super::calls::hostcall_request_value(input.suite, input.step, input.context, &input.admission.decision)?;
    require_hostcall_event(input.position, "hostcall-request", &input.observation.events[2], &expected_request)?;
    let expected_decision = super::calls::hostcall_decision_value(
        input.context,
        &input.observation.events[0],
        input.authority,
        &input.admission.decision,
    )?;
    require_hostcall_event(input.position, "hostcall-decision", &input.observation.events[3], &expected_decision)
}

fn validate_runtime_boundary(input: &RuntimeBoundary<'_>) -> crate::error::Result<()> {
    super::scheme::validate_steel_execution_evidence(
        input.suite,
        input.step,
        input.position,
        input.decision,
        input.runtime_events,
    )?;
    super::bytecode::validate_wasm_execution_evidence(&super::bytecode::WasmExecutionEvidenceInput {
        suite: input.suite,
        step: input.step,
        position: input.position,
        decision: input.decision,
        actor_input: &input.observation.events[1],
        runtime_events: input.runtime_events,
    })
}

fn require_hostcall_event(
    position: usize,
    kind: &str,
    actual: &preserves::IOValue,
    expected: &preserves::IOValue,
) -> crate::error::Result<()> {
    if actual == expected {
        return Ok(());
    }
    Err(crate::error::Failure::invalid_harness(format!(
        "{kind} evidence mismatch at observation {position}: got {}, expected {}",
        crate::preserves_rail::canonical_hash(actual)?,
        crate::preserves_rail::canonical_hash(expected)?
    )))
}
