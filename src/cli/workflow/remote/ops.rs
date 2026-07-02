type Command = super::RemoteCommand;
type EnvelopeCommand = super::RemoteEnvelopeCommand;
type FilePath = std::path::PathBuf;
type FilePathRef = std::path::Path;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Envelope { command } => run_remote_envelope_command(command),
        Command::PublishLocal {
            transport_root,
            envelope,
            node,
            receipt_out,
        } => publish_local(transport_root, envelope, node, receipt_out),
        command @ Command::DeliverLocal { .. } => deliver_local(command),
        Command::RunTwoPeer { transport_root, out } => run_two_peer(transport_root, out),
        Command::Gate {
            delivery_log,
            admission_receipts,
            turn_context_refs,
            receipt_out,
        } => gate(delivery_log, admission_receipts, turn_context_refs, receipt_out),
    }
}

fn publish_local(
    transport_root: FilePath,
    envelope: FilePath,
    node: String,
    receipt_out: Option<FilePath>,
) -> Outcome<()> {
    let envelope_value = super::io::read_preserves_file(&envelope)?;
    let envelope = molten::remote_dataspace::parse_envelope(&envelope_value)?;
    let published = molten::remote_dataspace::publish_local_gossip(&transport_root, &envelope, &node)?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "remote dataspace publish receipt", &published.receipt_value)?;
    println!("remote publish-local ok envelope={} root={}", published.envelope_ref, transport_root.display());
    Ok(())
}

fn deliver_local(command: Command) -> Outcome<()> {
    let Command::DeliverLocal {
        transport_root,
        topic,
        envelope_ref,
        receiver_peer,
        out,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("deliver-local"));
    };
    let delivered =
        molten::remote_dataspace::deliver_local_gossip(&transport_root, &topic, &envelope_ref, &receiver_peer)?;
    if let Some(out) = out {
        super::io::write_file(&out, &molten::preserves_rail::to_text(&delivered.envelope.value)?)?;
        println!("remote delivered envelope {} written to {}", delivered.envelope.envelope_ref, out.display());
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "remote dataspace deliver receipt", &delivered.receipt_value)?;
    println!(
        "remote deliver-local ok envelope={} topic={} receiver={}",
        delivered.envelope.envelope_ref, topic, receiver_peer
    );
    Ok(())
}

fn run_two_peer(transport_root: FilePath, out: FilePath) -> Outcome<()> {
    let harness =
        molten::remote_dataspace::two_peer_service_ready_harness(&transport_root, remote_evidence_fixture()?)?;
    std::fs::create_dir_all(&out).map_err(molten::error::MoltenError::from)?;
    write_two_peer_outputs(&out, &harness)?;
    println!(
        "remote run-two-peer ok delivery_log={} gate_receipt={} out={}",
        harness.delivery_log.log_ref,
        molten::preserves_rail::canonical_hash(&harness.receipt_value)?,
        out.display()
    );
    Ok(())
}

fn write_two_peer_outputs(out: &FilePathRef, harness: &molten::remote_dataspace::TwoPeerHarness) -> Outcome<()> {
    super::io::write_file(
        &out.join("delivery-log.preserves"),
        &molten::preserves_rail::to_text(&harness.delivery_log.value)?,
    )?;
    super::io::write_file(
        &out.join("admission-receipt.preserves"),
        &molten::preserves_rail::to_text(&harness.admission_receipt_value)?,
    )?;
    super::io::write_file(
        &out.join("gate-receipt.preserves"),
        &molten::preserves_rail::to_text(&harness.receipt_value)?,
    )?;
    let turn_context_ref = remote_gate_turn_context_ref(&harness.receipt_value)?;
    let summary = two_peer_summary(harness, &turn_context_ref)?;
    super::io::write_file(
        &out.join("turn-context-ref.preserves"),
        &molten::preserves_rail::to_text(&molten::preserves_rail::string(&turn_context_ref))?,
    )?;
    super::io::write_file(&out.join("summary.preserves"), &molten::preserves_rail::to_text(&summary)?)
}

fn two_peer_summary(
    harness: &molten::remote_dataspace::TwoPeerHarness,
    turn_context_ref: &str,
) -> Outcome<preserves::IOValue> {
    Ok(molten::preserves_rail::record("remote-dataspace-summary-v1", vec![
        molten::preserves_rail::record("delivery-log", vec![molten::preserves_rail::string(
            &harness.delivery_log.log_ref,
        )]),
        molten::preserves_rail::record("admission-receipt", vec![molten::preserves_rail::string(
            &molten::preserves_rail::canonical_hash(&harness.admission_receipt_value)?,
        )]),
        molten::preserves_rail::record("gate-receipt", vec![molten::preserves_rail::string(
            &molten::preserves_rail::canonical_hash(&harness.receipt_value)?,
        )]),
        molten::preserves_rail::record("turn-context-ref", vec![molten::preserves_rail::string(turn_context_ref)]),
    ]))
}

fn gate(
    delivery_log: FilePath,
    admission_receipts: Vec<FilePath>,
    turn_context_refs: Vec<String>,
    receipt_out: Option<FilePath>,
) -> Outcome<()> {
    let log_value = super::io::read_preserves_file(&delivery_log)?;
    let log = molten::remote_dataspace::parse_delivery_log(&log_value)?;
    let receipts = admission_receipts
        .iter()
        .map(|path| super::io::read_preserves_file(path))
        .collect::<Outcome<Vec<_>>>()?;
    let receipt = molten::remote_dataspace::gate_receipt_value(&log, &receipts, &turn_context_refs)?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "remote dataspace gate receipt", &receipt)
}

fn run_remote_envelope_command(command: EnvelopeCommand) -> Outcome<()> {
    match command {
        EnvelopeCommand::Build {
            from_peer,
            from_actor,
            to_peer,
            topic,
            operation,
            payload,
            content_refs,
            capability_refs,
            evidence_refs,
            out,
        } => {
            let payload = super::io::read_preserves_file(&payload)?;
            let operation = parse_remote_operation(&operation)?;
            let envelope = molten::remote_dataspace::build_envelope(molten::remote_dataspace::EnvelopeInput {
                from_peer,
                from_actor,
                to_peer,
                topic,
                operation,
                payload,
                content_refs,
                capability_refs,
                evidence_refs,
            })?;
            super::io::write_file(&out, &molten::preserves_rail::to_text(&envelope.value)?)?;
            println!("remote envelope {} written to {}", envelope.envelope_ref, out.display());
            Ok(())
        }
    }
}

fn parse_remote_operation(operation: &str) -> Outcome<molten::remote_dataspace::Operation> {
    match operation {
        "message" => Ok(molten::remote_dataspace::Operation::Message),
        "assert" => Ok(molten::remote_dataspace::Operation::Assert),
        "retract" => Ok(molten::remote_dataspace::Operation::Retract),
        "observe" => Ok(molten::remote_dataspace::Operation::Observe),
        _ => Err(molten::error::MoltenError::invalid_harness(format!(
            "unsupported remote dataspace operation {operation}; expected message/assert/retract/observe"
        ))),
    }
}

fn remote_evidence_fixture() -> Outcome<molten::remote_dataspace::DeliveryEvidence> {
    Ok(molten::remote_dataspace::DeliveryEvidence {
        peer_bootstrap_refs: vec![remote_cli_synthetic_ref("remote-bootstrap")?],
        capability_refs: vec![remote_cli_synthetic_ref("remote-capability")?],
        policy_refs: vec![remote_cli_synthetic_ref("remote-policy")?],
        resource_refs: vec![remote_cli_synthetic_ref("remote-resource")?],
        authority_refs: vec![remote_cli_synthetic_ref("remote-authority")?],
    })
}

fn remote_cli_synthetic_ref(label: &str) -> Outcome<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("remote-cli-ref", vec![
        molten::preserves_rail::string(label),
    ]))
}

fn remote_gate_turn_context_ref(gate_receipt: &preserves::IOValue) -> Outcome<String> {
    let fields = gate_receipt
        .collect_simple_record("remote-dataspace-gate-receipt-v1", Some(7))
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("expected remote dataspace gate receipt"))?;
    let context = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let refs = context
        .collect_simple_record("turn-journal-context-refs", Some(1))
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("expected remote turn context refs"))?;
    let sequence = refs[0]
        .collect_sequence()
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("expected turn context ref sequence"))?;
    let first = sequence
        .iter()
        .next()
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("missing turn context ref"))?;
    first
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("expected string turn context ref"))
}

pub(super) fn remote_dataspace_gate_summary(value: &preserves::IOValue) -> Outcome<String> {
    if molten::ledger::artifact_kind(value) != "remote-dataspace-gate-receipt" {
        return Err(molten::error::MoltenError::invalid_harness("not a remote dataspace gate receipt"));
    }
    Ok(format!("remote dataspace gate receipt ref={}", molten::preserves_rail::canonical_hash(value)?))
}

fn wrong_handler(name: &str) -> molten::error::MoltenError {
    molten::error::MoltenError::invalid_harness(format!("remote {name} handler called with another command"))
}
