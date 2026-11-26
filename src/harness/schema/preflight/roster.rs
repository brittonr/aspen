use preserves::ValueImpl;

pub fn actor_registry_value(actors: &[super::model::ActorDecl]) -> preserves::IOValue {
    crate::preserves_rail::record("actor-registry-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_ACTOR_REGISTRY_SCHEMA),
        crate::preserves_rail::sequence(actors.iter().map(actor_decl_value).collect()),
    ])
}

fn actor_decl_value(actor: &super::model::ActorDecl) -> preserves::IOValue {
    let mut fields = vec![
        crate::preserves_rail::string(&actor.id),
        crate::preserves_rail::string(actor.kind.as_str()),
    ];
    if let Some(executor) = &actor.executor {
        fields.push(actor_executor_config_value(executor));
    }
    crate::preserves_rail::record("actor", fields)
}

fn actor_executor_config_value(config: &super::model::ActorExecutorConfig) -> preserves::IOValue {
    match config {
        super::model::ActorExecutorConfig::Steel(config) => steel_executor_config_value(config),
        super::model::ActorExecutorConfig::Wasm(config) => wasm_executor_config_value(config),
        super::model::ActorExecutorConfig::Adapter(config) => adapter_executor_config_value(config),
        super::model::ActorExecutorConfig::RemoteProxy(config) => remote_proxy_executor_config_value(config),
    }
}

fn steel_executor_config_value(config: &super::model::SteelExecutorConfig) -> preserves::IOValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    crate::preserves_rail::record("steel-executor-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_STEEL_EXECUTOR_SCHEMA),
        crate::preserves_rail::record("source", vec![crate::preserves_rail::string(&config.source)]),
        crate::preserves_rail::record("callable", vec![crate::preserves_rail::string(&config.callable)]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
    ])
}

fn wasm_executor_config_value(config: &super::model::WasmExecutorConfig) -> preserves::IOValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    crate::preserves_rail::record("wasm-executor-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_WASM_EXECUTOR_SCHEMA),
        crate::preserves_rail::record("module-hex", vec![crate::preserves_rail::string(&config.module_hex)]),
        crate::preserves_rail::record("wit", vec![crate::preserves_rail::string(&config.wit)]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
    ])
}

fn adapter_executor_config_value(config: &super::model::AdapterExecutorConfig) -> preserves::IOValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    crate::preserves_rail::record("adapter-executor-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_ADAPTER_EXECUTOR_SCHEMA),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(&config.manifest)]),
        crate::preserves_rail::record("abi", vec![crate::preserves_rail::string(&config.abi)]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("transcript", vec![crate::preserves_rail::string(&config.transcript)]),
    ])
}

fn remote_proxy_executor_config_value(config: &super::model::RemoteProxyExecutorConfig) -> preserves::IOValue {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    crate::preserves_rail::record("remote-proxy-executor-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(&config.peer)]),
        crate::preserves_rail::record("endpoint", vec![crate::preserves_rail::string(&config.endpoint)]),
        crate::preserves_rail::record("contract", vec![crate::preserves_rail::string(&config.contract)]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("transcript", vec![crate::preserves_rail::string(&config.transcript)]),
    ])
}

pub fn parse_actor_registry(value: &preserves::IOValue) -> crate::error::Result<Vec<super::model::ActorDecl>> {
    let registry = super::fields::simple_record(value, "actor-registry-v1", 2)?;
    let schema = super::fields::required_string(&registry[0], "actor registry schema")?;
    if schema != crate::preserves_rail::HARNESS_ACTOR_REGISTRY_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported actor registry schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_ACTOR_REGISTRY_SCHEMA
        )));
    }
    let actor_values = super::fields::required_sequence(&registry[1], "actor registry entries")?;
    let mut seen = std::collections::BTreeSet::new();
    let mut actors = Vec::with_capacity(actor_values.len());
    for actor in actor_values.iter() {
        let actor_value = crate::preserves_rail::value_to_iovalue(&actor);
        let actor = actor_value
            .collect_simple_record("actor", None)
            .ok_or_else(|| crate::error::Failure::invalid_harness("expected <actor ...> in actor registry"))?;
        let arity = actor.fields_iter().count();
        if arity != 2 && arity != 3 {
            return Err(crate::error::Failure::invalid_harness(format!(
                "actor registry entry arity must be 2 or 3, got {arity}"
            )));
        }
        let id = super::fields::required_string(&actor[0], "actor id")?;
        if !seen.insert(id.clone()) {
            return Err(crate::error::Failure::invalid_harness(format!("duplicate actor id {id}")));
        }
        let kind = parse_actor_kind(&super::fields::required_string(&actor[1], "actor kind")?)?;
        let executor = if arity == 3 {
            Some(super::setup::parse_actor_executor_config(
                &crate::preserves_rail::value_to_iovalue(&actor[2]),
                &kind,
                &id,
            )?)
        } else {
            None
        };
        actors.push(super::model::ActorDecl { id, kind, executor });
    }
    Ok(actors)
}

pub(super) fn normalize_allowed_hostcalls(values: Vec<String>) -> crate::error::Result<Vec<String>> {
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        parse_admission_action(&value)?;
        if !seen.insert(value.clone()) {
            return Err(crate::error::Failure::invalid_harness(format!("duplicate allowed hostcall {value}")));
        }
    }
    Ok(seen.into_iter().collect())
}

pub(super) fn parse_admission_action(action: &str) -> crate::error::Result<crate::runtime::AdmissionAction> {
    match action {
        "send" => Ok(crate::runtime::AdmissionAction::Send),
        "observe" => Ok(crate::runtime::AdmissionAction::Observe),
        "assert" => Ok(crate::runtime::AdmissionAction::Assert),
        "retract" => Ok(crate::runtime::AdmissionAction::Retract),
        "clock" => Ok(crate::runtime::AdmissionAction::Clock),
        "random" => Ok(crate::runtime::AdmissionAction::Random),
        other => Err(crate::error::Failure::invalid_harness(format!("unknown admission action {other}"))),
    }
}

pub(super) fn parse_actor_kind(kind: &str) -> crate::error::Result<super::model::ActorKind> {
    match kind {
        "native" => Ok(super::model::ActorKind::Native),
        "steel" => Ok(super::model::ActorKind::Steel),
        "wasm" => Ok(super::model::ActorKind::Wasm),
        "adapter" => Ok(super::model::ActorKind::Adapter),
        "remote-proxy" => Ok(super::model::ActorKind::RemoteProxy),
        other => Err(crate::error::Failure::invalid_harness(format!("unknown actor kind {other}"))),
    }
}
