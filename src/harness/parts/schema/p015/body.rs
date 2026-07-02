
fn executor_conformance_suite_value(allowed_hostcalls: &[String]) -> IoValue {
    record("executor-conformance-suite-v1", vec![
        string(crate::preserves_rail::HARNESS_EXECUTOR_CONFORMANCE_SCHEMA),
        record("boundary", vec![string("molten.runtime.executor-hostcall-boundary.v1")]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("actor-input", vec![string(crate::preserves_rail::RUNTIME_ACTOR_INPUT_SCHEMA)]),
        record("hostcall-request", vec![string(crate::preserves_rail::RUNTIME_HOSTCALL_REQUEST_SCHEMA)]),
        record("hostcall-decision", vec![string(crate::preserves_rail::RUNTIME_HOSTCALL_DECISION_SCHEMA)]),
        record("actor-output", vec![string(crate::preserves_rail::RUNTIME_ACTOR_OUTPUT_SCHEMA)]),
        hostcall_checks_value(&[
            "canonical-preserves",
            "hostcall-admission-binding",
            "deterministic-replay",
            "no-ambient-executor-io",
            "cross-kind-compatible",
        ]),
    ])
}

fn executor_sandbox_value(kind: &ActorKind) -> IoValue {
    record("executor-sandbox-v1", vec![
        record("kind", vec![string(kind.as_str())]),
        record("ambient-io", vec![bool_value(false)]),
        record("hostcalls-only", vec![bool_value(true)]),
    ])
}

fn executor_preflight_checks(kind: &ActorKind) -> &'static [&'static str] {
    match kind {
        ActorKind::Native => &[
            "actor-kind-binding",
            "allowed-hostcall-binding",
            "no-ambient-executor-io",
            "native-local-executor",
        ],
        ActorKind::Steel | ActorKind::Wasm | ActorKind::Adapter | ActorKind::RemoteProxy => &[
            "actor-kind-binding",
            "allowed-hostcall-binding",
            "no-ambient-executor-io",
            "requires-executor-adapter",
        ],
    }
}

fn steel_executor_preflight_checks() -> &'static [&'static str] {
    &[
        "actor-kind-binding",
        "allowed-hostcall-binding",
        "no-ambient-executor-io",
        "steel-source-ref-binding",
        "steel-callable-review",
        "steel-hostcall-contract",
    ]
}

fn wasm_executor_preflight_checks() -> &'static [&'static str] {
    &[
        "actor-kind-binding",
        "allowed-hostcall-binding",
        "no-ambient-executor-io",
        "wasm-module-ref-binding",
        "wasmparser-inspection",
        "wasm-deny-by-default-wasi",
        "wasm-hostcall-contract",
        "wit-interface-binding",
    ]
}

fn adapter_executor_preflight_checks() -> &'static [&'static str] {
    &[
        "actor-kind-binding",
        "allowed-hostcall-binding",
        "no-ambient-executor-io",
        "adapter-manifest-binding",
        "adapter-permission-binding",
        "adapter-transcript-replay",
    ]
}

fn remote_proxy_executor_preflight_checks() -> &'static [&'static str] {
    &[
        "actor-kind-binding",
        "allowed-hostcall-binding",
        "no-ambient-executor-io",
        "remote-peer-binding",
        "remote-contract-binding",
        "remote-transcript-replay",
    ]
}

fn steel_review_receipt_value(config: &SteelExecutorConfig) -> Result<IoValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(record("steel-review-receipt-v1", vec![
        string(crate::preserves_rail::RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("source-ref", vec![string(steel_source_ref(config)?)]),
        record("callable", vec![string(&config.callable)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        hostcall_checks_value(&[
            "source-ref-binding",
            "reviewed-callable",
            "allowed-hostcall-contract",
            "no-ambient-steel-io",
        ]),
    ]))
}

fn adapter_preflight_receipt_value(config: &AdapterExecutorConfig) -> Result<IoValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(record("adapter-preflight-receipt-v1", vec![
        string(crate::preserves_rail::RUNTIME_ADAPTER_PREFLIGHT_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("manifest-ref", vec![string(adapter_manifest_ref(config)?)]),
        record("abi-ref", vec![string(canonical_hash(&string(&config.abi))?)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("transcript", vec![string(&config.transcript)]),
        hostcall_checks_value(&[
            "manifest-ref-binding",
            "permission-binding",
            "deterministic-transcript",
            "no-ambient-adapter-io",
        ]),
    ]))
}

fn remote_proxy_preflight_receipt_value(config: &RemoteProxyExecutorConfig) -> Result<IoValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(record("remote-proxy-preflight-receipt-v1", vec![
        string(crate::preserves_rail::RUNTIME_REMOTE_PROXY_PREFLIGHT_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("peer-ref", vec![string(canonical_hash(&string(&config.peer))?)]),
        record("endpoint-ref", vec![string(remote_proxy_endpoint_ref(config)?)]),
        record("contract-ref", vec![string(canonical_hash(&string(&config.contract))?)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("transcript", vec![string(&config.transcript)]),
        hostcall_checks_value(&[
            "peer-identity-binding",
            "endpoint-contract-binding",
            "verified-transcript",
            "transport-not-authority",
        ]),
    ]))
}

fn validate_steel_executor_config(actor_id: &str, config: &SteelExecutorConfig) -> Result<()> {
    if config.source.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("Steel executor source for actor {actor_id} is empty")));
    }
    if config.callable.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("Steel executor callable for actor {actor_id} is empty")));
    }
    for token in FORBIDDEN_STEEL_SOURCE_TOKENS {
        if config.source.contains(token) {
            return Err(MoltenError::invalid_harness(format!(
                "Steel executor source for actor {actor_id} references forbidden ambient IO token {token}; reviewed Steel preflight remains fail-closed"
            )));
        }
    }
    Ok(())
}

const FORBIDDEN_STEEL_SOURCE_TOKENS: &[&str] = &[
    "open-input-file",
    "open-output-file",
    "call-with-input-file",
    "call-with-output-file",
    "delete-file",
    "read-file",
    "write-file",
    "system",
    "process",
    "current-seconds",
    "current-inexact-milliseconds",
    "random",
    "tcp",
    "udp",
    "http",
    "ffi",
];

pub(crate) fn steel_source_ref(config: &SteelExecutorConfig) -> Result<String> {
    canonical_hash(&string(&config.source))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WasmInspection {
    module_kind: String,
    imports: Vec<WasmImportEvidence>,
}

fn wasm_inspection_receipt_value(config: &WasmExecutorConfig) -> Result<IoValue> {
    let inspection = inspect_wasm_module(config)?;
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(record("wasm-inspection-receipt-v1", vec![
        string(crate::preserves_rail::RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("module-ref", vec![string(wasm_module_ref(config)?)]),
        record("module-kind", vec![string(&inspection.module_kind)]),
        record("imports", vec![sequence(inspection.imports.iter().map(wasm_import_value).collect())]),
        record("wit-ref", vec![string(wasm_wit_ref(config)?)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        hostcall_checks_value(&[
            "module-ref-binding",
            "wasmparser-validated",
            "deny-by-default-wasi",
            "allowed-hostcall-contract",
            "wit-interface-binding",
        ]),
    ]))
}

fn validate_wasm_executor_config(actor_id: &str, config: &WasmExecutorConfig) -> Result<()> {
    if config.wit.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("Wasm executor WIT interface for actor {actor_id} is empty")));
    }
    let inspection = inspect_wasm_module(config).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor module for actor {actor_id} failed preflight: {error}"))
    })?;
    validate_wasm_imports(actor_id, &inspection.imports, &config.allowed_hostcalls)
}

fn validate_adapter_executor_config(actor_id: &str, config: &AdapterExecutorConfig) -> Result<()> {
    if config.manifest.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("adapter executor manifest for actor {actor_id} is empty")));
    }
    if config.abi.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("adapter executor ABI for actor {actor_id} is empty")));
    }
    for token in FORBIDDEN_ADAPTER_MANIFEST_TOKENS {
        if config.manifest.contains(token) || config.abi.contains(token) {
            return Err(MoltenError::invalid_harness(format!(
                "adapter executor manifest for actor {actor_id} references forbidden ambient or stale token {token}"
            )));
        }
    }
    if config.transcript != "deterministic-local" && config.transcript != "verified" {
        return Err(MoltenError::invalid_harness(format!(
            "adapter executor transcript profile for actor {actor_id} must be deterministic-local or verified"
        )));
    }
    Ok(())
}

const FORBIDDEN_ADAPTER_MANIFEST_TOKENS: &[&str] =
    &["ambient-network", "ambient-fs", "process", "socket", "stale-signature"];

fn validate_remote_proxy_executor_config(actor_id: &str, config: &RemoteProxyExecutorConfig) -> Result<()> {
    if config.peer.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("remote-proxy peer for actor {actor_id} is empty")));
    }
    if config.peer == "unknown" || config.peer.contains("revoked") {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy peer for actor {actor_id} cannot satisfy trusted deterministic gate evidence"
        )));
    }
    if config.endpoint.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("remote-proxy endpoint for actor {actor_id} is empty")));
    }
    if !config.endpoint.starts_with("iroh:") {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy endpoint for actor {actor_id} must use an explicit iroh: transport profile"
        )));
    }
    if config.contract.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("remote-proxy contract for actor {actor_id} is empty")));
    }
    if config.contract.contains("stale-signature") {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy contract for actor {actor_id} references stale signature evidence"
        )));
    }
    if config.transcript != "verified" {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy transcript profile for actor {actor_id} must be verified before deterministic gates"
        )));
    }
    Ok(())
}
