
pub(crate) fn steel_execution_receipt_value(input: SteelExecutionReceiptInput<'_>) -> IoValue {
    record("steel-execution-receipt-v1", vec![
        string(crate::preserves_rail::RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA),
        record("actor", vec![string(input.actor_id)]),
        record("source-ref", vec![string(input.source_ref)]),
        record("callable", vec![string(input.callable)]),
        record("operation", vec![string(input.operation)]),
        record("input-ref", vec![string(input.input_ref)]),
        record("output-ref", vec![string(input.output_ref)]),
        record("hostcalls", vec![sequence(
            input.hostcalls.iter().map(|hostcall| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("resources", vec![
            record("fuel", vec![
                u64_value(input.resource_limits.fuel_limit),
                u64_value(input.resource_limits.fuel_remaining),
            ]),
            record("source-bytes", vec![u64_value(input.resource_limits.source_bytes)]),
            record("input-bytes", vec![u64_value(input.resource_limits.input_bytes)]),
            record("output-bytes", vec![u64_value(input.resource_limits.output_bytes)]),
            record("hostcalls", vec![
                u64_value(input.resource_limits.hostcall_limit),
                u64_value(input.resource_limits.hostcall_count),
            ]),
        ]),
        hostcall_checks_value(&[
            "steel-vm-executed",
            "reviewed-callable-binding",
            "canonical-preserves-input",
            "canonical-preserves-output",
            "no-ambient-steel-io",
            "hostcall-envelope-binding",
            "effect-manifest-bound",
            "effect-request-admitted",
            "declared-effect-id-required",
            "resource-bounded",
            "fuel-bounded",
            "hostcall-count-bounded",
            "io-bytes-bounded",
        ]),
    ])
}

pub(crate) struct WasmExecutionReceiptInput<'a> {
    pub actor_id: &'a str,
    pub module_ref: &'a str,
    pub export: &'a str,
    pub operation: &'a str,
    pub hostcalls: &'a [String],
    pub fuel_limit: u64,
    pub fuel_remaining: u64,
    pub memory_limit_bytes: u64,
    pub abi: Option<WasmAbiReceiptInput>,
}

pub(crate) struct WasmAbiReceiptInput {
    pub input_ref: String,
    pub output_ref: String,
    pub output_bytes: u64,
}

pub(crate) fn wasm_execution_receipt_value(input: WasmExecutionReceiptInput<'_>) -> IoValue {
    let mut checks = vec![
        "wasmtime-instantiated",
        "no-wasi",
        "fuel-bounded",
        "memory-bounded",
        "hostcall-envelope-binding",
        "effect-manifest-bound",
        "effect-request-admitted",
        "declared-effect-id-required",
    ];
    let mut fields = vec![
        string(crate::preserves_rail::RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA),
        record("actor", vec![string(input.actor_id)]),
        record("module-ref", vec![string(input.module_ref)]),
        record("export", vec![string(input.export)]),
        record("operation", vec![string(input.operation)]),
        record("hostcalls", vec![sequence(
            input.hostcalls.iter().map(|hostcall| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("fuel", vec![u64_value(input.fuel_limit), u64_value(input.fuel_remaining)]),
        record("memory-limit", vec![u64_value(input.memory_limit_bytes)]),
    ];
    if let Some(abi) = input.abi {
        checks.extend([
            "preserves-abi-v1",
            "canonical-preserves-input",
            "canonical-preserves-output",
            "guest-memory-bounds",
        ]);
        fields.extend([
            record("abi", vec![string(crate::preserves_rail::RUNTIME_WASM_ABI_SCHEMA)]),
            record("input-ref", vec![string(&abi.input_ref)]),
            record("output-ref", vec![string(&abi.output_ref)]),
            record("output-bytes", vec![u64_value(abi.output_bytes)]),
        ]);
    }
    fields.push(hostcall_checks_value(&checks));
    record("wasm-execution-receipt-v1", fields)
}

fn hostcall_checks_value(checks: &[&str]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|name| record("check", vec![string(*name), string("pass")])).collect(),
    )])
}

fn actor_decl_for_primary_actor<'a>(suite: &'a Suite, actor: &str) -> Result<&'a ActorDecl> {
    suite
        .actors
        .iter()
        .find(|decl| decl.id == actor)
        .ok_or_else(|| MoltenError::invalid_harness(format!("actor {actor} missing from executor registry")))
}

fn actor_kind_for_primary_actor<'a>(suite: &'a Suite, actor: &str) -> Result<&'a ActorKind> {
    actor_decl_for_primary_actor(suite, actor).map(|decl| &decl.kind)
}

pub fn snapshot_value(snapshot: &super::core::RuntimeSnapshot) -> IoValue {
    record("runtime-state-v1", vec![
        u64_value(snapshot.logical_time),
        u64_value(snapshot.rng_state),
        u64_value(snapshot.effect_sequence),
        tuple_set("messages", &snapshot.messages, |message| {
            record("message", vec![
                string(&message.from),
                string(&message.to),
                message.body.as_iovalue().clone(),
            ])
        }),
        tuple_set("assertions", &snapshot.assertions, |assertion| {
            record("assertion", vec![string(&assertion.actor), assertion.value.as_iovalue().clone()])
        }),
        tuple_set("observers", &snapshot.observers, |observer| {
            record("observer", vec![string(&observer.actor), observer.pattern.as_iovalue().clone()])
        }),
    ])
}

pub fn observation_value(
    index: u64,
    step_ref: String,
    before_state_hash: String,
    after_state_hash: String,
    events: Vec<IoValue>,
) -> Result<IoValue> {
    let mut event_refs = Vec::with_capacity(events.len());
    for event in &events {
        event_refs.push(canonical_hash(event)?);
    }
    let mut event_ref_values: Vec<IoValue> = Vec::with_capacity(event_refs.len());
    for reference in event_refs {
        event_ref_values.push(string(reference));
    }
    Ok(record("turn-observation-v1", vec![
        string(crate::preserves_rail::HARNESS_OBSERVATION_SCHEMA),
        u64_value(index),
        string(step_ref),
        string(before_state_hash),
        string(after_state_hash),
        record("event-refs", vec![sequence(event_ref_values)]),
        sequence(events),
    ]))
}

pub struct ReportValueInput<'a> {
    pub suite: &'a Suite,
    pub suite_ref: String,
    pub initial_state_hash: String,
    pub final_state_hash: String,
    pub policy_gate: IoValue,
    pub capability_gate: IoValue,
    pub budget_gate: IoValue,
    pub observations: Vec<IoValue>,
    pub effect_log: Vec<EffectLogEntry>,
    pub budget: &'a Budget,
    pub usage: &'a BudgetUsage,
}

pub fn report_value(input: ReportValueInput<'_>) -> IoValue {
    let executor_preflights = match executor_preflights_value(input.suite) {
        Ok(value) => value,
        Err(error) => record("executor-preflights-invalid-v1", vec![
            string(crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA),
            record("error", vec![string(error.to_string())]),
        ]),
    };
    record("harness-report-v1", vec![
        string(crate::preserves_rail::HARNESS_REPORT_SCHEMA),
        string("pass"),
        string("deterministic"),
        string("local-deterministic"),
        string(crate::preserves_rail::HASH_ALGORITHM),
        string(input.suite_ref),
        string(input.initial_state_hash),
        string(input.final_state_hash),
        input.suite.source_value.clone(),
        input.policy_gate,
        input.capability_gate,
        input.budget_gate,
        actor_registry_value(&input.suite.actors),
        executor_preflights,
        sequence(input.observations),
        effect_log_value(&input.effect_log),
        budget_value(input.budget, input.usage),
    ])
}

pub fn failure_value(phase: &str, error: &MoltenError, mut diagnostics: Vec<IoValue>) -> IoValue {
    diagnostics.extend(error_diagnostics(error));
    record("harness-failure-v1", vec![
        string(crate::preserves_rail::HARNESS_FAILURE_SCHEMA),
        record("phase", vec![string(phase)]),
        record("kind", vec![string(error_kind(error))]),
        record("message", vec![string(error.to_string())]),
        sequence(diagnostics),
    ])
}

pub fn suite_failure_value(phase: &str, error: &MoltenError, suite_value: &IoValue) -> Result<IoValue> {
    Ok(failure_value(phase, error, vec![
        record("suite-ref", vec![string(canonical_hash(suite_value)?)]),
        record("suite", vec![suite_value.clone()]),
    ]))
}

pub fn report_failure_value(phase: &str, error: &MoltenError, report_value: &IoValue) -> Result<IoValue> {
    Ok(failure_value(phase, error, vec![
        record("report-ref", vec![string(canonical_hash(report_value)?)]),
        record("report", vec![report_value.clone()]),
    ]))
}

pub fn parse_failure(failure_value: &IoValue) -> Result<Failure> {
    let failure = simple_record(failure_value, "harness-failure-v1", 5)?;
    let schema = required_string(&failure[0], "failure schema")?;
    if schema != crate::preserves_rail::HARNESS_FAILURE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported failure schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_FAILURE_SCHEMA
        )));
    }
    let phase_value = value_to_iovalue(&failure[1]);
    let phase_record = simple_record(&phase_value, "phase", 1)?;
    let phase = required_string(&phase_record[0], "failure phase")?;
    if !matches!(phase.as_str(), "preflight" | "execute" | "replay" | "validate" | "export" | "verify" | "unpack") {
        return Err(MoltenError::invalid_harness(format!("unsupported failure phase {phase}")));
    }
    let kind_value = value_to_iovalue(&failure[2]);
    let kind_record = simple_record(&kind_value, "kind", 1)?;
    let kind = required_string(&kind_record[0], "failure kind")?;
    if kind.is_empty() {
        return Err(MoltenError::invalid_harness("failure kind must not be empty"));
    }
    let message_value = value_to_iovalue(&failure[3]);
    let message_record = simple_record(&message_value, "message", 1)?;
    let message = required_string(&message_record[0], "failure message")?;
    let diagnostic_values = required_sequence(&failure[4], "failure diagnostics")?;
    let mut diagnostics = Vec::with_capacity(diagnostic_values.len());
    for diagnostic in diagnostic_values.iter() {
        diagnostics.push(value_to_iovalue(&diagnostic));
    }
    Ok(Failure {
        failure_ref: canonical_hash(failure_value)?,
        phase,
        kind,
        message,
        diagnostics,
    })
}

pub fn failure_summary(failure_value: &IoValue) -> Result<String> {
    let failure = parse_failure(failure_value)?;
    Ok(format!(
        "failure {}\nstatus=fail\nphase={}\nkind={}\nmessage={}\ndiagnostics={}",
        failure.failure_ref,
        failure.phase,
        failure.kind,
        failure.message,
        failure.diagnostics.len()
    ))
}

struct ParsedHeader {
    status: String,
    replay_status: String,
    profile: String,
    hash_algorithm: String,
    suite_ref: String,
    initial_state_hash: String,
    final_state_hash: String,
    suite_value: IoValue,
    suite: Suite,
}
