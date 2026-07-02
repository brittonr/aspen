
pub fn parse_policy(value: &IoValue) -> Result<crate::runtime::AdmissionPolicy> {
    let policy = simple_record(value, "policy-v1", 2)?;
    let schema = required_string(&policy[0], "policy schema")?;
    if schema != crate::preserves_rail::HARNESS_POLICY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported policy schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_POLICY_SCHEMA
        )));
    }
    let rule_values = required_sequence(&policy[1], "policy deny rules")?;
    let mut rules = Vec::with_capacity(rule_values.len());
    for rule in rule_values.iter() {
        let rule_value = value_to_iovalue(&rule);
        if rule_value.collect_simple_record("steel-predicate", None).is_some()
            || rule_value.collect_simple_record("dynamic-predicate", None).is_some()
        {
            return Err(MoltenError::invalid_harness(
                "Steel predicates require reviewed callable receipts and are disabled in local harness policy fixtures",
            ));
        }
        let rule = simple_record(&rule_value, "deny", 5)?;
        let actor = optional_string(&rule[0], "policy deny actor")?;
        let action = optional_action(&rule[1], "policy deny action")?;
        let target = optional_string(&rule[2], "policy deny target")?;
        let value = optional_runtime_match_value(&rule[3])?;
        let reason = required_string(&rule[4], "policy deny reason")?;
        if reason.is_empty() {
            return Err(MoltenError::invalid_harness("policy deny reason must not be empty"));
        }
        rules.push(crate::runtime::AdmissionDenyRule {
            actor,
            action,
            target,
            value,
            reason,
        });
    }
    Ok(crate::runtime::AdmissionPolicy::from_deny_rules(rules))
}

pub fn parse_actor_registry(value: &IoValue) -> Result<Vec<ActorDecl>> {
    let registry = simple_record(value, "actor-registry-v1", 2)?;
    let schema = required_string(&registry[0], "actor registry schema")?;
    if schema != crate::preserves_rail::HARNESS_ACTOR_REGISTRY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported actor registry schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_ACTOR_REGISTRY_SCHEMA
        )));
    }
    let actor_values = required_sequence(&registry[1], "actor registry entries")?;
    let mut seen = OrderedSet::new();
    let mut actors = Vec::with_capacity(actor_values.len());
    for actor in actor_values.iter() {
        let actor_value = value_to_iovalue(&actor);
        let actor = actor_value
            .collect_simple_record("actor", None)
            .ok_or_else(|| MoltenError::invalid_harness("expected <actor ...> in actor registry"))?;
        let arity = actor.fields_iter().count();
        if arity != 2 && arity != 3 {
            return Err(MoltenError::invalid_harness(format!(
                "actor registry entry arity must be 2 or 3, got {arity}"
            )));
        }
        let id = required_string(&actor[0], "actor id")?;
        if !seen.insert(id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate actor id {id}")));
        }
        let kind = parse_actor_kind(&required_string(&actor[1], "actor kind")?)?;
        let executor = if arity == 3 {
            Some(parse_actor_executor_config(&value_to_iovalue(&actor[2]), &kind, &id)?)
        } else {
            None
        };
        actors.push(ActorDecl { id, kind, executor });
    }
    Ok(actors)
}

fn parse_actor_executor_config(value: &IoValue, kind: &ActorKind, actor_id: &str) -> Result<ActorExecutorConfig> {
    if value.collect_simple_record("steel-executor-v1", None).is_some() {
        if kind != &ActorKind::Steel {
            return Err(MoltenError::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use Steel executor config",
                kind.as_str()
            )));
        }
        return parse_steel_executor_config(value).map(ActorExecutorConfig::Steel);
    }
    if value.collect_simple_record("wasm-executor-v1", None).is_some() {
        if kind != &ActorKind::Wasm {
            return Err(MoltenError::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use Wasm executor config",
                kind.as_str()
            )));
        }
        return parse_wasm_executor_config(value).map(ActorExecutorConfig::Wasm);
    }
    if value.collect_simple_record("adapter-executor-v1", None).is_some() {
        if kind != &ActorKind::Adapter {
            return Err(MoltenError::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use adapter executor config",
                kind.as_str()
            )));
        }
        return parse_adapter_executor_config(value).map(ActorExecutorConfig::Adapter);
    }
    if value.collect_simple_record("remote-proxy-executor-v1", None).is_some() {
        if kind != &ActorKind::RemoteProxy {
            return Err(MoltenError::invalid_harness(format!(
                "actor {actor_id} kind {} cannot use remote-proxy executor config",
                kind.as_str()
            )));
        }
        return parse_remote_proxy_executor_config(value).map(ActorExecutorConfig::RemoteProxy);
    }
    Err(MoltenError::invalid_harness(format!(
        "unsupported executor config for actor {actor_id}; expected <steel-executor-v1 ...>, <wasm-executor-v1 ...>, <adapter-executor-v1 ...>, or <remote-proxy-executor-v1 ...>"
    )))
}

fn parse_steel_executor_config(value: &IoValue) -> Result<SteelExecutorConfig> {
    let config = simple_record(value, "steel-executor-v1", 4)?;
    let schema = required_string(&config[0], "Steel executor schema")?;
    if schema != crate::preserves_rail::RUNTIME_STEEL_EXECUTOR_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Steel executor schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_STEEL_EXECUTOR_SCHEMA
        )));
    }
    let source = required_record_string(&config[1], "source", "Steel executor source")?;
    let callable = required_record_string(&config[2], "callable", "Steel executor callable")?;
    let allowed_hostcalls = normalize_allowed_hostcalls(required_record_string_sequence(
        &config[3],
        "allowed-hostcalls",
        "Steel executor allowed hostcalls",
    )?)?;
    Ok(SteelExecutorConfig {
        source,
        callable,
        allowed_hostcalls,
    })
}

fn parse_wasm_executor_config(value: &IoValue) -> Result<WasmExecutorConfig> {
    let config = simple_record(value, "wasm-executor-v1", 4)?;
    let schema = required_string(&config[0], "Wasm executor schema")?;
    if schema != crate::preserves_rail::RUNTIME_WASM_EXECUTOR_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported Wasm executor schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_WASM_EXECUTOR_SCHEMA
        )));
    }
    let module_hex = normalize_hex(
        &required_record_string(&config[1], "module-hex", "Wasm executor module hex")?,
        "Wasm executor module hex",
    )?;
    let wit = required_record_string(&config[2], "wit", "Wasm executor WIT interface")?;
    let allowed_hostcalls = normalize_allowed_hostcalls(required_record_string_sequence(
        &config[3],
        "allowed-hostcalls",
        "Wasm executor allowed hostcalls",
    )?)?;
    Ok(WasmExecutorConfig {
        module_hex,
        wit,
        allowed_hostcalls,
    })
}

fn parse_adapter_executor_config(value: &IoValue) -> Result<AdapterExecutorConfig> {
    let config = simple_record(value, "adapter-executor-v1", 5)?;
    let schema = required_string(&config[0], "adapter executor schema")?;
    if schema != crate::preserves_rail::RUNTIME_ADAPTER_EXECUTOR_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported adapter executor schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_ADAPTER_EXECUTOR_SCHEMA
        )));
    }
    let manifest = required_record_string(&config[1], "manifest", "adapter manifest")?;
    let abi = required_record_string(&config[2], "abi", "adapter ABI")?;
    let allowed_hostcalls = normalize_allowed_hostcalls(required_record_string_sequence(
        &config[3],
        "allowed-hostcalls",
        "adapter allowed hostcalls",
    )?)?;
    let transcript = required_record_string(&config[4], "transcript", "adapter transcript")?;
    Ok(AdapterExecutorConfig {
        manifest,
        abi,
        allowed_hostcalls,
        transcript,
    })
}

fn parse_remote_proxy_executor_config(value: &IoValue) -> Result<RemoteProxyExecutorConfig> {
    let config = simple_record(value, "remote-proxy-executor-v1", 6)?;
    let schema = required_string(&config[0], "remote-proxy executor schema")?;
    if schema != crate::preserves_rail::RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported remote-proxy executor schema {schema}; expected {}",
            crate::preserves_rail::RUNTIME_REMOTE_PROXY_EXECUTOR_SCHEMA
        )));
    }
    let peer = required_record_string(&config[1], "peer", "remote-proxy peer")?;
    let endpoint = required_record_string(&config[2], "endpoint", "remote-proxy endpoint")?;
    let contract = required_record_string(&config[3], "contract", "remote-proxy contract")?;
    let allowed_hostcalls = normalize_allowed_hostcalls(required_record_string_sequence(
        &config[4],
        "allowed-hostcalls",
        "remote-proxy allowed hostcalls",
    )?)?;
    let transcript = required_record_string(&config[5], "transcript", "remote-proxy transcript")?;
    Ok(RemoteProxyExecutorConfig {
        peer,
        endpoint,
        contract,
        allowed_hostcalls,
        transcript,
    })
}

fn normalize_allowed_hostcalls(values: Vec<String>) -> Result<Vec<String>> {
    let mut seen = OrderedSet::new();
    for value in values {
        parse_admission_action(&value)?;
        if !seen.insert(value.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate allowed hostcall {value}")));
        }
    }
    Ok(seen.into_iter().collect())
}

pub fn actor_ids_for_step(step: &super::core::CoreStep) -> Vec<&str> {
    step.actor_ids()
}

fn actor_ids_for_event(event: &IoValue) -> Result<Vec<String>> {
    if let Some(actors) = message_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = assertion_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = effect_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = decision_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = boundary_participants(event)? {
        return Ok(actors);
    }
    if let Some(actors) = receipt_participants(event)? {
        return Ok(actors);
    }
    Ok(Vec::new())
}

fn message_participants(event: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(message) = event.collect_simple_record("message-delivered", Some(3)) {
        return Ok(Some(vec![
            required_string(&message[0], "message sender")?,
            required_string(&message[1], "message recipient")?,
        ]));
    }
    if let Some(observe) = event.collect_simple_record("observe-registered", Some(2)) {
        return Ok(Some(vec![required_string(&observe[0], "observer actor")?]));
    }
    Ok(None)
}

fn assertion_participants(event: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(observed) = event.collect_simple_record("assertion-observed", Some(3)) {
        return Ok(Some(vec![
            required_string(&observed[0], "assertion observer")?,
            required_string(&observed[1], "assertion owner")?,
        ]));
    }
    if let Some(assertion) = event.collect_simple_record("assertion-committed", Some(2)) {
        return Ok(Some(vec![required_string(&assertion[0], "assertion actor")?]));
    }
    if let Some(retraction) = event.collect_simple_record("assertion-retracted", Some(2)) {
        return Ok(Some(vec![required_string(&retraction[0], "retraction actor")?]));
    }
    if let Some(observed) = event.collect_simple_record("assertion-retraction-observed", Some(3)) {
        return Ok(Some(vec![
            required_string(&observed[0], "assertion retraction observer")?,
            required_string(&observed[1], "assertion retraction owner")?,
        ]));
    }
    Ok(None)
}
