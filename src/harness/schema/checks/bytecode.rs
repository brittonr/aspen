use preserves::ValueImpl;

const WASM_ABI_MAX_OUTPUT_BYTES_FOR_VALIDATION: u64 = 8 * 1024;

pub(super) struct WasmExecutionEvidenceInput<'a> {
    pub(super) suite: &'a super::model::Suite,
    pub(super) step: &'a crate::harness::nucleus::CoreStep,
    pub(super) position: usize,
    pub(super) decision: &'a crate::runtime::AdmissionDecision,
    pub(super) actor_input: &'a preserves::IOValue,
    pub(super) runtime_events: &'a [preserves::IOValue],
}

pub(super) fn validate_wasm_execution_evidence(input: &WasmExecutionEvidenceInput<'_>) -> crate::error::Result<()> {
    let actor = input.step.primary_actor();
    let decl = super::intake::actor_decl_for_primary_actor(input.suite, actor)?;
    if decl.kind != super::model::ActorKind::Wasm {
        return Ok(());
    }
    if !input.decision.is_allowed() {
        if input
            .runtime_events
            .iter()
            .any(|event| super::tape::event_boundary(event) == super::observed::EventBoundary::WasmExecution)
        {
            return Err(crate::error::Failure::invalid_harness(format!(
                "denied Wasm step at observation {} must not carry Wasm execution evidence",
                input.position
            )));
        }
        return Ok(());
    }
    let Some(receipt) = input.runtime_events.first() else {
        return Err(crate::error::Failure::invalid_harness(format!(
            "missing Wasm execution evidence at observation {}",
            input.position
        )));
    };
    validate_wasm_execution_receipt(decl, input.step, input.position, input.actor_input, receipt)
}

fn validate_wasm_execution_receipt(
    actor: &super::model::ActorDecl,
    step: &crate::harness::nucleus::CoreStep,
    position: usize,
    actor_input: &preserves::IOValue,
    value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let receipt_value = value
        .collect_simple_record("wasm-execution-receipt-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <wasm-execution-receipt-v1 ...>"))?;
    let arity = receipt_value.fields_iter().count();
    if arity != 9 && arity != 13 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected <wasm-execution-receipt-v1 ...> with arity 9 or 13, got {arity}"
        )));
    }
    let receipt = &receipt_value;
    let schema = super::fields::required_string(&receipt[0], "Wasm execution receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Wasm execution receipt schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA
        )));
    }
    let actor_id = super::fields::required_record_string(&receipt[1], "actor", "Wasm execution actor")?;
    if actor_id != actor.id {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm execution actor mismatch at observation {position}: got {actor_id}, expected {}",
            actor.id
        )));
    }
    let super::model::ActorExecutorConfig::Wasm(config) = actor.executor.as_ref().ok_or_else(|| {
        crate::error::Failure::invalid_harness(format!("Wasm actor {} missing Wasm executor config", actor.id))
    })?
    else {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm actor {} has non-Wasm executor config",
            actor.id
        )));
    };
    let module_ref = super::fields::required_record_hash(&receipt[2], "module-ref", "Wasm execution module ref")?;
    if module_ref != super::introspect::wasm_module_ref(config)? {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm execution module ref mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let operation = crate::harness::nucleus::AdmissionRequest::from_step(step).action.as_str().to_string();
    let expected_export = super::introspect::wasm_executor_export_name(&operation);
    let export = super::fields::required_record_string(&receipt[3], "export", "Wasm execution export")?;
    if export != expected_export {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm execution export mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let receipt_operation =
        super::fields::required_record_string(&receipt[4], "operation", "Wasm execution operation")?;
    if receipt_operation != operation {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm execution operation mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let hostcalls =
        super::fields::required_record_string_sequence(&receipt[5], "hostcalls", "Wasm execution hostcalls")?;
    if hostcalls != vec![operation] {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm execution hostcalls mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let checks_index = wasm_execution_checks_index(actor, position, actor_input, receipt, arity)?;
    let checks = super::clearance::parse_executor_preflight_checks(&receipt[checks_index])?;
    require_wasm_execution_checks(&checks, arity)
}

fn wasm_execution_checks_index(
    actor: &super::model::ActorDecl,
    position: usize,
    actor_input: &preserves::IOValue,
    receipt: &preserves::Record<preserves::Value<preserves::IOValue>>,
    arity: usize,
) -> crate::error::Result<usize> {
    validate_wasm_execution_resources(receipt)?;
    if arity == 13 {
        validate_wasm_abi_fields(actor, position, actor_input, receipt)?;
        Ok(12)
    } else {
        Ok(8)
    }
}

fn validate_wasm_execution_resources(
    receipt: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<()> {
    let fuel_value = crate::preserves_rail::value_to_iovalue(&receipt[6]);
    let fuel = super::fields::simple_record(&fuel_value, "fuel", 2)?;
    let fuel_limit = super::fields::required_u64(&fuel[0], "Wasm execution fuel limit")?;
    let fuel_remaining = super::fields::required_u64(&fuel[1], "Wasm execution fuel remaining")?;
    if fuel_remaining > fuel_limit {
        return Err(crate::error::Failure::invalid_harness("Wasm execution remaining fuel exceeds limit"));
    }
    let memory_value = crate::preserves_rail::value_to_iovalue(&receipt[7]);
    let memory = super::fields::simple_record(&memory_value, "memory-limit", 1)?;
    super::fields::required_u64(&memory[0], "Wasm execution memory limit")?;
    Ok(())
}

fn validate_wasm_abi_fields(
    actor: &super::model::ActorDecl,
    position: usize,
    actor_input: &preserves::IOValue,
    receipt: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<()> {
    let abi = super::fields::required_record_string(&receipt[8], "abi", "Wasm execution ABI schema")?;
    if abi != crate::preserves_rail::RUNTIME_WASM_ABI_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Wasm execution ABI schema {abi}; expected {}",
            crate::preserves_rail::RUNTIME_WASM_ABI_SCHEMA
        )));
    }
    let input_ref = super::fields::required_record_hash(&receipt[9], "input-ref", "Wasm execution input ref")?;
    let expected_input_ref = crate::preserves_rail::canonical_hash(actor_input)?;
    if input_ref != expected_input_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm execution input ref mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    super::fields::required_record_hash(&receipt[10], "output-ref", "Wasm execution output ref")?;
    let output_bytes =
        super::fields::required_record_u64(&receipt[11], "output-bytes", "Wasm execution output byte count")?;
    if output_bytes > WASM_ABI_MAX_OUTPUT_BYTES_FOR_VALIDATION {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm execution output byte count exceeds molten.wasm.abi.v1 limit for actor {} at observation {position}",
            actor.id
        )));
    }
    Ok(())
}

fn require_wasm_execution_checks(checks: &[String], arity: usize) -> crate::error::Result<()> {
    if arity == 13 {
        super::clearance::require_executor_preflight_check(checks, "preserves-abi-v1")?;
        super::clearance::require_executor_preflight_check(checks, "canonical-preserves-input")?;
        super::clearance::require_executor_preflight_check(checks, "canonical-preserves-output")?;
        super::clearance::require_executor_preflight_check(checks, "guest-memory-bounds")?;
    }
    super::clearance::require_executor_preflight_check(checks, "wasmtime-instantiated")?;
    super::clearance::require_executor_preflight_check(checks, "no-wasi")?;
    super::clearance::require_executor_preflight_check(checks, "fuel-bounded")?;
    super::clearance::require_executor_preflight_check(checks, "memory-bounded")?;
    super::clearance::require_executor_preflight_check(checks, "hostcall-envelope-binding")?;
    Ok(())
}
