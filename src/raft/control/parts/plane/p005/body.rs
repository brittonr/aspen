
fn state_maps(state: &ControlRegistryState) -> Result<RegistryMaps> {
    let entries = state
        .entries
        .iter()
        .map(|entry| {
            (
                ControlRegistryKey {
                    namespace: entry.namespace.clone(),
                    name: entry.name.clone(),
                },
                entry.target_ref.clone(),
            )
        })
        .collect::<OrderedMap<_, _>>();
    if entries.len() != state.entries.len() {
        return Err(MoltenError::invalid_harness("duplicate control registry entry key"));
    }
    let sessions = state
        .client_sessions
        .iter()
        .map(|session| {
            (
                ClientSequenceKey {
                    client_session: session.client_session.clone(),
                    sequence: session.sequence,
                },
                session.clone(),
            )
        })
        .collect::<OrderedMap<_, _>>();
    if sessions.len() != state.client_sessions.len() {
        return Err(MoltenError::invalid_harness("duplicate control registry client sequence"));
    }
    Ok(RegistryMaps { entries, sessions })
}

fn entries_from_map(entries: &OrderedMap<ControlRegistryKey, String>) -> Vec<ControlRegistryEntry> {
    entries
        .iter()
        .map(|(key, target_ref)| ControlRegistryEntry {
            namespace: key.namespace.clone(),
            name: key.name.clone(),
            target_ref: target_ref.clone(),
        })
        .collect()
}

fn sessions_from_map(sessions: &OrderedMap<ClientSequenceKey, ClientSessionRecord>) -> Vec<ClientSessionRecord> {
    sessions.values().cloned().collect()
}

fn parse_registry_entries(value: &Value<IoValue>) -> Result<Vec<ControlRegistryEntry>> {
    let values = field_sequence(value, "entries")?;
    ensure_count_at_most(values.len(), MAX_RAFT_ENTRIES, "control registry entries")?;
    let mut entries = Vec::with_capacity(values.len());
    for entry in values {
        let entry_value = value_to_iovalue(&entry);
        let fields = entry_value
            .collect_simple_record("entry", Some(3))
            .ok_or_else(|| MoltenError::invalid_harness("expected control registry entry"))?;
        let namespace = required_string(&fields[0], "entry namespace")?;
        validate_namespace(&namespace)?;
        let name = required_string(&fields[1], "entry name")?;
        validate_non_empty(&name, "entry name")?;
        let target_ref = required_ref(&fields[2], "entry target ref")?;
        entries.push(ControlRegistryEntry {
            namespace,
            name,
            target_ref,
        });
    }
    Ok(entries)
}

fn parse_client_sessions(value: &Value<IoValue>) -> Result<Vec<ClientSessionRecord>> {
    let values = field_sequence(value, "client-sessions")?;
    ensure_count_at_most(values.len(), MAX_RAFT_ENTRIES, "control registry client sessions")?;
    let mut sessions = Vec::with_capacity(values.len());
    for session in values {
        let session_value = value_to_iovalue(&session);
        let fields = session_value
            .collect_simple_record("session", Some(3))
            .ok_or_else(|| MoltenError::invalid_harness("expected client session record"))?;
        let client_session = required_string(&fields[0], "client session id")?;
        validate_client_session(&client_session)?;
        sessions.push(ClientSessionRecord {
            client_session,
            sequence: required_u64(&fields[1], "client session sequence")?,
            result_command_ref: required_ref(&fields[2], "client session command ref")?,
        });
    }
    Ok(sessions)
}

fn find_entry<'a>(state: &'a ControlRegistryState, namespace: &str, name: &str) -> Option<&'a ControlRegistryEntry> {
    state.entries.iter().find(|entry| entry.namespace == namespace && entry.name == name)
}

fn validate_control_command(input: &ControlRegistryCommandInput) -> Result<()> {
    validate_operation(&input.operation)?;
    validate_namespace(&input.namespace)?;
    validate_non_empty(&input.name, "control registry command name")?;
    if let Some(target_ref) = &input.target_ref {
        require_ref(target_ref, "control registry command target ref")?;
    }
    match input.operation.as_str() {
        "install-protocol" if input.namespace == "protocol" && input.target_ref.is_some() => Ok(()),
        "set-artifact-name" if input.namespace == "artifact-name" && input.target_ref.is_some() => Ok(()),
        "set-policy-version" if input.namespace == "policy" && input.target_ref.is_some() => Ok(()),
        "set-capability-version" if input.namespace == "capability" && input.target_ref.is_some() => Ok(()),
        "set-receipt-index" if input.namespace == "receipt-index" && input.target_ref.is_some() => Ok(()),
        "set-coordination-state" if input.namespace.starts_with("coordination:") && input.target_ref.is_some() => {
            Ok(())
        }
        "remove" if input.target_ref.is_none() => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!(
            "operation {} is not admitted for namespace {}",
            input.operation, input.namespace
        ))),
    }
}

fn validate_group_id(group_id: &str) -> Result<()> {
    if group_id.starts_with("raft:") {
        validate_non_empty(group_id, "raft group id")
    } else {
        Err(MoltenError::invalid_harness(format!("raft group id must start with raft:, got {group_id}")))
    }
}

fn validate_client_session(client_session: &str) -> Result<()> {
    validate_non_empty(client_session, "raft client session")
}

fn validate_operation(operation: &str) -> Result<()> {
    if [
        "install-protocol",
        "set-artifact-name",
        "set-policy-version",
        "set-capability-version",
        "set-receipt-index",
        "set-coordination-state",
        "remove",
    ]
    .contains(&operation)
    {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported control registry operation {operation}")))
    }
}

fn validate_namespace(namespace: &str) -> Result<()> {
    if ["protocol", "artifact-name", "policy", "capability", "receipt-index"].contains(&namespace)
        || namespace.starts_with("coordination:")
    {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported control registry namespace {namespace}")))
    }
}

fn validate_command_schema_list(command_schemas: &[String]) -> Result<()> {
    ensure_count_at_most(command_schemas.len(), MAX_RAFT_COMMANDS, "raft command schemas")?;
    for schema in command_schemas {
        if !allowed_command_schemas().contains(&schema.as_str()) {
            return Err(MoltenError::invalid_harness(format!("unsupported raft command schema {schema}")));
        }
    }
    Ok(())
}

fn allowed_command_schemas() -> &'static [&'static str] {
    &[
        "install-protocol",
        "set-artifact-name",
        "set-policy-version",
        "set-capability-version",
        "set-receipt-index",
        "set-coordination-state",
        "remove",
    ]
}

fn operation_schema(operation: &str) -> &str {
    match operation {
        "install-protocol" => "install-protocol",
        "set-artifact-name" => "set-artifact-name",
        "set-policy-version" => "set-policy-version",
        "set-capability-version" => "set-capability-version",
        "set-receipt-index" => "set-receipt-index",
        "set-coordination-state" => "set-coordination-state",
        "remove" => "remove",
        _ => "unknown",
    }
}

fn validate_read_mode(read_mode: &str) -> Result<()> {
    if read_mode == READ_MODE_READ_INDEX {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported raft read mode {read_mode}")))
    }
}

fn validate_read_consistency_mode(read_consistency_mode: &str) -> Result<()> {
    match read_consistency_mode {
        READ_CONSISTENCY_LINEARIZABLE | READ_CONSISTENCY_LOCAL_STALE => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!(
            "unsupported raft read consistency mode {read_consistency_mode}"
        ))),
    }
}

fn validate_read_consistency_support(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_RAFT_COMMANDS, "raft read consistency modes")?;
    if !values.iter().any(|value| value == READ_CONSISTENCY_LINEARIZABLE) {
        return Err(MoltenError::invalid_harness("consensus profile must support linearizable reads"));
    }
    for value in values {
        validate_read_consistency_mode(value)?;
    }
    Ok(())
}

fn validate_consensus_algorithm_profile(profile: &ConsensusAlgorithmProfileInput) -> Result<()> {
    validate_non_empty(&profile.algorithm_profile, "consensus algorithm profile")?;
    validate_non_empty(&profile.admitted_profile_version, "consensus profile version")?;
    validate_non_empty(&profile.quorum_rule, "consensus quorum rule")?;
    validate_read_consistency_support(&profile.read_consistency_support)?;
    validate_refs(&profile.membership_policy_refs, "consensus membership policy ref")?;
    validate_refs(&profile.required_evidence_refs, "consensus required evidence ref")?;
    validate_caveats(&profile.fault_model_caveats)?;
    let placement = profile
        .placement_ref
        .as_deref()
        .ok_or_else(|| MoltenError::invalid_harness("consensus manifest requires placement ref"))?;
    require_ref(placement, "consensus placement ref")?;
    match profile.algorithm_profile.as_str() {
        CONSENSUS_PROFILE_RAFT => validate_raft_consensus_profile(profile),
        CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL => validate_leaderless_consensus_profile(profile),
        value => Err(MoltenError::invalid_harness(format!("unsupported consensus algorithm profile {value}"))),
    }
}

fn validate_raft_consensus_profile(profile: &ConsensusAlgorithmProfileInput) -> Result<()> {
    if profile.admitted_profile_version != CONSENSUS_PROFILE_VERSION_RAFT {
        return Err(MoltenError::invalid_harness("raft consensus profile version mismatch"));
    }
    if profile.quorum_rule != QUORUM_RULE_MAJORITY_READ_INDEX {
        return Err(MoltenError::invalid_harness("raft consensus profile must use majority read-index quorum"));
    }
    Ok(())
}

fn validate_leaderless_consensus_profile(profile: &ConsensusAlgorithmProfileInput) -> Result<()> {
    if profile.admitted_profile_version != CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL {
        return Err(MoltenError::invalid_harness("leaderless experimental profile version mismatch"));
    }
    if profile.quorum_rule != QUORUM_RULE_LEADERLESS_MAJORITY {
        return Err(MoltenError::invalid_harness("leaderless experimental profile must use leaderless majority quorum"));
    }
    if profile.required_evidence_refs.is_empty() {
        return Err(MoltenError::invalid_harness(
            "leaderless experimental profile requires proof policy simulation placement and membership evidence",
        ));
    }
    Ok(())
}

fn validate_caveats(caveats: &[String]) -> Result<()> {
    ensure_count_at_most(caveats.len(), MAX_RAFT_REFS, "consensus caveats")?;
    if caveats.is_empty() {
        return Err(MoltenError::invalid_harness("consensus profile requires explicit fault-model caveats"));
    }
    for caveat in caveats {
        validate_non_empty(caveat, "consensus caveat")?;
    }
    Ok(())
}

fn default_consensus_caveats() -> Vec<String> {
    vec![
        "no-byzantine-tolerance".to_string(),
        "not-a-general-purpose-database".to_string(),
        "no-ordinary-actor-traffic".to_string(),
        "no-lease-reads-without-timing-policy".to_string(),
    ]
}

fn manifest_checks(profile: &ConsensusAlgorithmProfileInput) -> Vec<(&'static str, &'static str)> {
    let profile_check = match profile.algorithm_profile.as_str() {
        CONSENSUS_PROFILE_RAFT => "pass",
        CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL => "diagnostic",
        _ => "fail",
    };
    vec![
        ("control-plane-only", "pass"),
        ("explicit-command-schemas", "pass"),
        ("read-index-default", "pass"),
        ("algorithm-profile-declared", "pass"),
        ("linearizable-read-supported", "pass"),
        ("placement-ref-bound", "pass"),
        ("production-profile", profile_check),
    ]
}

fn consensus_production_status(profile: &str) -> &'static str {
    match profile {
        CONSENSUS_PROFILE_RAFT => PRODUCTION_STATUS_ADMITTED,
        CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL => PRODUCTION_STATUS_EXPERIMENTAL,
        _ => "unsupported",
    }
}

fn validate_non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_RAFT_REFS, label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

fn require_ref(reference: &str, label: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {label}, got {reference}: {error}"))
    })
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    reference.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn session_record_value(session: &ClientSessionRecord) -> IoValue {
    record("session", vec![
        string(&session.client_session),
        u64_value(session.sequence),
        string(&session.result_command_ref),
    ])
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    parse_optional_ref_value(&fields[0])
}

fn record_iovalue(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_u64(&fields[0], label)
}

fn record_bool(value: &Value<IoValue>, label: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}
