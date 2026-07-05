fn validate_commands(topology: &Topology, commands: &[SimulationCommand]) -> Result<()> {
    if commands.is_empty() {
        return Err(MoltenError::invalid_harness("distributed simulation requires commands"));
    }
    ensure_count_at_most(commands.len(), MAX_DISTRIBUTED_COMMANDS, "distributed commands")?;
    let peers = topology.peers.iter().map(|peer| peer.id.as_str()).collect::<OrderedSet<_>>();
    for command in commands {
        validate_text("simulation operation", &command.operation_id)?;
        if !peers.contains(command.from_peer.as_str()) || !peers.contains(command.to_peer.as_str()) {
            return Err(MoltenError::invalid_harness(format!(
                "simulation command {} references peer outside topology",
                command.operation_id
            )));
        }
        validate_ref(&command.payload_ref, "simulation payload")?;
        validate_ref(&command.commit_ref, "simulation commit")?;
        validate_optional_ref(command.authority_ref.as_deref(), "simulation authority")?;
        validate_optional_ref(command.policy_ref.as_deref(), "simulation policy")?;
        validate_optional_ref(command.resource_ref.as_deref(), "simulation resource")?;
        validate_optional_ref(command.transport_ref.as_deref(), "simulation transport")?;
    }
    Ok(())
}

fn validate_fault_kind(kind: &str) -> Result<()> {
    match kind {
        FAULT_DELAY
        | FAULT_DROP
        | FAULT_DUPLICATE
        | FAULT_REORDER
        | FAULT_PARTITION
        | FAULT_REJOIN
        | FAULT_CRASH
        | FAULT_RESTART
        | FAULT_RESOURCE_PRESSURE
        | FAULT_STALE_EVIDENCE
        | FAULT_AMBIENT_STATE_DRIFT
        | FAULT_CORRUPTED_RECEIPT
        | FAULT_UNAUTHORIZED_TRANSPORT => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported distributed fault kind {other}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        PASS_DECISION | DENY_DECISION => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported distributed decision {other}"))),
    }
}

fn validate_event_decision(decision: &str) -> Result<()> {
    match decision {
        PASS_DECISION | DENY_DECISION => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported simulation event decision {other}"))),
    }
}

fn command_tick(index: usize) -> Result<u64> {
    u64::try_from(index).map_err(|_| MoltenError::invalid_harness("simulation command index exceeds u64"))
}

fn peer_values(peers: &[Peer]) -> Result<Vec<IoValue>> {
    peers
        .iter()
        .map(|peer| {
            Ok(record("peer", vec![
                record("id", vec![string(&peer.id)]),
                record("roles", vec![sequence(peer.roles.iter().map(string).collect())]),
            ]))
        })
        .collect()
}

fn channel_values(channels: &[Channel]) -> Result<Vec<IoValue>> {
    channels
        .iter()
        .map(|channel| {
            Ok(record("channel", vec![
                record("id", vec![string(&channel.id)]),
                record("from", vec![string(&channel.from_peer)]),
                record("to", vec![string(&channel.to_peer)]),
                record("topic", vec![string(&channel.topic)]),
            ]))
        })
        .collect()
}

fn fault_event_values(events: &[FaultEvent]) -> Result<Vec<IoValue>> {
    events
        .iter()
        .map(|event| {
            Ok(record("fault-event", vec![
                record("kind", vec![string(&event.kind)]),
                record("target-kind", vec![string(&event.target_kind)]),
                record("target", vec![string(&event.target)]),
                record("operation", vec![optional_string_value(event.operation_id.as_deref())]),
                record("start-tick", vec![u64_value(event.start_tick)]),
                record("duration-ticks", vec![u64_value(event.duration_ticks)]),
                record("diagnostic", vec![string(&event.diagnostic)]),
            ]))
        })
        .collect()
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
}

fn validate_optional_ref(reference: Option<&str>, label: &str) -> Result<()> {
    if let Some(reference) = reference {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_DISTRIBUTED_REFS, label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} must not be empty")));
    }
    Ok(())
}

fn validate_strings(label: &str, values: &[String], maximum: usize) -> Result<()> {
    ensure_count_at_most(values.len(), maximum, label)?;
    for value in values {
        validate_text(label, value)?;
    }
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds bound {maximum}")))
    }
}

trait DiagnosticSink {
    fn push_bounded(&mut self, diagnostic: String) -> Result<()>;
}

impl DiagnosticSink for Vec<String> {
    fn push_bounded(&mut self, diagnostic: String) -> Result<()> {
        let next =
            self.len().checked_add(1).ok_or_else(|| MoltenError::invalid_harness("diagnostic count overflow"))?;
        ensure_count_at_most(next, MAX_DISTRIBUTED_COMMANDS, "diagnostics")?;
        self.push(diagnostic);
        Ok(())
    }
}

fn string_values(label: &str, values: &[String], maximum: usize) -> Result<Vec<IoValue>> {
    validate_strings(label, values, maximum)?;
    Ok(values.iter().map(string).collect())
}

fn record_string(value: &preserves::Value<IoValue>, record_name: &str, context: &str) -> Result<String> {
    let field = value_to_iovalue(value);
    let record = field
        .collect_simple_record(record_name, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {record_name} for {context}")))?;
    record[0]
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {context}")))
}

fn record_ref(value: &preserves::Value<IoValue>, record_name: &str, context: &str) -> Result<String> {
    let reference = record_string(value, record_name, context)?;
    validate_ref(&reference, context)?;
    Ok(reference)
}

fn record_string_sequence(value: &preserves::Value<IoValue>, record_name: &str, context: &str) -> Result<Vec<String>> {
    let field = value_to_iovalue(value);
    let record = field
        .collect_simple_record(record_name, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {record_name} for {context}")))?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {context}")))?;
    let mut output = Vec::with_capacity(sequence.len());
    for item in sequence.iter() {
        output.push(
            item.as_string()
                .map(|value| value.to_string())
                .ok_or_else(|| MoltenError::invalid_harness(format!("expected string sequence item for {context}")))?,
        );
    }
    Ok(output)
}

fn record_ref_sequence(value: &preserves::Value<IoValue>, record_name: &str, context: &str) -> Result<Vec<String>> {
    let refs = record_string_sequence(value, record_name, context)?;
    validate_ref_slice(context, &refs)?;
    Ok(refs)
}

fn require_schema(value: &preserves::Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = value
        .as_string()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected schema string for {context}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{context} schema mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn value_to_iovalue(value: &preserves::Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

fn canonical_ref(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

