pub struct SteelExecutionReceiptInput<'a> {
    pub actor_id: &'a str,
    pub source_ref: &'a str,
    pub callable: &'a str,
    pub operation: &'a str,
    pub input_ref: &'a str,
    pub output_ref: &'a str,
    pub hostcalls: &'a [String],
    pub resource_limits: SteelResourceReceiptInput,
}

pub struct SteelResourceReceiptInput {
    pub fuel_limit: u64,
    pub fuel_remaining: u64,
    pub source_bytes: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub hostcall_limit: u64,
    pub hostcall_count: u64,
}

pub fn steel_execution_receipt_value(input: SteelExecutionReceiptInput<'_>) -> preserves::IOValue {
    crate::preserves_rail::record("steel-execution-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_STEEL_EXECUTION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("actor", vec![crate::preserves_rail::string(input.actor_id)]),
        crate::preserves_rail::record("source-ref", vec![crate::preserves_rail::string(input.source_ref)]),
        crate::preserves_rail::record("callable", vec![crate::preserves_rail::string(input.callable)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("input-ref", vec![crate::preserves_rail::string(input.input_ref)]),
        crate::preserves_rail::record("output-ref", vec![crate::preserves_rail::string(input.output_ref)]),
        crate::preserves_rail::record("hostcalls", vec![crate::preserves_rail::sequence(
            input
                .hostcalls
                .iter()
                .map(|hostcall| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("resources", vec![
            crate::preserves_rail::record("fuel", vec![
                crate::preserves_rail::u64_value(input.resource_limits.fuel_limit),
                crate::preserves_rail::u64_value(input.resource_limits.fuel_remaining),
            ]),
            crate::preserves_rail::record("source-bytes", vec![crate::preserves_rail::u64_value(
                input.resource_limits.source_bytes,
            )]),
            crate::preserves_rail::record("input-bytes", vec![crate::preserves_rail::u64_value(
                input.resource_limits.input_bytes,
            )]),
            crate::preserves_rail::record("output-bytes", vec![crate::preserves_rail::u64_value(
                input.resource_limits.output_bytes,
            )]),
            crate::preserves_rail::record("hostcalls", vec![
                crate::preserves_rail::u64_value(input.resource_limits.hostcall_limit),
                crate::preserves_rail::u64_value(input.resource_limits.hostcall_count),
            ]),
        ]),
        super::calls::hostcall_checks_value(&[
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

pub struct WasmExecutionReceiptInput<'a> {
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

pub struct WasmAbiReceiptInput {
    pub input_ref: String,
    pub output_ref: String,
    pub output_bytes: u64,
}

pub fn wasm_execution_receipt_value(input: WasmExecutionReceiptInput<'_>) -> preserves::IOValue {
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
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_WASM_EXECUTION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("actor", vec![crate::preserves_rail::string(input.actor_id)]),
        crate::preserves_rail::record("module-ref", vec![crate::preserves_rail::string(input.module_ref)]),
        crate::preserves_rail::record("export", vec![crate::preserves_rail::string(input.export)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("hostcalls", vec![crate::preserves_rail::sequence(
            input
                .hostcalls
                .iter()
                .map(|hostcall| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("fuel", vec![
            crate::preserves_rail::u64_value(input.fuel_limit),
            crate::preserves_rail::u64_value(input.fuel_remaining),
        ]),
        crate::preserves_rail::record("memory-limit", vec![crate::preserves_rail::u64_value(input.memory_limit_bytes)]),
    ];
    if let Some(abi) = input.abi {
        checks.extend([
            "preserves-abi-v1",
            "canonical-preserves-input",
            "canonical-preserves-output",
            "guest-memory-bounds",
        ]);
        fields.extend([
            crate::preserves_rail::record("abi", vec![crate::preserves_rail::string(
                crate::preserves_rail::RUNTIME_WASM_ABI_SCHEMA,
            )]),
            crate::preserves_rail::record("input-ref", vec![crate::preserves_rail::string(&abi.input_ref)]),
            crate::preserves_rail::record("output-ref", vec![crate::preserves_rail::string(&abi.output_ref)]),
            crate::preserves_rail::record("output-bytes", vec![crate::preserves_rail::u64_value(abi.output_bytes)]),
        ]);
    }
    fields.push(super::calls::hostcall_checks_value(&checks));
    crate::preserves_rail::record("wasm-execution-receipt-v1", fields)
}
