
pub fn parse_repro_bundle(value: &IoValue) -> Result<ReproBundle> {
    let bundle = value
        .collect_simple_record("harness-repro-bundle-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <harness-repro-bundle-v1 ...>"))?;
    let arity = bundle.fields_iter().count();
    let schema = required_string(&bundle[0], "repro bundle schema")?;
    if schema != crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported repro bundle schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_REPRO_BUNDLE_SCHEMA
        )));
    }

    if arity == 11 {
        return parse_legacy_report_repro_bundle(value, &bundle);
    }
    if arity == 16 {
        return parse_report_repro_bundle(value, &bundle);
    }
    if arity == 19 || arity == 21 {
        return parse_sealed_report_repro_bundle(value, &bundle);
    }
    if arity == 23 || arity == 24 {
        return parse_profiled_report_repro_bundle(value, &bundle);
    }
    if arity == 8 {
        return parse_failure_repro_bundle(value, &bundle);
    }
    Err(MoltenError::invalid_harness(format!(
        "expected <harness-repro-bundle-v1 ...> with arity 8, 11, 16, 19, 21, 23, or 24, got {arity}"
    )))
}

pub fn repro_bundle_report_value(bundle_value: &IoValue) -> Result<IoValue> {
    let bundle = parse_repro_bundle(bundle_value)?;
    match (bundle.kind, bundle.report_value) {
        (ReproBundleKind::Report, Some(report_value)) => Ok(report_value),
        (ReproBundleKind::Failure, _) => Err(MoltenError::invalid_harness(format!(
            "failure repro bundle {} cannot satisfy pass evidence gate",
            bundle.bundle_ref
        ))),
        (ReproBundleKind::Report, None) => {
            Err(MoltenError::invalid_harness("report repro bundle missing report value"))
        }
    }
}

pub fn repro_bundle_summary(bundle_value: &IoValue) -> Result<String> {
    let bundle = parse_repro_bundle(bundle_value)?;
    let gate_receipt = bundle.gate_receipt_ref.as_deref().unwrap_or("none");
    let export_profile = bundle.export_profile.as_deref().unwrap_or("legacy");
    let loss_classification = bundle.loss_classification.as_deref().unwrap_or("unknown");
    Ok(format!(
        "repro bundle {}\nkind={}\nartifact={}\ngate_receipt={}\nprofile={}\nloss_classification={}",
        bundle.bundle_ref,
        match bundle.kind {
            ReproBundleKind::Report => "report",
            ReproBundleKind::Failure => "failure",
        },
        bundle.artifact_ref,
        gate_receipt,
        export_profile,
        loss_classification
    ))
}

pub fn actor_registry_value(actors: &[ActorDecl]) -> IoValue {
    record("actor-registry-v1", vec![
        string(crate::preserves_rail::HARNESS_ACTOR_REGISTRY_SCHEMA),
        sequence(actors.iter().map(actor_decl_value).collect()),
    ])
}

fn actor_decl_value(actor: &ActorDecl) -> IoValue {
    let mut fields = vec![string(&actor.id), string(actor.kind.as_str())];
    if let Some(executor) = &actor.executor {
        fields.push(actor_executor_config_value(executor));
    }
    record("actor", fields)
}

fn actor_executor_config_value(config: &ActorExecutorConfig) -> IoValue {
    match config {
        ActorExecutorConfig::Steel(config) => steel_executor_config_value(config),
        ActorExecutorConfig::Wasm(config) => wasm_executor_config_value(config),
        ActorExecutorConfig::Adapter(config) => adapter_executor_config_value(config),
        ActorExecutorConfig::RemoteProxy(config) => remote_proxy_executor_config_value(config),
    }
}

fn steel_executor_config_value(config: &SteelExecutorConfig) -> IoValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    record("steel-executor-v1", vec![
        string(crate::preserves_rail::RUNTIME_STEEL_EXECUTOR_SCHEMA),
        record("source", vec![string(&config.source)]),
        record("callable", vec![string(&config.callable)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
    ])
}

fn wasm_executor_config_value(config: &WasmExecutorConfig) -> IoValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    record("wasm-executor-v1", vec![
        string(crate::preserves_rail::RUNTIME_WASM_EXECUTOR_SCHEMA),
        record("module-hex", vec![string(&config.module_hex)]),
        record("wit", vec![string(&config.wit)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
    ])
}

fn adapter_executor_config_value(config: &AdapterExecutorConfig) -> IoValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    record("adapter-executor-v1", vec![
        string(crate::preserves_rail::RUNTIME_ADAPTER_EXECUTOR_SCHEMA),
        record("manifest", vec![string(&config.manifest)]),
        record("abi", vec![string(&config.abi)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("transcript", vec![string(&config.transcript)]),
    ])
}

fn remote_proxy_executor_config_value(config: &RemoteProxyExecutorConfig) -> IoValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    record("remote-proxy-executor-v1", vec![
        string(crate::preserves_rail::RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA),
        record("peer", vec![string(&config.peer)]),
        record("endpoint", vec![string(&config.endpoint)]),
        record("contract", vec![string(&config.contract)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("transcript", vec![string(&config.transcript)]),
    ])
}

pub fn executor_preflights_value(suite: &Suite) -> Result<IoValue> {
    validate_executor_preflight_inputs(suite)?;
    Ok(record("executor-preflights-v1", vec![
        string(crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA),
        sequence(
            suite
                .actors
                .iter()
                .map(|actor| executor_preflight_value(actor, &allowed_hostcalls_for_actor(suite, actor)))
                .collect::<Result<Vec<_>>>()?,
        ),
    ]))
}

pub fn validate_executor_preflight_inputs(suite: &Suite) -> Result<()> {
    for actor in &suite.actors {
        match (&actor.kind, &actor.executor) {
            (ActorKind::Native, None) => {}
            (ActorKind::Native, Some(_)) => {
                return Err(MoltenError::invalid_harness(format!(
                    "native actor {} must not declare non-native executor preflight fixture",
                    actor.id
                )));
            }
            (ActorKind::Steel, Some(ActorExecutorConfig::Steel(config))) => {
                validate_steel_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "Steel")?;
            }
            (ActorKind::Wasm, Some(ActorExecutorConfig::Wasm(config))) => {
                validate_wasm_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "Wasm")?;
            }
            (ActorKind::Adapter, Some(ActorExecutorConfig::Adapter(config))) => {
                validate_adapter_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "adapter")?;
            }
            (ActorKind::RemoteProxy, Some(ActorExecutorConfig::RemoteProxy(config))) => {
                validate_remote_proxy_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "remote-proxy")?;
            }
            (ActorKind::Steel, None) => {
                return Err(MoltenError::invalid_harness(format!(
                    "steel actor {} missing reviewed Steel executor preflight fixture",
                    actor.id
                )));
            }
            (ActorKind::Wasm, None) => {
                return Err(MoltenError::invalid_harness(format!(
                    "wasm actor {} missing Wasm executor preflight fixture",
                    actor.id
                )));
            }
            (ActorKind::Adapter, None) | (ActorKind::RemoteProxy, None) => {
                return Err(MoltenError::invalid_harness(format!(
                    "executor kind {} requires executor adapter preflight and remains disabled in local harness",
                    actor.kind.as_str()
                )));
            }
            (ActorKind::Steel, Some(_))
            | (ActorKind::Wasm, Some(_))
            | (ActorKind::Adapter, Some(_))
            | (ActorKind::RemoteProxy, Some(_)) => {
                return Err(MoltenError::invalid_harness(format!(
                    "actor {} kind {} has mismatched executor preflight fixture",
                    actor.id,
                    actor.kind.as_str()
                )));
            }
        }
    }
    Ok(())
}

fn validate_required_hostcalls_allowed(
    suite: &Suite,
    actor: &ActorDecl,
    allowed_hostcalls: &[String],
    executor_name: &str,
) -> Result<()> {
    let required_hostcalls = hostcalls_required_by_steps(suite, &actor.id);
    for operation in required_hostcalls {
        if !allowed_hostcalls.iter().any(|allowed| allowed.as_str() == operation.as_str()) {
            return Err(MoltenError::invalid_harness(format!(
                "hostcall operation {operation} is not allowed by {executor_name} executor preflight for actor {}",
                actor.id
            )));
        }
    }
    Ok(())
}

fn executor_preflight_value(actor: &ActorDecl, allowed_hostcalls: &[String]) -> Result<IoValue> {
    let sandbox = executor_sandbox_value(&actor.kind);
    let sandbox_ref = canonical_hash(&sandbox)?;
    let conformance_refs: Vec<std::string::String> = executor_conformance_refs(allowed_hostcalls)?;
    let (artifact_ref, receipts, checks) = match (&actor.kind, &actor.executor) {
        (ActorKind::Native, None) => (None, Vec::new(), executor_preflight_checks(&actor.kind).to_vec()),
        (ActorKind::Steel, Some(ActorExecutorConfig::Steel(config))) => {
            let source_ref = steel_source_ref(config)?;
            let receipt = steel_review_receipt_value(config)?;
            (Some(source_ref), vec![receipt], steel_executor_preflight_checks().to_vec())
        }
        (ActorKind::Wasm, Some(ActorExecutorConfig::Wasm(config))) => {
            let module_ref = wasm_module_ref(config)?;
            let receipt = wasm_inspection_receipt_value(config)?;
            (Some(module_ref), vec![receipt], wasm_executor_preflight_checks().to_vec())
        }
        (ActorKind::Adapter, Some(ActorExecutorConfig::Adapter(config))) => {
            let manifest_ref = adapter_manifest_ref(config)?;
            let receipt = adapter_preflight_receipt_value(config)?;
            (Some(manifest_ref), vec![receipt], adapter_executor_preflight_checks().to_vec())
        }
        (ActorKind::RemoteProxy, Some(ActorExecutorConfig::RemoteProxy(config))) => {
            let endpoint_ref = remote_proxy_endpoint_ref(config)?;
            let receipt = remote_proxy_preflight_receipt_value(config)?;
            (Some(endpoint_ref), vec![receipt], remote_proxy_executor_preflight_checks().to_vec())
        }
        _ => {
            return Err(MoltenError::invalid_harness(format!(
                "unsupported executor preflight fixture for actor {} kind {}",
                actor.id,
                actor.kind.as_str()
            )));
        }
    };
    Ok(record("executor-preflight-v1", vec![
        string(crate::preserves_rail::RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA),
        record("actor", vec![string(&actor.id)]),
        record("kind", vec![string(actor.kind.as_str())]),
        record("artifact-ref", vec![optional_string_value(artifact_ref.as_deref())]),
        record("sandbox-ref", vec![string(sandbox_ref)]),
        record("allowed-hostcalls", vec![sequence(
            allowed_hostcalls.iter().map(|hostcall: &String| string(hostcall.as_str())).collect::<Vec<_>>(),
        )]),
        record("conformance-suites", vec![sequence(vec![string(&conformance_refs[0])])]),
        record("executor-receipts", vec![sequence(receipts)]),
        hostcall_checks_value(&checks),
    ]))
}

fn executor_conformance_refs(allowed_hostcalls: &[String]) -> Result<Vec<String>> {
    Ok(vec![canonical_hash(&executor_conformance_suite_value(allowed_hostcalls))?])
}
