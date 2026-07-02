
fn validate_hostcall_preflight_bindings(observations: &[Observation], by_actor: &PreflightMap<'_>) -> Result<()> {
    for (position, observation) in observations.iter().enumerate() {
        for event in &observation.events {
            validate_hostcall_preflight_event(position, event, by_actor)?;
        }
    }
    Ok(())
}

fn validate_hostcall_preflight_event(position: usize, event: &IoValue, by_actor: &PreflightMap<'_>) -> Result<()> {
    let Some(request) = event.collect_simple_record("hostcall-request-v1", None) else {
        return Ok(());
    };
    let arity = request.fields_iter().count();
    if arity != 9 && arity != 11 && arity != 15 {
        return Err(MoltenError::invalid_harness(format!(
            "hostcall request at observation {position} must have arity 9, 11, or 15, got {arity}"
        )));
    }
    let admission_request = parse_admission_request(&request[4])?;
    let Some(preflight) = by_actor.get(admission_request.actor.as_str()) else {
        return Err(MoltenError::invalid_harness(format!(
            "hostcall request at observation {position} has no executor preflight for actor {}",
            admission_request.actor
        )));
    };
    let allowed_hostcalls = preflight.allowed_hostcalls.as_slice();
    let operation = admission_request.action.as_str();
    if !allowed_hostcalls.iter().any(|allowed| allowed.as_str() == operation) {
        return Err(MoltenError::invalid_harness(format!(
            "hostcall operation {operation} at observation {position} is not allowed by executor preflight for actor {}",
            admission_request.actor
        )));
    }
    Ok(())
}

fn validate_actor_executor_preflight(actor: &ActorDecl, preflight: &ExecutorPreflightEvidence) -> Result<()> {
    let expected_conformance_refs = executor_conformance_refs(&preflight.allowed_hostcalls)?;
    if preflight.conformance_refs != expected_conformance_refs {
        return Err(MoltenError::invalid_harness(format!(
            "executor conformance suite refs mismatch for actor {}",
            actor.id
        )));
    }
    match (&actor.kind, &actor.executor) {
        (ActorKind::Native, None) => validate_native_preflight(actor, preflight),
        (ActorKind::Steel, Some(ActorExecutorConfig::Steel(config))) => {
            validate_steel_preflight(actor, preflight, config)
        }
        (ActorKind::Wasm, Some(ActorExecutorConfig::Wasm(config))) => validate_wasm_preflight(actor, preflight, config),
        (ActorKind::Adapter, Some(ActorExecutorConfig::Adapter(config))) => {
            validate_adapter_preflight(actor, preflight, config)
        }
        (ActorKind::RemoteProxy, Some(ActorExecutorConfig::RemoteProxy(config))) => {
            validate_remote_preflight(actor, preflight, config)
        }
        (ActorKind::Steel, None) => Err(MoltenError::invalid_harness(format!(
            "steel actor {} missing reviewed Steel executor preflight fixture",
            actor.id
        ))),
        (ActorKind::Wasm, None) => Err(MoltenError::invalid_harness(format!(
            "wasm actor {} missing Wasm executor preflight fixture",
            actor.id
        ))),
        (ActorKind::Adapter | ActorKind::RemoteProxy, _) => Err(MoltenError::invalid_harness(format!(
            "executor kind {} requires executor adapter preflight and remains disabled in local harness",
            actor.kind.as_str()
        ))),
        (ActorKind::Steel, Some(_)) | (ActorKind::Wasm, Some(_)) => Err(MoltenError::invalid_harness(format!(
            "actor {} kind {} has mismatched executor preflight fixture",
            actor.id,
            actor.kind.as_str()
        ))),
        (ActorKind::Native, Some(_)) => Err(MoltenError::invalid_harness(format!(
            "native actor {} must not declare non-native executor preflight fixture",
            actor.id
        ))),
    }
}

fn validate_native_preflight(actor: &ActorDecl, preflight: &ExecutorPreflightEvidence) -> Result<()> {
    if preflight.artifact_ref.is_some() || !preflight.executor_receipts.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "native executor preflight for actor {} must not carry artifact or review receipts",
            actor.id
        )));
    }
    require_executor_preflight_check(&preflight.checks, "native-local-executor")
}

fn validate_steel_preflight(
    actor: &ActorDecl,
    preflight: &ExecutorPreflightEvidence,
    config: &SteelExecutorConfig,
) -> Result<()> {
    require_executor_preflight_check(&preflight.checks, "steel-source-ref-binding")?;
    require_executor_preflight_check(&preflight.checks, "steel-callable-review")?;
    require_executor_preflight_check(&preflight.checks, "steel-hostcall-contract")?;
    let source_ref = steel_source_ref(config)?;
    if preflight.artifact_ref.as_deref() != Some(source_ref.as_str()) {
        return Err(MoltenError::invalid_harness(format!(
            "Steel executor preflight source ref mismatch for actor {}",
            actor.id
        )));
    }
    let review = preflight.steel_review.as_ref().ok_or_else(|| {
        MoltenError::invalid_harness(format!("Steel executor preflight missing review receipt for actor {}", actor.id))
    })?;
    if review.source_ref != source_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Steel review receipt source ref mismatch for actor {}",
            actor.id
        )));
    }
    if review.callable != config.callable {
        return Err(MoltenError::invalid_harness(format!(
            "Steel review receipt callable mismatch for actor {}",
            actor.id
        )));
    }
    if review.allowed_hostcalls != config.allowed_hostcalls || preflight.allowed_hostcalls != config.allowed_hostcalls {
        return Err(MoltenError::invalid_harness(format!(
            "Steel review receipt allowed hostcalls mismatch for actor {}",
            actor.id
        )));
    }
    Ok(())
}

fn validate_wasm_preflight(
    actor: &ActorDecl,
    preflight: &ExecutorPreflightEvidence,
    config: &WasmExecutorConfig,
) -> Result<()> {
    require_executor_preflight_check(&preflight.checks, "wasm-module-ref-binding")?;
    require_executor_preflight_check(&preflight.checks, "wasmparser-inspection")?;
    require_executor_preflight_check(&preflight.checks, "wasm-deny-by-default-wasi")?;
    require_executor_preflight_check(&preflight.checks, "wasm-hostcall-contract")?;
    require_executor_preflight_check(&preflight.checks, "wit-interface-binding")?;
    let module_ref = wasm_module_ref(config)?;
    if preflight.artifact_ref.as_deref() != Some(module_ref.as_str()) {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor preflight module ref mismatch for actor {}",
            actor.id
        )));
    }
    let inspection = preflight.wasm_inspection.as_ref().ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "Wasm executor preflight missing inspection receipt for actor {}",
            actor.id
        ))
    })?;
    if inspection.module_ref != module_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm inspection receipt module ref mismatch for actor {}",
            actor.id
        )));
    }
    if inspection.wit_ref != wasm_wit_ref(config)? {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm inspection receipt WIT ref mismatch for actor {}",
            actor.id
        )));
    }
    if inspection.allowed_hostcalls != config.allowed_hostcalls
        || preflight.allowed_hostcalls != config.allowed_hostcalls
    {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm inspection receipt allowed hostcalls mismatch for actor {}",
            actor.id
        )));
    }
    validate_wasm_imports(&actor.id, &inspection.imports, &config.allowed_hostcalls)
}

fn validate_adapter_preflight(
    actor: &ActorDecl,
    preflight: &ExecutorPreflightEvidence,
    config: &AdapterExecutorConfig,
) -> Result<()> {
    require_executor_preflight_check(&preflight.checks, "adapter-manifest-binding")?;
    require_executor_preflight_check(&preflight.checks, "adapter-permission-binding")?;
    require_executor_preflight_check(&preflight.checks, "adapter-transcript-replay")?;
    let manifest_ref = adapter_manifest_ref(config)?;
    if preflight.artifact_ref.as_deref() != Some(manifest_ref.as_str()) {
        return Err(MoltenError::invalid_harness(format!(
            "adapter executor preflight manifest ref mismatch for actor {}",
            actor.id
        )));
    }
    if preflight.allowed_hostcalls != config.allowed_hostcalls {
        return Err(MoltenError::invalid_harness(format!(
            "adapter executor preflight allowed hostcalls mismatch for actor {}",
            actor.id
        )));
    }
    Ok(())
}

fn validate_remote_preflight(
    actor: &ActorDecl,
    preflight: &ExecutorPreflightEvidence,
    config: &RemoteProxyExecutorConfig,
) -> Result<()> {
    require_executor_preflight_check(&preflight.checks, "remote-peer-binding")?;
    require_executor_preflight_check(&preflight.checks, "remote-contract-binding")?;
    require_executor_preflight_check(&preflight.checks, "remote-transcript-replay")?;
    let endpoint_ref = remote_proxy_endpoint_ref(config)?;
    if preflight.artifact_ref.as_deref() != Some(endpoint_ref.as_str()) {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy executor preflight endpoint ref mismatch for actor {}",
            actor.id
        )));
    }
    if preflight.allowed_hostcalls != config.allowed_hostcalls {
        return Err(MoltenError::invalid_harness(format!(
            "remote-proxy executor preflight allowed hostcalls mismatch for actor {}",
            actor.id
        )));
    }
    Ok(())
}

fn parse_optional_steel_review_receipt(receipts: &[IoValue]) -> Result<Option<SteelReviewReceipt>> {
    let mut parsed = None;
    for receipt in receipts {
        if receipt.collect_simple_record("steel-review-receipt-v1", None).is_some() {
            if parsed.is_some() {
                return Err(MoltenError::invalid_harness("duplicate Steel review receipt in executor preflight"));
            }
            parsed = Some(parse_steel_review_receipt(receipt)?);
        }
    }
    Ok(parsed)
}

fn parse_steel_review_receipt(value: &IoValue) -> Result<SteelReviewReceipt> {
    let receipt = simple_record(value, "steel-review-receipt-v1", 6)?;
    let schema = required_string(&receipt[0], "Steel review receipt schema")?;
    if schema != crate::preserves_rail::RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Steel review receipt schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "Steel review receipt decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported Steel review receipt decision {decision}")));
    }
    let source_ref = required_record_hash(&receipt[2], "source-ref", "Steel review receipt source ref")?;
    let callable = required_record_string(&receipt[3], "callable", "Steel review receipt callable")?;
    let allowed_hostcalls =
        required_record_string_sequence(&receipt[4], "allowed-hostcalls", "Steel review receipt allowed hostcalls")?;
    let checks = parse_executor_preflight_checks(&receipt[5])?;
    require_executor_preflight_check(&checks, "source-ref-binding")?;
    require_executor_preflight_check(&checks, "reviewed-callable")?;
    require_executor_preflight_check(&checks, "allowed-hostcall-contract")?;
    require_executor_preflight_check(&checks, "no-ambient-steel-io")?;
    Ok(SteelReviewReceipt {
        value: value.clone(),
        source_ref,
        callable,
        allowed_hostcalls,
        checks,
    })
}

fn parse_optional_wasm_inspection_receipt(receipts: &[IoValue]) -> Result<Option<WasmInspectionReceipt>> {
    let mut parsed = None;
    for receipt in receipts {
        if receipt.collect_simple_record("wasm-inspection-receipt-v1", None).is_some() {
            if parsed.is_some() {
                return Err(MoltenError::invalid_harness("duplicate Wasm inspection receipt in executor preflight"));
            }
            parsed = Some(parse_wasm_inspection_receipt(receipt)?);
        }
    }
    Ok(parsed)
}
