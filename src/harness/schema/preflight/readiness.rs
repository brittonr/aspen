pub fn executor_preflights_value(suite: &super::model::Suite) -> crate::error::Result<preserves::IOValue> {
    validate_executor_preflight_inputs(suite)?;
    Ok(crate::preserves_rail::record("executor-preflights-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_EXECUTOR_PREFLIGHTS_SCHEMA),
        crate::preserves_rail::sequence(
            suite
                .actors
                .iter()
                .map(|actor| executor_preflight_value(actor, &allowed_hostcalls_for_actor(suite, actor)))
                .collect::<crate::error::Result<Vec<_>>>()?,
        ),
    ]))
}

pub fn validate_executor_preflight_inputs(suite: &super::model::Suite) -> crate::error::Result<()> {
    for actor in &suite.actors {
        match (&actor.kind, &actor.executor) {
            (super::model::ActorKind::Native, None) => {}
            (super::model::ActorKind::Native, Some(_)) => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "native actor {} must not declare non-native executor preflight fixture",
                    actor.id
                )));
            }
            (super::model::ActorKind::Steel, Some(super::model::ActorExecutorConfig::Steel(config))) => {
                super::sources::validate_steel_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "Steel")?;
            }
            (super::model::ActorKind::Wasm, Some(super::model::ActorExecutorConfig::Wasm(config))) => {
                super::introspect::validate_wasm_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "Wasm")?;
            }
            (super::model::ActorKind::Adapter, Some(super::model::ActorExecutorConfig::Adapter(config))) => {
                super::sources::validate_adapter_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "adapter")?;
            }
            (super::model::ActorKind::RemoteProxy, Some(super::model::ActorExecutorConfig::RemoteProxy(config))) => {
                super::sources::validate_remote_proxy_executor_config(&actor.id, config)?;
                validate_required_hostcalls_allowed(suite, actor, &config.allowed_hostcalls, "remote-proxy")?;
            }
            (super::model::ActorKind::Steel, None) => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "steel actor {} missing reviewed Steel executor preflight fixture",
                    actor.id
                )));
            }
            (super::model::ActorKind::Wasm, None) => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "wasm actor {} missing Wasm executor preflight fixture",
                    actor.id
                )));
            }
            (super::model::ActorKind::Adapter, None) | (super::model::ActorKind::RemoteProxy, None) => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "executor kind {} requires executor adapter preflight and remains disabled in local harness",
                    actor.kind.as_str()
                )));
            }
            (super::model::ActorKind::Steel, Some(_))
            | (super::model::ActorKind::Wasm, Some(_))
            | (super::model::ActorKind::Adapter, Some(_))
            | (super::model::ActorKind::RemoteProxy, Some(_)) => {
                return Err(crate::error::Failure::invalid_harness(format!(
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
    suite: &super::model::Suite,
    actor: &super::model::ActorDecl,
    allowed_hostcalls: &[String],
    executor_name: &str,
) -> crate::error::Result<()> {
    let required_hostcalls = hostcalls_required_by_steps(suite, &actor.id);
    for operation in required_hostcalls {
        if !allowed_hostcalls.iter().any(|allowed| allowed.as_str() == operation.as_str()) {
            return Err(crate::error::Failure::invalid_harness(format!(
                "hostcall operation {operation} is not allowed by {executor_name} executor preflight for actor {}",
                actor.id
            )));
        }
    }
    Ok(())
}

pub(super) fn executor_preflight_value(
    actor: &super::model::ActorDecl,
    allowed_hostcalls: &[String],
) -> crate::error::Result<preserves::IOValue> {
    let sandbox = executor_sandbox_value(&actor.kind);
    let sandbox_ref = crate::preserves_rail::canonical_hash(&sandbox)?;
    let conformance_refs: Vec<std::string::String> = executor_conformance_refs(allowed_hostcalls)?;
    let (artifact_ref, receipts, checks) = match (&actor.kind, &actor.executor) {
        (super::model::ActorKind::Native, None) => (None, Vec::new(), executor_preflight_checks(&actor.kind).to_vec()),
        (super::model::ActorKind::Steel, Some(super::model::ActorExecutorConfig::Steel(config))) => {
            let source_ref = super::sources::steel_source_ref(config)?;
            let receipt = super::sources::steel_review_receipt_value(config)?;
            (Some(source_ref), vec![receipt], steel_executor_preflight_checks().to_vec())
        }
        (super::model::ActorKind::Wasm, Some(super::model::ActorExecutorConfig::Wasm(config))) => {
            let module_ref = super::introspect::wasm_module_ref(config)?;
            let receipt = super::introspect::wasm_inspection_receipt_value(config)?;
            (Some(module_ref), vec![receipt], wasm_executor_preflight_checks().to_vec())
        }
        (super::model::ActorKind::Adapter, Some(super::model::ActorExecutorConfig::Adapter(config))) => {
            let manifest_ref = super::sources::adapter_manifest_ref(config)?;
            let receipt = super::sources::adapter_preflight_receipt_value(config)?;
            (Some(manifest_ref), vec![receipt], adapter_executor_preflight_checks().to_vec())
        }
        (super::model::ActorKind::RemoteProxy, Some(super::model::ActorExecutorConfig::RemoteProxy(config))) => {
            let endpoint_ref = super::sources::remote_proxy_endpoint_ref(config)?;
            let receipt = super::sources::remote_proxy_preflight_receipt_value(config)?;
            (Some(endpoint_ref), vec![receipt], remote_proxy_executor_preflight_checks().to_vec())
        }
        _ => {
            return Err(crate::error::Failure::invalid_harness(format!(
                "unsupported executor preflight fixture for actor {} kind {}",
                actor.id,
                actor.kind.as_str()
            )));
        }
    };
    Ok(crate::preserves_rail::record("executor-preflight-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_EXECUTOR_PREFLIGHT_SCHEMA),
        crate::preserves_rail::record("actor", vec![crate::preserves_rail::string(&actor.id)]),
        crate::preserves_rail::record("kind", vec![crate::preserves_rail::string(actor.kind.as_str())]),
        crate::preserves_rail::record("artifact-ref", vec![super::events::optional_string_value(
            artifact_ref.as_deref(),
        )]),
        crate::preserves_rail::record("sandbox-ref", vec![crate::preserves_rail::string(sandbox_ref)]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("conformance-suites", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::string(&conformance_refs[0]),
        ])]),
        crate::preserves_rail::record("executor-receipts", vec![crate::preserves_rail::sequence(receipts)]),
        super::calls::hostcall_checks_value(&checks),
    ]))
}

pub(super) fn executor_conformance_refs(allowed_hostcalls: &[String]) -> crate::error::Result<Vec<String>> {
    Ok(vec![crate::preserves_rail::canonical_hash(
        &executor_conformance_suite_value(allowed_hostcalls),
    )?])
}

fn executor_conformance_suite_value(allowed_hostcalls: &[String]) -> preserves::IOValue {
    crate::preserves_rail::record("executor-conformance-suite-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_EXECUTOR_CONFORMANCE_SCHEMA),
        crate::preserves_rail::record("boundary", vec![crate::preserves_rail::string(
            "molten.runtime.executor-hostcall-boundary.v1",
        )]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("actor-input", vec![crate::preserves_rail::string(
            crate::preserves_rail::RUNTIME_ACTOR_INPUT_SCHEMA,
        )]),
        crate::preserves_rail::record("hostcall-request", vec![crate::preserves_rail::string(
            crate::preserves_rail::RUNTIME_HOSTCALL_REQUEST_SCHEMA,
        )]),
        crate::preserves_rail::record("hostcall-decision", vec![crate::preserves_rail::string(
            crate::preserves_rail::RUNTIME_HOSTCALL_DECISION_SCHEMA,
        )]),
        crate::preserves_rail::record("actor-output", vec![crate::preserves_rail::string(
            crate::preserves_rail::RUNTIME_ACTOR_OUTPUT_SCHEMA,
        )]),
        super::calls::hostcall_checks_value(&[
            "canonical-preserves",
            "hostcall-admission-binding",
            "deterministic-replay",
            "no-ambient-executor-io",
            "cross-kind-compatible",
        ]),
    ])
}

fn executor_sandbox_value(kind: &super::model::ActorKind) -> preserves::IOValue {
    crate::preserves_rail::record("executor-sandbox-v1", vec![
        crate::preserves_rail::record("kind", vec![crate::preserves_rail::string(kind.as_str())]),
        crate::preserves_rail::record("ambient-io", vec![crate::preserves_rail::bool_value(false)]),
        crate::preserves_rail::record("hostcalls-only", vec![crate::preserves_rail::bool_value(true)]),
    ])
}

fn executor_preflight_checks(kind: &super::model::ActorKind) -> &'static [&'static str] {
    match kind {
        super::model::ActorKind::Native => &[
            "actor-kind-binding",
            "allowed-hostcall-binding",
            "no-ambient-executor-io",
            "native-local-executor",
        ],
        super::model::ActorKind::Steel
        | super::model::ActorKind::Wasm
        | super::model::ActorKind::Adapter
        | super::model::ActorKind::RemoteProxy => &[
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

pub(super) fn allowed_hostcalls_for_actor(suite: &super::model::Suite, actor: &super::model::ActorDecl) -> Vec<String> {
    match &actor.executor {
        Some(super::model::ActorExecutorConfig::Steel(config)) => config.allowed_hostcalls.clone(),
        Some(super::model::ActorExecutorConfig::Wasm(config)) => config.allowed_hostcalls.clone(),
        Some(super::model::ActorExecutorConfig::Adapter(config)) => config.allowed_hostcalls.clone(),
        Some(super::model::ActorExecutorConfig::RemoteProxy(config)) => config.allowed_hostcalls.clone(),
        None => hostcalls_required_by_steps(suite, &actor.id),
    }
}

fn hostcalls_required_by_steps(suite: &super::model::Suite, actor_id: &str) -> Vec<String> {
    let mut hostcalls = std::collections::BTreeSet::new();
    for step in &suite.steps {
        if step.primary_actor() == actor_id {
            hostcalls.insert(crate::harness::nucleus::AdmissionRequest::from_step(step).action.as_str().to_string());
        }
    }
    hostcalls.into_iter().collect()
}
