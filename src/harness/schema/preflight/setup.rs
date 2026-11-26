use preserves::ValueImpl;

pub(super) fn parse_actor_executor_config(
    value: &preserves::IOValue,
    kind: &super::model::ActorKind,
    actor_id: &str,
) -> crate::error::Result<super::model::ActorExecutorConfig> {
    if value.collect_simple_record("steel-executor-v1", None).is_some() {
        if kind != &super::model::ActorKind::Steel {
            return Err(crate::error::Failure::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use Steel executor config",
                kind.as_str()
            )));
        }
        return parse_steel_executor_config(value).map(super::model::ActorExecutorConfig::Steel);
    }
    if value.collect_simple_record("wasm-executor-v1", None).is_some() {
        if kind != &super::model::ActorKind::Wasm {
            return Err(crate::error::Failure::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use Wasm executor config",
                kind.as_str()
            )));
        }
        return parse_wasm_executor_config(value).map(super::model::ActorExecutorConfig::Wasm);
    }
    if value.collect_simple_record("adapter-executor-v1", None).is_some() {
        if kind != &super::model::ActorKind::Adapter {
            return Err(crate::error::Failure::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use adapter executor config",
                kind.as_str()
            )));
        }
        return parse_adapter_executor_config(value).map(super::model::ActorExecutorConfig::Adapter);
    }
    if value.collect_simple_record("remote-proxy-executor-v1", None).is_some() {
        if kind != &super::model::ActorKind::RemoteProxy {
            return Err(crate::error::Failure::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use remote-proxy executor config",
                kind.as_str()
            )));
        }
        return parse_remote_proxy_executor_config(value).map(super::model::ActorExecutorConfig::RemoteProxy);
    }
    Err(crate::error::Failure::invalid_harness(format!(
        "unsupported executor config for actor {actor_id}; expected <steel-executor-v1 ...>, <wasm-executor-v1 ...>, <adapter-executor-v1 ...>, or <remote-proxy-executor-v1 ...>"
    )))
}

fn parse_steel_executor_config(value: &preserves::IOValue) -> crate::error::Result<super::model::SteelExecutorConfig> {
    let config = super::fields::simple_record(value, "steel-executor-v1", 4)?;
    let schema = super::fields::required_string(&config[0], "Steel executor schema")?;
    if schema != crate::preserves_rail::RUNTIME_STEEL_EXECUTOR_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Steel executor schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_STEEL_EXECUTOR_SCHEMA
        )));
    }
    let source = super::fields::required_record_string(&config[1], "source", "Steel executor source")?;
    let callable = super::fields::required_record_string(&config[2], "callable", "Steel executor callable")?;
    let allowed_hostcalls =
        super::roster::normalize_allowed_hostcalls(super::fields::required_record_string_sequence(
            &config[3],
            "allowed-hostcalls",
            "Steel executor allowed hostcalls",
        )?)?;
    Ok(super::model::SteelExecutorConfig {
        source,
        callable,
        allowed_hostcalls,
    })
}

fn parse_wasm_executor_config(value: &preserves::IOValue) -> crate::error::Result<super::model::WasmExecutorConfig> {
    let config = super::fields::simple_record(value, "wasm-executor-v1", 4)?;
    let schema = super::fields::required_string(&config[0], "Wasm executor schema")?;
    if schema != crate::preserves_rail::RUNTIME_WASM_EXECUTOR_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported Wasm executor schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_WASM_EXECUTOR_SCHEMA
        )));
    }
    let module_hex = super::introspect::normalize_hex(
        &super::fields::required_record_string(&config[1], "module-hex", "Wasm executor module hex")?,
        "Wasm executor module hex",
    )?;
    let wit = super::fields::required_record_string(&config[2], "wit", "Wasm executor WIT interface")?;
    let allowed_hostcalls =
        super::roster::normalize_allowed_hostcalls(super::fields::required_record_string_sequence(
            &config[3],
            "allowed-hostcalls",
            "Wasm executor allowed hostcalls",
        )?)?;
    Ok(super::model::WasmExecutorConfig {
        module_hex,
        wit,
        allowed_hostcalls,
    })
}

fn parse_adapter_executor_config(
    value: &preserves::IOValue,
) -> crate::error::Result<super::model::AdapterExecutorConfig> {
    let config = super::fields::simple_record(value, "adapter-executor-v1", 5)?;
    let schema = super::fields::required_string(&config[0], "adapter executor schema")?;
    if schema != crate::preserves_rail::RUNTIME_ADAPTER_EXECUTOR_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported adapter executor schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_ADAPTER_EXECUTOR_SCHEMA
        )));
    }
    let manifest = super::fields::required_record_string(&config[1], "manifest", "adapter manifest")?;
    let abi = super::fields::required_record_string(&config[2], "abi", "adapter ABI")?;
    let allowed_hostcalls = super::roster::normalize_allowed_hostcalls(
        super::fields::required_record_string_sequence(&config[3], "allowed-hostcalls", "adapter allowed hostcalls")?,
    )?;
    let transcript = super::fields::required_record_string(&config[4], "transcript", "adapter transcript")?;
    Ok(super::model::AdapterExecutorConfig {
        manifest,
        abi,
        allowed_hostcalls,
        transcript,
    })
}

fn parse_remote_proxy_executor_config(
    value: &preserves::IOValue,
) -> crate::error::Result<super::model::RemoteProxyExecutorConfig> {
    let config = super::fields::simple_record(value, "remote-proxy-executor-v1", 6)?;
    let schema = super::fields::required_string(&config[0], "remote-proxy executor schema")?;
    if schema != crate::preserves_rail::RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported remote-proxy executor schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA
        )));
    }
    let peer = super::fields::required_record_string(&config[1], "peer", "remote-proxy peer")?;
    let endpoint = super::fields::required_record_string(&config[2], "endpoint", "remote-proxy endpoint")?;
    let contract = super::fields::required_record_string(&config[3], "contract", "remote-proxy contract")?;
    let allowed_hostcalls =
        super::roster::normalize_allowed_hostcalls(super::fields::required_record_string_sequence(
            &config[4],
            "allowed-hostcalls",
            "remote-proxy allowed hostcalls",
        )?)?;
    let transcript = super::fields::required_record_string(&config[5], "transcript", "remote-proxy transcript")?;
    Ok(super::model::RemoteProxyExecutorConfig {
        peer,
        endpoint,
        contract,
        allowed_hostcalls,
        transcript,
    })
}
