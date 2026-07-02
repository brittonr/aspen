
struct ExpectedBoundary<'a> {
    suite: &'a Suite,
    step: &'a super::core::CoreStep,
    position: usize,
    observation: &'a Observation,
    context: HostcallEvidenceContext<'a>,
    admission: &'a AdmissionDecisionEvent,
    authority: &'a AdmissionAuthorityEvidence,
}

struct RuntimeBoundary<'a> {
    suite: &'a Suite,
    step: &'a super::core::CoreStep,
    position: usize,
    observation: &'a Observation,
    decision: &'a crate::runtime::AdmissionDecision,
    runtime_events: &'a [IoValue],
}

pub fn validate_hostcall_evidence(
    suite: &Suite,
    observations: &[Observation],
    policy_gate: &PolicyGateEvidence,
    capability_gate: &CapabilityGateEvidence,
    budget_gate: &BudgetGateEvidence,
) -> Result<()> {
    if observations.len() != suite.steps.len() {
        return Err(MoltenError::invalid_harness(format!(
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
    step: &super::core::CoreStep,
    observation: &Observation,
) -> Result<()> {
    let step_ref = validated_step_ref(position, step, observation)?;
    let (actor_output_index, actor_output_event) = actor_output_slot(position, observation)?;
    boundary_order(position, observation, actor_output_event)?;
    let admission = parse_admission_decision_event(&observation.events[0])?;
    let authority = admission.authority.as_ref().ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "missing capability authority evidence for hostcall decision at observation {position}"
        ))
    })?;
    let suite_ref = suite_ref(evidence.suite)?;
    let context = HostcallEvidenceContext {
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
    let expected_output = actor_output_value(step, context, &admission.decision, runtime_events)?;
    require_hostcall_event(position, "actor-output", actor_output_event, &expected_output)
}

fn validated_step_ref(position: usize, step: &super::core::CoreStep, observation: &Observation) -> Result<String> {
    let step_ref = canonical_hash(&step_value(step))?;
    if observation.step_ref != step_ref {
        return Err(MoltenError::invalid_harness(format!("hostcall step ref mismatch at observation {position}")));
    }
    Ok(step_ref)
}

fn actor_output_slot(position: usize, observation: &Observation) -> Result<(usize, &IoValue)> {
    if observation.events.len() < 5 {
        return Err(MoltenError::invalid_harness(format!(
            "missing executor hostcall boundary evidence at observation {position}"
        )));
    }
    let actor_output_index = if observation.events.last().is_some_and(is_turn_journal) {
        observation.events.len() - 2
    } else {
        observation.events.len() - 1
    };
    let Some(actor_output_event) = observation.events.as_slice().get(actor_output_index) else {
        return Err(MoltenError::invalid_harness(format!("missing final actor output at observation {position}")));
    };
    Ok((actor_output_index, actor_output_event))
}

fn boundary_order(position: usize, observation: &Observation, actor_output_event: &IoValue) -> Result<()> {
    if event_boundary(&observation.events[1]) != EventBoundary::ActorInput
        || event_boundary(&observation.events[2]) != EventBoundary::HostcallRequest
        || event_boundary(&observation.events[3]) != EventBoundary::HostcallDecision
        || event_boundary(actor_output_event) != EventBoundary::ActorOutput
    {
        return Err(MoltenError::invalid_harness(format!(
            "executor hostcall boundary order mismatch at observation {position}"
        )));
    }
    Ok(())
}

fn validate_expected_boundary(input: &ExpectedBoundary<'_>) -> Result<()> {
    let expected_input = actor_input_value(input.suite, input.step, input.context)?;
    require_hostcall_event(input.position, "actor-input", &input.observation.events[1], &expected_input)?;
    let expected_request = hostcall_request_value(input.suite, input.step, input.context, &input.admission.decision)?;
    require_hostcall_event(input.position, "hostcall-request", &input.observation.events[2], &expected_request)?;
    let expected_decision = hostcall_decision_value(
        input.context,
        &input.observation.events[0],
        input.authority,
        &input.admission.decision,
    )?;
    require_hostcall_event(input.position, "hostcall-decision", &input.observation.events[3], &expected_decision)
}

fn validate_runtime_boundary(input: &RuntimeBoundary<'_>) -> Result<()> {
    validate_steel_execution_evidence(input.suite, input.step, input.position, input.decision, input.runtime_events)?;
    validate_wasm_execution_evidence(&WasmExecutionEvidenceInput {
        suite: input.suite,
        step: input.step,
        position: input.position,
        decision: input.decision,
        actor_input: &input.observation.events[1],
        runtime_events: input.runtime_events,
    })
}

struct WasmExecutionEvidenceInput<'a> {
    suite: &'a Suite,
    step: &'a super::core::CoreStep,
    position: usize,
    decision: &'a crate::runtime::AdmissionDecision,
    actor_input: &'a IoValue,
    runtime_events: &'a [IoValue],
}

fn validate_steel_execution_evidence(
    suite: &Suite,
    step: &super::core::CoreStep,
    position: usize,
    decision: &crate::runtime::AdmissionDecision,
    runtime_events: &[IoValue],
) -> Result<()> {
    let actor = step.primary_actor();
    let decl = actor_decl_for_primary_actor(suite, actor)?;
    if decl.kind != ActorKind::Steel {
        return Ok(());
    }
    if !decision.is_allowed() {
        if runtime_events.iter().any(|event| event_boundary(event) == EventBoundary::SteelExecution) {
            return Err(MoltenError::invalid_harness(format!(
                "denied Steel step at observation {position} must not carry Steel execution evidence"
            )));
        }
        return Ok(());
    }
    let Some(receipt) = runtime_events.first() else {
        return Err(MoltenError::invalid_harness(format!(
            "missing Steel execution evidence at observation {position}"
        )));
    };
    validate_steel_execution_receipt(decl, step, position, receipt)
}

fn validate_steel_execution_receipt(
    actor: &ActorDecl,
    step: &super::core::CoreStep,
    position: usize,
    value: &IoValue,
) -> Result<()> {
    let receipt_value = value
        .collect_simple_record("steel-execution-receipt-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <steel-execution-receipt-v1 ...>"))?;
    let arity = receipt_value.fields_iter().count();
    if arity != 9 && arity != 10 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <steel-execution-receipt-v1 ...> with arity 9 or 10, got {arity}"
        )));
    }
    let receipt = &receipt_value;
    let schema = required_string(&receipt[0], "Steel execution receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Steel execution receipt schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA
        )));
    }
    let actor_id = required_record_string(&receipt[1], "actor", "Steel execution actor")?;
    if actor_id != actor.id {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution actor mismatch at observation {position}: got {actor_id}, expected {}",
            actor.id
        )));
    }
    let ActorExecutorConfig::Steel(config) = actor.executor.as_ref().ok_or_else(|| {
        MoltenError::invalid_harness(format!("Steel actor {} missing Steel executor config", actor.id))
    })?
    else {
        return Err(MoltenError::invalid_harness(format!("Steel actor {} has non-Steel executor config", actor.id)));
    };
    let source_ref = required_record_hash(&receipt[2], "source-ref", "Steel execution source ref")?;
    if source_ref != steel_source_ref(config)? {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution source ref mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let callable = required_record_string(&receipt[3], "callable", "Steel execution callable")?;
    if callable != config.callable {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution callable mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let operation = super::core::AdmissionRequest::from_step(step).action.as_str().to_string();
    let receipt_operation = required_record_string(&receipt[4], "operation", "Steel execution operation")?;
    if receipt_operation != operation {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution operation mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    required_record_hash(&receipt[5], "input-ref", "Steel execution input ref")?;
    required_record_hash(&receipt[6], "output-ref", "Steel execution output ref")?;
    let hostcalls = required_record_string_sequence(&receipt[7], "hostcalls", "Steel execution hostcalls")?;
    if hostcalls != vec![operation] {
        return Err(MoltenError::invalid_harness(format!(
            "Steel execution hostcalls mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let checks_index = steel_execution_checks_index(receipt, arity)?;
    let checks = parse_executor_preflight_checks(&receipt[checks_index])?;
    require_steel_execution_checks(&checks, arity)
}

fn steel_execution_checks_index(receipt: &Record<Value<IoValue>>, arity: usize) -> Result<usize> {
    if arity == 10 {
        validate_steel_execution_resources(&receipt[8])?;
        Ok(9)
    } else {
        Ok(8)
    }
}

fn validate_steel_execution_resources(value: &Value<IoValue>) -> Result<()> {
    let resources_value = value_to_iovalue(value);
    let resources = simple_record(&resources_value, "resources", 5)?;
    let fuel_value = value_to_iovalue(&resources[0]);
    let fuel = simple_record(&fuel_value, "fuel", 2)?;
    let fuel_limit = required_u64(&fuel[0], "Steel execution fuel limit")?;
    let fuel_remaining = required_u64(&fuel[1], "Steel execution fuel remaining")?;
    if fuel_remaining > fuel_limit {
        return Err(MoltenError::invalid_harness("Steel execution remaining fuel exceeds limit"));
    }
    required_record_u64(&resources[1], "source-bytes", "Steel execution source byte count")?;
    required_record_u64(&resources[2], "input-bytes", "Steel execution input byte count")?;
    required_record_u64(&resources[3], "output-bytes", "Steel execution output byte count")?;
    let hostcalls_value = value_to_iovalue(&resources[4]);
    let hostcalls_record = simple_record(&hostcalls_value, "hostcalls", 2)?;
    let hostcall_limit = required_u64(&hostcalls_record[0], "Steel execution hostcall limit")?;
    let hostcall_count = required_u64(&hostcalls_record[1], "Steel execution hostcall count")?;
    if hostcall_count > hostcall_limit {
        return Err(MoltenError::invalid_harness("Steel execution hostcall count exceeds limit"));
    }
    Ok(())
}
