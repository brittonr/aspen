
fn inspect_wasm_module(config: &WasmExecutorConfig) -> Result<WasmInspection> {
    let bytes = wasm_module_bytes(config)?;
    wasmparser::Validator::new()
        .validate_all(&bytes)
        .map_err(|error| MoltenError::invalid_harness(format!("wasmparser validation failed: {error}")))?;
    let mut module_kind = None;
    let mut imports = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        match payload.map_err(|error| MoltenError::invalid_harness(format!("wasmparser parse failed: {error}")))? {
            wasmparser::Payload::Version { encoding, .. } => {
                module_kind = Some(match encoding {
                    wasmparser::Encoding::Module => "core-module".to_string(),
                    wasmparser::Encoding::Component => "component".to_string(),
                });
            }
            wasmparser::Payload::ImportSection(section) => {
                for import in section {
                    let import = import
                        .map_err(|error| MoltenError::invalid_harness(format!("wasm import parse failed: {error}")))?;
                    push_bounded(
                        &mut imports,
                        WasmImportEvidence {
                            module: import.module.to_string(),
                            name: import.name.to_string(),
                            kind: wasm_type_ref_kind(&import.ty).to_string(),
                        },
                        MAX_WASM_IMPORT_EVIDENCE,
                        "wasm import evidence",
                    )?;
                }
            }
            wasmparser::Payload::ComponentImportSection(section) => {
                for import in section {
                    let import = import.map_err(|error| {
                        MoltenError::invalid_harness(format!("wasm component import parse failed: {error}"))
                    })?;
                    push_bounded(
                        &mut imports,
                        WasmImportEvidence {
                            module: "component".to_string(),
                            name: import.name.0.to_string(),
                            kind: format!("component:{:?}", import.ty.kind()),
                        },
                        MAX_WASM_IMPORT_EVIDENCE,
                        "wasm import evidence",
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(WasmInspection {
        module_kind: module_kind.unwrap_or_else(|| "unknown".to_string()),
        imports,
    })
}

fn validate_wasm_imports(actor_id: &str, imports: &[WasmImportEvidence], allowed_hostcalls: &[String]) -> Result<()> {
    for import in imports {
        if import.module == "molten:hostcall" {
            if import.kind != "func" {
                return Err(MoltenError::invalid_harness(format!(
                    "Wasm executor import {}::{} for actor {actor_id} must be a function hostcall",
                    import.module, import.name
                )));
            }
            if !allowed_hostcalls.iter().any(|allowed| allowed == &import.name) {
                return Err(MoltenError::invalid_harness(format!(
                    "Wasm executor import {}::{} for actor {actor_id} is not in allowed hostcalls",
                    import.module, import.name
                )));
            }
            continue;
        }
        if import.module == "component" && allowed_hostcalls.iter().any(|allowed| allowed == &import.name) {
            continue;
        }
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor import {}::{} for actor {actor_id} is not an allowed Molten hostcall; WASI and ambient imports remain disabled",
            import.module, import.name
        )));
    }
    Ok(())
}

fn wasm_type_ref_kind(ty: &wasmparser::TypeRef) -> &'static str {
    match ty {
        wasmparser::TypeRef::Func(_) => "func",
        wasmparser::TypeRef::Table(_) => "table",
        wasmparser::TypeRef::Memory(_) => "memory",
        wasmparser::TypeRef::Global(_) => "global",
        wasmparser::TypeRef::Tag(_) => "tag",
    }
}

fn wasm_import_value(import: &WasmImportEvidence) -> IoValue {
    record("import", vec![string(&import.module), string(&import.name), string(&import.kind)])
}

pub(crate) fn wasm_module_ref(config: &WasmExecutorConfig) -> Result<String> {
    canonical_hash(&record("wasm-module-bytes-v1", vec![string(&config.module_hex)]))
}

fn adapter_manifest_ref(config: &AdapterExecutorConfig) -> Result<String> {
    canonical_hash(&record("adapter-manifest-v1", vec![string(&config.manifest), string(&config.abi)]))
}

fn remote_proxy_endpoint_ref(config: &RemoteProxyExecutorConfig) -> Result<String> {
    canonical_hash(&record("remote-proxy-endpoint-v1", vec![
        string(&config.peer),
        string(&config.endpoint),
        string(&config.contract),
    ]))
}

pub(crate) fn wasm_executor_export_name(operation: &str) -> String {
    format!("molten_hostcall_{operation}")
}

fn wasm_wit_ref(config: &WasmExecutorConfig) -> Result<String> {
    canonical_hash(&string(&config.wit))
}

pub(crate) fn wasm_module_bytes(config: &WasmExecutorConfig) -> Result<Vec<u8>> {
    decode_hex_bytes(&config.module_hex, "Wasm executor module hex")
}

fn normalize_hex(input: &str, field: &str) -> Result<String> {
    let mut normalized = String::new();
    for character in input.chars() {
        if character.is_ascii_whitespace() || character == '_' {
            continue;
        }
        if !character.is_ascii_hexdigit() {
            return Err(MoltenError::invalid_harness(format!("{field} contains non-hex character {character:?}")));
        }
        normalized.push(character.to_ascii_lowercase());
    }
    if normalized.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    if !normalized.len().is_multiple_of(2) {
        return Err(MoltenError::invalid_harness(format!("{field} must contain an even number of hex digits")));
    }
    Ok(normalized)
}

fn decode_hex_bytes(input: &str, field: &str) -> Result<Vec<u8>> {
    let normalized = normalize_hex(input, field)?;
    let mut bytes = Vec::with_capacity(normalized.len() / 2);
    for index in (0..normalized.len()).step_by(2) {
        let byte = u8::from_str_radix(&normalized[index..index + 2], 16).map_err(|error| {
            MoltenError::invalid_harness(format!("{field} contains invalid byte at offset {index}: {error}"))
        })?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn allowed_hostcalls_for_actor(suite: &Suite, actor: &ActorDecl) -> Vec<String> {
    match &actor.executor {
        Some(ActorExecutorConfig::Steel(config)) => config.allowed_hostcalls.clone(),
        Some(ActorExecutorConfig::Wasm(config)) => config.allowed_hostcalls.clone(),
        Some(ActorExecutorConfig::Adapter(config)) => config.allowed_hostcalls.clone(),
        Some(ActorExecutorConfig::RemoteProxy(config)) => config.allowed_hostcalls.clone(),
        None => hostcalls_required_by_steps(suite, &actor.id),
    }
}

fn hostcalls_required_by_steps(suite: &Suite, actor_id: &str) -> Vec<String> {
    let mut hostcalls = OrderedSet::new();
    for step in &suite.steps {
        if step.primary_actor() == actor_id {
            hostcalls.insert(super::core::AdmissionRequest::from_step(step).action.as_str().to_string());
        }
    }
    hostcalls.into_iter().collect()
}

pub fn parse_executor_preflights(value: &IoValue) -> Result<ExecutorPreflightsEvidence> {
    let preflights = simple_record(value, "executor-preflights-v1", 2)?;
    let schema = required_string(&preflights[0], "executor preflights schema")?;
    if schema != crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported executor preflights schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA
        )));
    }
    let preflight_values = required_sequence(&preflights[1], "executor preflight entries")?;
    let mut entries = Vec::with_capacity(preflight_values.len());
    for preflight in preflight_values.iter() {
        entries.push(parse_executor_preflight(&value_to_iovalue(&preflight))?);
    }
    Ok(ExecutorPreflightsEvidence {
        value: value.clone(),
        preflights: entries,
    })
}

fn parse_executor_preflight(value: &IoValue) -> Result<ExecutorPreflightEvidence> {
    let preflight = value
        .collect_simple_record("executor-preflight-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <executor-preflight-v1 ...>"))?;
    let arity = preflight.fields_iter().count();
    if arity != 8 && arity != 9 {
        return Err(MoltenError::invalid_harness(format!(
            "expected <executor-preflight-v1 ...> with arity 8 or 9, got {arity}"
        )));
    }
    let schema = required_string(&preflight[0], "executor preflight schema")?;
    if schema != crate::preserves_rail::RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported executor preflight schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA
        )));
    }
    let actor_id = required_record_string(&preflight[1], "actor", "executor preflight actor")?;
    let kind = parse_actor_kind(&required_record_string(&preflight[2], "kind", "executor preflight kind")?)?;
    let artifact_ref = optional_executor_hash(&preflight[3], "artifact-ref", "executor artifact ref")?;
    let sandbox_ref = required_record_hash(&preflight[4], "sandbox-ref", "executor sandbox ref")?;
    let allowed_hostcalls =
        required_record_string_sequence(&preflight[5], "allowed-hostcalls", "executor allowed hostcalls")?;
    let conformance_refs =
        required_record_hash_sequence(&preflight[6], "conformance-suites", "executor conformance suites")?;
    let (executor_receipts, checks_index) = if arity == 9 {
        let receipts = required_record_iovalue_sequence(&preflight[7], "executor-receipts", "executor receipts")?;
        (receipts, 8)
    } else {
        (Vec::new(), 7)
    };
    let steel_review = parse_optional_steel_review_receipt(&executor_receipts)?;
    let wasm_inspection = parse_optional_wasm_inspection_receipt(&executor_receipts)?;
    let checks = parse_executor_preflight_checks(&preflight[checks_index])?;
    require_executor_preflight_check(&checks, "actor-kind-binding")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-binding")?;
    require_executor_preflight_check(&checks, "no-ambient-executor-io")?;
    Ok(ExecutorPreflightEvidence {
        value: value.clone(),
        actor_id,
        kind,
        artifact_ref,
        sandbox_ref,
        allowed_hostcalls,
        conformance_refs,
        executor_receipts,
        steel_review,
        wasm_inspection,
        checks,
    })
}

pub fn validate_executor_preflight_evidence(
    suite: &Suite,
    observations: &[Observation],
    preflights: Option<&ExecutorPreflightsEvidence>,
) -> Result<()> {
    let preflights = preflights.ok_or_else(|| MoltenError::invalid_harness("missing executor preflight evidence"))?;
    let expected = executor_preflights_value(suite)?;
    if preflights.value != expected {
        return Err(MoltenError::invalid_harness(format!(
            "executor preflight evidence mismatch: got {}, expected {}",
            canonical_hash(&preflights.value)?,
            canonical_hash(&expected)?
        )));
    }
    let by_actor = preflights_by_actor(&preflights.preflights)?;
    validate_actor_preflights(suite, &by_actor)?;
    validate_hostcall_preflight_bindings(observations, &by_actor)
}

type PreflightMap<'a> = std::collections::BTreeMap<&'a str, &'a ExecutorPreflightEvidence>;

fn preflights_by_actor(preflights: &[ExecutorPreflightEvidence]) -> Result<PreflightMap<'_>> {
    let mut by_actor = std::collections::BTreeMap::new();
    for preflight in preflights {
        if by_actor.insert(preflight.actor_id.as_str(), preflight).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate executor preflight for actor {}",
                preflight.actor_id
            )));
        }
    }
    Ok(by_actor)
}

fn validate_actor_preflights(suite: &Suite, by_actor: &PreflightMap<'_>) -> Result<()> {
    for actor in &suite.actors {
        let Some(preflight) = by_actor.get(actor.id.as_str()) else {
            return Err(MoltenError::invalid_harness(format!("missing executor preflight for actor {}", actor.id)));
        };
        if preflight.kind != actor.kind {
            return Err(MoltenError::invalid_harness(format!("executor kind binding mismatch for actor {}", actor.id)));
        }
        validate_actor_executor_preflight(actor, preflight)?;
    }
    Ok(())
}
