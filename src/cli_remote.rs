use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::remote_dataspace;

#[derive(Debug, Subcommand)]
pub enum RemoteCommand {
    Envelope {
        #[command(subcommand)]
        command: RemoteEnvelopeCommand,
    },
    PublishLocal {
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long)]
        envelope: PathBuf,
        #[arg(long)]
        node: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    DeliverLocal {
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        envelope_ref: String,
        #[arg(long)]
        receiver_peer: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RunTwoPeer {
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Gate {
        #[arg(long)]
        delivery_log: PathBuf,
        #[arg(long = "admission-receipt")]
        admission_receipts: Vec<PathBuf>,
        #[arg(long = "turn-context-ref")]
        turn_context_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum RemoteEnvelopeCommand {
    Build {
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        from_actor: String,
        #[arg(long)]
        to_peer: String,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        payload: PathBuf,
        #[arg(long = "content-ref")]
        content_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
}

pub fn run_remote_command(command: RemoteCommand) -> Result<()> {
    match command {
        RemoteCommand::Envelope { command } => run_remote_envelope_command(command),
        RemoteCommand::PublishLocal {
            transport_root,
            envelope,
            node,
            receipt_out,
        } => {
            let envelope_value = read_preserves_file(&envelope)?;
            let envelope = remote_dataspace::parse_envelope(&envelope_value)?;
            let published = remote_dataspace::publish_local_gossip(&transport_root, &envelope, &node)?;
            emit_named_receipt(receipt_out.as_ref(), "remote dataspace publish receipt", &published.receipt_value)?;
            println!("remote publish-local ok envelope={} root={}", published.envelope_ref, transport_root.display());
            Ok(())
        }
        RemoteCommand::DeliverLocal {
            transport_root,
            topic,
            envelope_ref,
            receiver_peer,
            out,
            receipt_out,
        } => {
            let delivered =
                remote_dataspace::deliver_local_gossip(&transport_root, &topic, &envelope_ref, &receiver_peer)?;
            if let Some(out) = out {
                write_file(&out, &to_text(&delivered.envelope.value)?)?;
                println!("remote delivered envelope {} written to {}", delivered.envelope.envelope_ref, out.display());
            }
            emit_named_receipt(receipt_out.as_ref(), "remote dataspace deliver receipt", &delivered.receipt_value)?;
            println!(
                "remote deliver-local ok envelope={} topic={} receiver={}",
                delivered.envelope.envelope_ref, topic, receiver_peer
            );
            Ok(())
        }
        RemoteCommand::RunTwoPeer { transport_root, out } => {
            let harness =
                remote_dataspace::two_peer_service_ready_harness(&transport_root, remote_evidence_fixture()?)?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("delivery-log.preserves"), &to_text(&harness.delivery_log.value)?)?;
            write_file(&out.join("admission-receipt.preserves"), &to_text(&harness.admission_receipt_value)?)?;
            write_file(&out.join("gate-receipt.preserves"), &to_text(&harness.gate_receipt_value)?)?;
            let turn_context_ref = remote_gate_turn_context_ref(&harness.gate_receipt_value)?;
            write_file(&out.join("turn-context-ref.preserves"), &to_text(&string(&turn_context_ref))?)?;
            let summary = record("remote-dataspace-summary-v1", vec![
                record("delivery-log", vec![string(&harness.delivery_log.log_ref)]),
                record("admission-receipt", vec![string(&canonical_hash(&harness.admission_receipt_value)?)]),
                record("gate-receipt", vec![string(&canonical_hash(&harness.gate_receipt_value)?)]),
                record("turn-context-ref", vec![string(&turn_context_ref)]),
            ]);
            write_file(&out.join("summary.preserves"), &to_text(&summary)?)?;
            println!(
                "remote run-two-peer ok delivery_log={} gate_receipt={} out={}",
                harness.delivery_log.log_ref,
                canonical_hash(&harness.gate_receipt_value)?,
                out.display()
            );
            Ok(())
        }
        RemoteCommand::Gate {
            delivery_log,
            admission_receipts,
            turn_context_refs,
            receipt_out,
        } => {
            let log_value = read_preserves_file(&delivery_log)?;
            let log = remote_dataspace::parse_delivery_log(&log_value)?;
            let receipts =
                admission_receipts.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let receipt = remote_dataspace::remote_dataspace_gate_receipt_value(&log, &receipts, &turn_context_refs)?;
            emit_named_receipt(receipt_out.as_ref(), "remote dataspace gate receipt", &receipt)
        }
    }
}

fn run_remote_envelope_command(command: RemoteEnvelopeCommand) -> Result<()> {
    match command {
        RemoteEnvelopeCommand::Build {
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
            let payload = read_preserves_file(&payload)?;
            let operation = parse_remote_operation(&operation)?;
            let envelope = remote_dataspace::build_envelope(remote_dataspace::RemoteDataspaceEnvelopeInput {
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
            write_file(&out, &to_text(&envelope.value)?)?;
            println!("remote envelope {} written to {}", envelope.envelope_ref, out.display());
            Ok(())
        }
    }
}

fn parse_remote_operation(operation: &str) -> Result<remote_dataspace::RemoteDataspaceOperation> {
    match operation {
        "message" => Ok(remote_dataspace::RemoteDataspaceOperation::Message),
        "assert" => Ok(remote_dataspace::RemoteDataspaceOperation::Assert),
        "retract" => Ok(remote_dataspace::RemoteDataspaceOperation::Retract),
        "observe" => Ok(remote_dataspace::RemoteDataspaceOperation::Observe),
        _ => Err(MoltenError::invalid_harness(format!(
            "unsupported remote dataspace operation {operation}; expected message/assert/retract/observe"
        ))),
    }
}

fn remote_evidence_fixture() -> Result<remote_dataspace::RemoteDeliveryEvidence> {
    Ok(remote_dataspace::RemoteDeliveryEvidence {
        peer_bootstrap_refs: vec![remote_cli_synthetic_ref("remote-bootstrap")?],
        capability_refs: vec![remote_cli_synthetic_ref("remote-capability")?],
        policy_refs: vec![remote_cli_synthetic_ref("remote-policy")?],
        resource_refs: vec![remote_cli_synthetic_ref("remote-resource")?],
        authority_refs: vec![remote_cli_synthetic_ref("remote-authority")?],
    })
}

fn remote_cli_synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("remote-cli-ref", vec![string(label)]))
}

fn remote_gate_turn_context_ref(gate_receipt: &preserves::IOValue) -> Result<String> {
    let fields = gate_receipt
        .collect_simple_record("remote-dataspace-gate-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected remote dataspace gate receipt"))?;
    let context = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let refs = context
        .collect_simple_record("turn-journal-context-refs", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected remote turn context refs"))?;
    let sequence = refs[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected turn context ref sequence"))?;
    let first = sequence.iter().next().ok_or_else(|| MoltenError::invalid_harness("missing turn context ref"))?;
    first
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness("expected string turn context ref"))
}

pub fn remote_dataspace_gate_summary(value: &preserves::IOValue) -> Result<String> {
    if molten::ledger::artifact_kind(value) != "remote-dataspace-gate-receipt" {
        return Err(MoltenError::invalid_harness("not a remote dataspace gate receipt"));
    }
    Ok(format!("remote dataspace gate receipt ref={}", canonical_hash(value)?))
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
