
fn validate_verified_segment(
    index: &ChainIndex,
    anchor_ref: Option<&str>,
    verified_links: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) {
    if verified_links.is_empty() {
        return;
    }
    if let Some(anchor_ref) = anchor_ref {
        if verified_links.first().map(String::as_str) != Some(anchor_ref) {
            diagnostics.push_item(ChainDiagnostic::new(
                "anchor-descent",
                "verified segment does not begin at requested anchor",
                vec![anchor_ref.to_string()],
            ));
        }
    } else if let Some(first_ref) = verified_links.first() {
        let Some(first) = index.links_by_ref.get(first_ref) else {
            return;
        };
        if let Err(error) = validate_genesis(first) {
            diagnostics.push_item(ChainDiagnostic::new(
                "genesis-invalid",
                format!("segment does not begin with a valid genesis link: {error}"),
                vec![first_ref.clone()],
            ));
        }
    }

    for window in verified_links.windows(2) {
        let previous_ref = &window[0];
        let next_ref = &window[1];
        let (Some(previous), Some(next)) = (index.links_by_ref.get(previous_ref), index.links_by_ref.get(next_ref))
        else {
            diagnostics.push_item(ChainDiagnostic::new(
                "gap",
                "verified segment references an unavailable adjacent link",
                vec![previous_ref.clone(), next_ref.clone()],
            ));
            continue;
        };
        if let Err(error) = validate_append(previous, next) {
            diagnostics.push_item(ChainDiagnostic::new(
                "gap",
                format!("adjacent links are not a valid append: {error}"),
                vec![previous_ref.clone(), next_ref.clone()],
            ));
        }
    }
}

fn diagnostic_is_fatal(diagnostic: &ChainDiagnostic, fork_policy: ChainForkPolicy) -> bool {
    match diagnostic.kind.as_str() {
        "fork" | "sequence-conflict" => fork_policy.fork_diagnostics_are_fatal(),
        _ => true,
    }
}

fn diagnostic_check(label: &str, diagnostics: &[ChainDiagnostic], failing_kinds: &[&str]) -> IoValue {
    let decision = if diagnostics.iter().any(|diagnostic| failing_kinds.contains(&diagnostic.kind.as_str())) {
        "fail"
    } else {
        "pass"
    };
    record("check", vec![string(label), string(decision)])
}

fn fork_policy_check(diagnostics: &[ChainDiagnostic], fork_policy: ChainForkPolicy) -> IoValue {
    let has_fork = diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.kind.as_str(), "fork" | "sequence-conflict"));
    let decision = match (has_fork, fork_policy) {
        (false, _) => "pass",
        (true, ChainForkPolicy::RejectUnexpectedForks) => "fail",
        (true, ChainForkPolicy::RetainForkEvidence) => "retained",
    };
    record("check", vec![string("no-fork-policy"), string(decision)])
}

fn diagnostic_value(diagnostic: &ChainDiagnostic) -> IoValue {
    record("diagnostic", vec![
        string(&diagnostic.kind),
        string(&diagnostic.detail),
        ref_sequence_value(&diagnostic.refs),
    ])
}

fn ref_sequence_value(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(|reference| string(reference)).collect())
}

fn check_value(check: &ChainCheck) -> IoValue {
    record("check", vec![string(&check.name), string(&check.decision)])
}

fn producer_record(producer: &ChainProducer) -> IoValue {
    record("producer", vec![
        record("id", vec![string(&producer.id)]),
        record("key", vec![string(&producer.key_ref)]),
    ])
}

fn parse_checkpoint_range(value: &Value<IoValue>) -> Result<(String, String, String, String)> {
    let value = value_to_iovalue(value);
    let range = value
        .collect_simple_record("range", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("chain checkpoint missing range record"))?;
    Ok((
        record_string(&range[0], "anchor", "checkpoint range anchor")?,
        record_string(&range[1], "head", "checkpoint range head")?,
        record_string(&range[2], "verify-receipt", "checkpoint range verify receipt")?,
        record_string(&range[3], "predicate", "checkpoint range predicate receipt")?,
    ))
}

fn parse_control_plane(value: &Value<IoValue>) -> Result<()> {
    let value = value_to_iovalue(value);
    let control = value
        .collect_simple_record("control-plane", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("chain checkpoint missing control-plane record"))?;
    let mode = record_string(&control[0], "mode", "checkpoint control-plane mode")?;
    if mode != "trellis-raft" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported checkpoint control-plane mode {mode}; expected trellis-raft"
        )));
    }
    let command = record_string(&control[1], "command", "checkpoint control-plane command")?;
    if command != "accept-chain-head" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported checkpoint control-plane command {command}; expected accept-chain-head"
        )));
    }
    Ok(())
}

fn chain_record(chain: &ChainScope) -> IoValue {
    record("chain", vec![
        record("scope", vec![string(&chain.scope)]),
        record("id", vec![string(&chain.id)]),
        record("epoch", vec![string(&chain.epoch)]),
    ])
}

fn ensure_sequence_unoccupied(index: &ChainIndex, link: &ChainLink) -> Result<()> {
    let occupants = index.links_for_sequence(&link.chain, link.sequence);
    if occupants.is_empty() {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "chain sequence {} for {:?} is already occupied by {:?}",
            link.sequence, link.chain, occupants
        )))
    }
}

fn sorted_refs(refs: Option<&OrderedSet<String>>) -> Vec<String> {
    refs.map_or_else(Vec::new, |refs| refs.iter().cloned().collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_chain(value: &Value<IoValue>) -> Result<ChainScope> {
    let value = value_to_iovalue(value);
    let chain = value
        .collect_simple_record("chain", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing chain record"))?;
    Ok(ChainScope {
        scope: record_string(&chain[0], "scope", "chain scope")?,
        id: record_string(&chain[1], "id", "chain id")?,
        epoch: record_string(&chain[2], "epoch", "chain epoch")?,
    })
}

fn parse_previous_link_ref(value: &Value<IoValue>) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let prev = value
        .collect_simple_record("prev", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing prev record"))?;
    let prev_value = value_to_iovalue(&prev[0]);
    if prev_value.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = prev_value.collect_simple_record("some", Some(1)) {
        required_string(&some[0], "previous link ref").map(Some)
    } else {
        Err(MoltenError::invalid_harness("chain link prev must be <none> or <some ref>"))
    }
}

fn parse_payload(value: &Value<IoValue>) -> Result<ChainPayload> {
    let value = value_to_iovalue(value);
    let payload = value
        .collect_simple_record("payload", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing payload record"))?;
    Ok(ChainPayload {
        kind: record_string(&payload[0], "kind", "payload kind")?,
        artifact_ref: record_string(&payload[1], "ref", "payload ref")?,
        schema: record_string(&payload[2], "schema", "payload schema")?,
    })
}

fn parse_context_refs(value: &Value<IoValue>) -> Result<Vec<ChainContextRef>> {
    let value = value_to_iovalue(value);
    let context = value
        .collect_simple_record("context", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing context record"))?;
    let refs = context[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("chain link context must be a sequence"))?;
    refs.iter()
        .map(|value| {
            let value = value_to_iovalue(value);
            let context_ref = value
                .collect_simple_record("ref", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("chain context item must be <ref label artifact-ref>"))?;
            Ok(ChainContextRef {
                label: required_string(&context_ref[0], "context ref label")?,
                artifact_ref: required_string(&context_ref[1], "context artifact ref")?,
            })
        })
        .collect()
}

fn parse_producer(value: &Value<IoValue>) -> Result<ChainProducer> {
    let value = value_to_iovalue(value);
    let producer = value
        .collect_simple_record("producer", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing producer record"))?;
    Ok(ChainProducer {
        id: record_string(&producer[0], "id", "producer id")?,
        key_ref: record_string(&producer[1], "key", "producer key ref")?,
    })
}

fn parse_trellis(value: &Value<IoValue>) -> Result<ChainTrellisEvidence> {
    let value = value_to_iovalue(value);
    let trellis = value
        .collect_simple_record("trellis", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing trellis record"))?;
    Ok(ChainTrellisEvidence {
        predicate: record_string(&trellis[0], "predicate", "trellis predicate")?,
        input_ref: record_string(&trellis[1], "input", "trellis predicate input ref")?,
        decision: record_string(&trellis[2], "decision", "trellis decision")?,
    })
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<ChainCheck>> {
    let value = value_to_iovalue(value);
    let checks = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing checks record"))?;
    let sequence = checks[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("chain link checks must be a sequence"))?;
    sequence
        .iter()
        .map(|value| {
            let value = value_to_iovalue(value);
            let check = value
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("chain check item must be <check name decision>"))?;
            Ok(ChainCheck {
                name: required_string(&check[0], "check name")?,
                decision: required_string(&check[1], "check decision")?,
            })
        })
        .collect()
}

fn validate_chain_link_shape(link: &ChainLink) -> Result<()> {
    validate_chain_scope(&link.chain)?;
    require_ref(&link.link_ref, "chain link ref")?;
    if let Some(previous_link_ref) = &link.previous_link_ref {
        require_ref(previous_link_ref, "previous link ref")?;
    }
    require_non_empty(&link.payload.kind, "payload kind")?;
    require_ref(&link.payload.artifact_ref, "payload ref")?;
    require_non_empty(&link.payload.schema, "payload schema")?;
    for context_ref in &link.context_refs {
        require_non_empty(&context_ref.label, "context ref label")?;
        require_ref(&context_ref.artifact_ref, "context artifact ref")?;
    }
    require_non_empty(&link.producer.id, "producer id")?;
    require_ref(&link.producer.key_ref, "producer key ref")?;
    require_non_empty(&link.trellis.predicate, "trellis predicate")?;
    require_ref(&link.trellis.input_ref, "trellis input ref")?;
    require_non_empty(&link.trellis.decision, "trellis decision")?;
    for check in &link.checks {
        require_non_empty(&check.name, "chain check name")?;
        require_non_empty(&check.decision, "chain check decision")?;
    }
    Ok(())
}
