pub(super) fn validate_native_preflight(
    actor: &super::model::ActorDecl,
    preflight: &super::observed::ExecutorPreflightEvidence,
) -> crate::error::Result<()> {
    if preflight.artifact_ref.is_some() || !preflight.executor_receipts.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "native executor preflight for actor {} must not carry artifact or review receipts",
            actor.id
        )));
    }
    super::clearance::require_executor_preflight_check(&preflight.checks, "native-local-executor")
}

pub(super) fn validate_steel_preflight(
    actor: &super::model::ActorDecl,
    preflight: &super::observed::ExecutorPreflightEvidence,
    config: &super::model::SteelExecutorConfig,
) -> crate::error::Result<()> {
    super::clearance::require_executor_preflight_check(&preflight.checks, "steel-source-ref-binding")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "steel-callable-review")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "steel-hostcall-contract")?;
    let source_ref = super::sources::steel_source_ref(config)?;
    if preflight.artifact_ref.as_deref() != Some(source_ref.as_str()) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel executor preflight source ref mismatch for actor {}",
            actor.id
        )));
    }
    let review = preflight.steel_review.as_ref().ok_or_else(|| {
        crate::error::Failure::invalid_harness(format!(
            "Steel executor preflight missing review receipt for actor {}",
            actor.id
        ))
    })?;
    if review.source_ref != source_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel review receipt source ref mismatch for actor {}",
            actor.id
        )));
    }
    if review.callable != config.callable {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel review receipt callable mismatch for actor {}",
            actor.id
        )));
    }
    if review.allowed_hostcalls != config.allowed_hostcalls || preflight.allowed_hostcalls != config.allowed_hostcalls {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel review receipt allowed hostcalls mismatch for actor {}",
            actor.id
        )));
    }
    Ok(())
}

pub(super) fn validate_wasm_preflight(
    actor: &super::model::ActorDecl,
    preflight: &super::observed::ExecutorPreflightEvidence,
    config: &super::model::WasmExecutorConfig,
) -> crate::error::Result<()> {
    super::clearance::require_executor_preflight_check(&preflight.checks, "wasm-module-ref-binding")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "wasmparser-inspection")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "wasm-deny-by-default-wasi")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "wasm-hostcall-contract")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "wit-interface-binding")?;
    let module_ref = super::introspect::wasm_module_ref(config)?;
    if preflight.artifact_ref.as_deref() != Some(module_ref.as_str()) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm executor preflight module ref mismatch for actor {}",
            actor.id
        )));
    }
    let inspection = preflight.wasm_inspection.as_ref().ok_or_else(|| {
        crate::error::Failure::invalid_harness(format!(
            "Wasm executor preflight missing inspection receipt for actor {}",
            actor.id
        ))
    })?;
    if inspection.module_ref != module_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm inspection receipt module ref mismatch for actor {}",
            actor.id
        )));
    }
    if inspection.wit_ref != super::introspect::wasm_wit_ref(config)? {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm inspection receipt WIT ref mismatch for actor {}",
            actor.id
        )));
    }
    if inspection.allowed_hostcalls != config.allowed_hostcalls
        || preflight.allowed_hostcalls != config.allowed_hostcalls
    {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm inspection receipt allowed hostcalls mismatch for actor {}",
            actor.id
        )));
    }
    super::introspect::validate_wasm_imports(&actor.id, &inspection.imports, &config.allowed_hostcalls)
}

pub(super) fn validate_adapter_preflight(
    actor: &super::model::ActorDecl,
    preflight: &super::observed::ExecutorPreflightEvidence,
    config: &super::model::AdapterExecutorConfig,
) -> crate::error::Result<()> {
    super::clearance::require_executor_preflight_check(&preflight.checks, "adapter-manifest-binding")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "adapter-permission-binding")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "adapter-transcript-replay")?;
    let manifest_ref = super::sources::adapter_manifest_ref(config)?;
    if preflight.artifact_ref.as_deref() != Some(manifest_ref.as_str()) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "adapter executor preflight manifest ref mismatch for actor {}",
            actor.id
        )));
    }
    if preflight.allowed_hostcalls != config.allowed_hostcalls {
        return Err(crate::error::Failure::invalid_harness(format!(
            "adapter executor preflight allowed hostcalls mismatch for actor {}",
            actor.id
        )));
    }
    Ok(())
}

pub(super) fn validate_remote_preflight(
    actor: &super::model::ActorDecl,
    preflight: &super::observed::ExecutorPreflightEvidence,
    config: &super::model::RemoteProxyExecutorConfig,
) -> crate::error::Result<()> {
    super::clearance::require_executor_preflight_check(&preflight.checks, "remote-peer-binding")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "remote-contract-binding")?;
    super::clearance::require_executor_preflight_check(&preflight.checks, "remote-transcript-replay")?;
    let endpoint_ref = super::sources::remote_proxy_endpoint_ref(config)?;
    if preflight.artifact_ref.as_deref() != Some(endpoint_ref.as_str()) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "remote-proxy executor preflight endpoint ref mismatch for actor {}",
            actor.id
        )));
    }
    if preflight.allowed_hostcalls != config.allowed_hostcalls {
        return Err(crate::error::Failure::invalid_harness(format!(
            "remote-proxy executor preflight allowed hostcalls mismatch for actor {}",
            actor.id
        )));
    }
    Ok(())
}
