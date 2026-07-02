
fn require_steel_execution_checks(checks: &[String], arity: usize) -> Result<()> {
    require_executor_preflight_check(checks, "steel-vm-executed")?;
    require_executor_preflight_check(checks, "reviewed-callable-binding")?;
    require_executor_preflight_check(checks, "canonical-preserves-input")?;
    require_executor_preflight_check(checks, "canonical-preserves-output")?;
    require_executor_preflight_check(checks, "no-ambient-steel-io")?;
    require_executor_preflight_check(checks, "hostcall-envelope-binding")?;
    if arity == 10 {
        require_executor_preflight_check(checks, "resource-bounded")?;
        require_executor_preflight_check(checks, "fuel-bounded")?;
        require_executor_preflight_check(checks, "hostcall-count-bounded")?;
        require_executor_preflight_check(checks, "io-bytes-bounded")?;
    }
    Ok(())
}

fn validate_wasm_execution_evidence(input: &WasmExecutionEvidenceInput<'_>) -> Result<()> {
    let actor = input.step.primary_actor();
    let decl = actor_decl_for_primary_actor(input.suite, actor)?;
    if decl.kind != ActorKind::Wasm {
        return Ok(());
    }
    if !input.decision.is_allowed() {
        if input.runtime_events.iter().any(|event| event_boundary(event) == EventBoundary::WasmExecution) {
            return Err(MoltenError::invalid_harness(format!(
                "denied Wasm step at observation {} must not carry Wasm execution evidence",
                input.position
            )));
        }
        return Ok(());
    }
    let Some(receipt) = input.runtime_events.first() else {
        return Err(MoltenError::invalid_harness(format!(
            "missing Wasm execution evidence at observation {}",
            input.position
        )));
    };
    validate_wasm_execution_receipt(decl, input.step, input.position, input.actor_input, receipt)
}

fn validate_wasm_execution_receipt(
    actor: &ActorDecl,
    step: &super::core::CoreStep,
    position: usize,
    actor_input: &IoValue,
    value: &IoValue,
) -> Result<()> {
    let receipt_value = value
        .collect_simple_record("wasm-execution-receipt-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <wasm-execution-receipt-v1 ...>"))?;
    let arity = receipt_value.fields_iter().count();
    if arity != 9 && arity != 13 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <wasm-execution-receipt-v1 ...> with arity 9 or 13, got {arity}"
        )));
    }
    let receipt = &receipt_value;
    let schema = required_string(&receipt[0], "Wasm execution receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Wasm execution receipt schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA
        )));
    }
    let actor_id = required_record_string(&receipt[1], "actor", "Wasm execution actor")?;
    if actor_id != actor.id {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution actor mismatch at observation {position}: got {actor_id}, expected {}",
            actor.id
        )));
    }
    let ActorExecutorConfig::Wasm(config) = actor
        .executor
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness(format!("Wasm actor {} missing Wasm executor config", actor.id)))?
    else {
        return Err(MoltenError::invalid_harness(format!("Wasm actor {} has non-Wasm executor config", actor.id)));
    };
    let module_ref = required_record_hash(&receipt[2], "module-ref", "Wasm execution module ref")?;
    if module_ref != wasm_module_ref(config)? {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution module ref mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let operation = super::core::AdmissionRequest::from_step(step).action.as_str().to_string();
    let expected_export = wasm_executor_export_name(&operation);
    let export = required_record_string(&receipt[3], "export", "Wasm execution export")?;
    if export != expected_export {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution export mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let receipt_operation = required_record_string(&receipt[4], "operation", "Wasm execution operation")?;
    if receipt_operation != operation {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution operation mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let hostcalls = required_record_string_sequence(&receipt[5], "hostcalls", "Wasm execution hostcalls")?;
    if hostcalls != vec![operation] {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution hostcalls mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let checks_index = wasm_execution_checks_index(actor, position, actor_input, receipt, arity)?;
    let checks = parse_executor_preflight_checks(&receipt[checks_index])?;
    require_wasm_execution_checks(&checks, arity)
}

fn wasm_execution_checks_index(
    actor: &ActorDecl,
    position: usize,
    actor_input: &IoValue,
    receipt: &Record<Value<IoValue>>,
    arity: usize,
) -> Result<usize> {
    validate_wasm_execution_resources(receipt)?;
    if arity == 13 {
        validate_wasm_abi_fields(actor, position, actor_input, receipt)?;
        Ok(12)
    } else {
        Ok(8)
    }
}

fn validate_wasm_execution_resources(receipt: &Record<Value<IoValue>>) -> Result<()> {
    let fuel_value = value_to_iovalue(&receipt[6]);
    let fuel = simple_record(&fuel_value, "fuel", 2)?;
    let fuel_limit = required_u64(&fuel[0], "Wasm execution fuel limit")?;
    let fuel_remaining = required_u64(&fuel[1], "Wasm execution fuel remaining")?;
    if fuel_remaining > fuel_limit {
        return Err(MoltenError::invalid_harness("Wasm execution remaining fuel exceeds limit"));
    }
    let memory_value = value_to_iovalue(&receipt[7]);
    let memory = simple_record(&memory_value, "memory-limit", 1)?;
    required_u64(&memory[0], "Wasm execution memory limit")?;
    Ok(())
}

fn validate_wasm_abi_fields(
    actor: &ActorDecl,
    position: usize,
    actor_input: &IoValue,
    receipt: &Record<Value<IoValue>>,
) -> Result<()> {
    let abi = required_record_string(&receipt[8], "abi", "Wasm execution ABI schema")?;
    if abi != crate::preserves_rail::RUNTIME_WASM_ABI_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Wasm execution ABI schema {abi}; expected {}",
            crate::preserves_rail::RUNTIME_WASM_ABI_SCHEMA
        )));
    }
    let input_ref = required_record_hash(&receipt[9], "input-ref", "Wasm execution input ref")?;
    let expected_input_ref = canonical_hash(actor_input)?;
    if input_ref != expected_input_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution input ref mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    required_record_hash(&receipt[10], "output-ref", "Wasm execution output ref")?;
    let output_bytes = required_record_u64(&receipt[11], "output-bytes", "Wasm execution output byte count")?;
    if output_bytes > WASM_ABI_MAX_OUTPUT_BYTES_FOR_VALIDATION {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm execution output byte count exceeds molten.wasm.abi.v1 limit for actor {} at observation {position}",
            actor.id
        )));
    }
    Ok(())
}

fn require_wasm_execution_checks(checks: &[String], arity: usize) -> Result<()> {
    if arity == 13 {
        require_executor_preflight_check(checks, "preserves-abi-v1")?;
        require_executor_preflight_check(checks, "canonical-preserves-input")?;
        require_executor_preflight_check(checks, "canonical-preserves-output")?;
        require_executor_preflight_check(checks, "guest-memory-bounds")?;
    }
    require_executor_preflight_check(checks, "wasmtime-instantiated")?;
    require_executor_preflight_check(checks, "no-wasi")?;
    require_executor_preflight_check(checks, "fuel-bounded")?;
    require_executor_preflight_check(checks, "memory-bounded")?;
    require_executor_preflight_check(checks, "hostcall-envelope-binding")?;
    Ok(())
}

fn require_hostcall_event(position: usize, kind: &str, actual: &IoValue, expected: &IoValue) -> Result<()> {
    if actual == expected {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!(
        "{kind} evidence mismatch at observation {position}: got {}, expected {}",
        canonical_hash(actual)?,
        canonical_hash(expected)?
    )))
}

pub fn parse_admission_decision_event(value: &IoValue) -> Result<AdmissionDecisionEvent> {
    let admission = value
        .collect_simple_record("admission-decision-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <admission-decision-v1 ...>"))?;
    let arity = admission.fields_iter().count();
    if arity != 3 && arity != 4 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <admission-decision-v1 ...> with arity 3 or 4, got {arity}"
        )));
    }
    let schema = required_string(&admission[0], "admission decision schema")?;
    if schema != crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported admission decision schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA
        )));
    }
    let request = parse_admission_request(&admission[1])?;
    let (authority, decision_index) = if arity == 4 {
        (Some(parse_admission_authority(&admission[2])?), 3)
    } else {
        (None, 2)
    };
    let decision = parse_admission_decision(&admission[decision_index])?;
    Ok(AdmissionDecisionEvent {
        value: value.clone(),
        request,
        authority,
        decision,
    })
}

pub fn report_suite_value(report_value: &IoValue) -> Result<IoValue> {
    Ok(parse_report(report_value)?.suite_value)
}
