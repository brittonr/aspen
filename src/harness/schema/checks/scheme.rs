use preserves::ValueImpl;

pub(super) fn validate_steel_execution_evidence(
    suite: &super::model::Suite,
    step: &crate::harness::nucleus::CoreStep,
    position: usize,
    decision: &crate::runtime::AdmissionDecision,
    runtime_events: &[preserves::IOValue],
) -> crate::error::Result<()> {
    let actor = step.primary_actor();
    let decl = super::intake::actor_decl_for_primary_actor(suite, actor)?;
    if decl.kind != super::model::ActorKind::Steel {
        return Ok(());
    }
    if !decision.is_allowed() {
        if runtime_events
            .iter()
            .any(|event| super::tape::event_boundary(event) == super::observed::EventBoundary::SteelExecution)
        {
            return Err(crate::error::Failure::invalid_harness(format!(
                "denied Steel step at observation {position} must not carry Steel execution evidence"
            )));
        }
        return Ok(());
    }
    let Some(receipt) = runtime_events.first() else {
        return Err(crate::error::Failure::invalid_harness(format!(
            "missing Steel execution evidence at observation {position}"
        )));
    };
    validate_steel_execution_receipt(decl, step, position, receipt)
}

fn steel_config(actor: &super::model::ActorDecl) -> crate::error::Result<&super::model::SteelExecutorConfig> {
    let super::model::ActorExecutorConfig::Steel(config) = actor.executor.as_ref().ok_or_else(|| {
        crate::error::Failure::invalid_harness(format!("Steel actor {} missing Steel executor config", actor.id))
    })?
    else {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel actor {} has non-Steel executor config",
            actor.id
        )));
    };
    Ok(config)
}

fn validate_steel_execution_receipt(
    actor: &super::model::ActorDecl,
    step: &crate::harness::nucleus::CoreStep,
    position: usize,
    value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let receipt_value = value
        .collect_simple_record("steel-execution-receipt-v1", None)
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <steel-execution-receipt-v1 ...>"))?;
    let arity = receipt_value.fields_iter().count();
    if arity != 9 && arity != 10 {
        return Err(crate::error::Failure::invalid_harness(format!(
            "expected <steel-execution-receipt-v1 ...> with arity 9 or 10, got {arity}"
        )));
    }
    let receipt = &receipt_value;
    let schema = super::fields::required_string(&receipt[0], "Steel execution receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Steel execution receipt schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA
        )));
    }
    let actor_id = super::fields::required_record_string(&receipt[1], "actor", "Steel execution actor")?;
    if actor_id != actor.id {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel execution actor mismatch at observation {position}: got {actor_id}, expected {}",
            actor.id
        )));
    }
    let config = steel_config(actor)?;
    let source_ref = super::fields::required_record_hash(&receipt[2], "source-ref", "Steel execution source ref")?;
    if source_ref != super::sources::steel_source_ref(config)? {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel execution source ref mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let callable = super::fields::required_record_string(&receipt[3], "callable", "Steel execution callable")?;
    if callable != config.callable {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel execution callable mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let operation = crate::harness::nucleus::AdmissionRequest::from_step(step).action.as_str().to_string();
    let receipt_operation =
        super::fields::required_record_string(&receipt[4], "operation", "Steel execution operation")?;
    if receipt_operation != operation {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel execution operation mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    super::fields::required_record_hash(&receipt[5], "input-ref", "Steel execution input ref")?;
    super::fields::required_record_hash(&receipt[6], "output-ref", "Steel execution output ref")?;
    let hostcalls =
        super::fields::required_record_string_sequence(&receipt[7], "hostcalls", "Steel execution hostcalls")?;
    if hostcalls != vec![operation] {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel execution hostcalls mismatch for actor {} at observation {position}",
            actor.id
        )));
    }
    let checks_index = steel_execution_checks_index(receipt, arity)?;
    let checks = super::clearance::parse_executor_preflight_checks(&receipt[checks_index])?;
    require_steel_execution_checks(&checks, arity)
}

fn steel_execution_checks_index(
    receipt: &preserves::Record<preserves::Value<preserves::IOValue>>,
    arity: usize,
) -> crate::error::Result<usize> {
    if arity == 10 {
        validate_steel_execution_resources(&receipt[8])?;
        Ok(9)
    } else {
        Ok(8)
    }
}

fn validate_steel_execution_resources(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<()> {
    let resources_value = crate::preserves_rail::value_to_iovalue(value);
    let resources = super::fields::simple_record(&resources_value, "resources", 5)?;
    let fuel_value = crate::preserves_rail::value_to_iovalue(&resources[0]);
    let fuel = super::fields::simple_record(&fuel_value, "fuel", 2)?;
    let fuel_limit = super::fields::required_u64(&fuel[0], "Steel execution fuel limit")?;
    let fuel_remaining = super::fields::required_u64(&fuel[1], "Steel execution fuel remaining")?;
    if fuel_remaining > fuel_limit {
        return Err(crate::error::Failure::invalid_harness("Steel execution remaining fuel exceeds limit"));
    }
    super::fields::required_record_u64(&resources[1], "source-bytes", "Steel execution source byte count")?;
    super::fields::required_record_u64(&resources[2], "input-bytes", "Steel execution input byte count")?;
    super::fields::required_record_u64(&resources[3], "output-bytes", "Steel execution output byte count")?;
    let hostcalls_value = crate::preserves_rail::value_to_iovalue(&resources[4]);
    let hostcalls_record = super::fields::simple_record(&hostcalls_value, "hostcalls", 2)?;
    let hostcall_limit = super::fields::required_u64(&hostcalls_record[0], "Steel execution hostcall limit")?;
    let hostcall_count = super::fields::required_u64(&hostcalls_record[1], "Steel execution hostcall count")?;
    if hostcall_count > hostcall_limit {
        return Err(crate::error::Failure::invalid_harness("Steel execution hostcall count exceeds limit"));
    }
    Ok(())
}

fn require_steel_execution_checks(checks: &[String], arity: usize) -> crate::error::Result<()> {
    super::clearance::require_executor_preflight_check(checks, "steel-vm-executed")?;
    super::clearance::require_executor_preflight_check(checks, "reviewed-callable-binding")?;
    super::clearance::require_executor_preflight_check(checks, "canonical-preserves-input")?;
    super::clearance::require_executor_preflight_check(checks, "canonical-preserves-output")?;
    super::clearance::require_executor_preflight_check(checks, "no-ambient-steel-io")?;
    super::clearance::require_executor_preflight_check(checks, "hostcall-envelope-binding")?;
    if arity == 10 {
        super::clearance::require_executor_preflight_check(checks, "resource-bounded")?;
        super::clearance::require_executor_preflight_check(checks, "fuel-bounded")?;
        super::clearance::require_executor_preflight_check(checks, "hostcall-count-bounded")?;
        super::clearance::require_executor_preflight_check(checks, "io-bytes-bounded")?;
    }
    Ok(())
}
