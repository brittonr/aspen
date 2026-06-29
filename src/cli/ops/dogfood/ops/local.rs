type Command = super::Command;
type Outcome<T> = super::Outcome<T>;

pub(super) fn local_node(command: Command) -> Outcome<()> {
    let Command::LocalNode {
        state_root,
        out,
        release_gate_out,
        replay_verify_out,
        replay_index_out,
    } = command
    else {
        return Err(super::wrong_handler("local-node"));
    };
    let run = molten::operator_dogfood::run_local_node_dogfood(&molten::operator_dogfood::LocalNodeDogfoodInput {
        state_root: &state_root,
    })?;
    super::super::io::write_file(&out, &molten::preserves_rail::to_text(&run.report_value)?)?;
    if let (Some(path), Some(value)) = (release_gate_out.as_ref(), run.release_gate_value.as_ref()) {
        super::super::io::write_file(path, &molten::preserves_rail::to_text(value)?)?;
    }
    if let (Some(path), Some(value)) = (replay_verify_out.as_ref(), run.replay_verify_value.as_ref()) {
        super::super::io::write_file(path, &molten::preserves_rail::to_text(value)?)?;
    }
    if let (Some(path), Some(value)) = (replay_index_out.as_ref(), run.replay_index_value.as_ref()) {
        super::super::io::write_file(path, &molten::preserves_rail::to_text(value)?)?;
    }
    println!(
        "dogfood local-node decision={} report={} release-gate={}",
        run.decision,
        run.report_ref,
        run.release_gate_ref.as_deref().unwrap_or("none")
    );
    Ok(())
}

pub(super) fn nix_release_export(command: Command) -> Outcome<()> {
    let Command::NixReleaseExport { output_path, out } = command else {
        return Err(super::wrong_handler("nix-release-export"));
    };
    let evidence = molten::operator_dogfood::nix_dogfood_release_evidence_value(
        &molten::operator_dogfood::NixDogfoodEvidenceInput {
            output_path: &output_path,
        },
    )?;
    let parsed = molten::operator_dogfood::parse_nix_dogfood_evidence(&evidence)?;
    super::super::io::write_file(&out, &molten::preserves_rail::to_text(&evidence)?)?;
    println!(
        "dogfood nix-release-export evidence={} report={} release-gate={}",
        parsed.evidence_ref, parsed.report_ref, parsed.release_gate_ref
    );
    Ok(())
}

pub(super) fn nix_release_verify(command: Command) -> Outcome<()> {
    let Command::NixReleaseVerify {
        output_path,
        evidence,
        receipt_out,
    } = command
    else {
        return Err(super::wrong_handler("nix-release-verify"));
    };
    let evidence_value = super::super::io::read_preserves_file(&evidence)?;
    let receipt =
        molten::operator_dogfood::verify_nix_dogfood_evidence(&molten::operator_dogfood::NixDogfoodVerifyInput {
            output_path: &output_path,
            evidence_value: &evidence_value,
        })?;
    super::super::io::write_file(&receipt_out, &molten::preserves_rail::to_text(&receipt.value)?)?;
    println!(
        "dogfood nix-release-verify decision={} receipt={} evidence={}",
        receipt.decision, receipt.receipt_ref, receipt.evidence_ref
    );
    Ok(())
}
