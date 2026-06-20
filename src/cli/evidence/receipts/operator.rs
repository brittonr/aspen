type Command = super::ReceiptsCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

struct ExportInput {
    receipt_ref: String,
    ledger: FilePath,
    out: FilePath,
    receipt_out: Option<FilePath>,
}

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::List { ledger } => list(ledger),
        Command::Show { receipt_ref, ledger } => show(receipt_ref, ledger),
        Command::Validate { receipt_ref, ledger } => validate_receipt(receipt_ref, ledger),
        Command::Export {
            receipt_ref,
            ledger,
            out,
            receipt_out,
        } => export(ExportInput {
            receipt_ref,
            ledger,
            out,
            receipt_out,
        }),
        Command::Sign {
            receipt,
            out,
            signer,
            purpose,
            trust_root,
            key,
            parents,
        } => super::signing::run_operator_sign(super::signing::Sign {
            receipt,
            out,
            signer,
            purpose,
            trust_root,
            key,
            parents,
        }),
        Command::VerifySigned {
            signed_receipt,
            purpose,
            trust_root,
            key,
            key_ledger,
            key_ref,
            key_id,
            signer,
            subject_ref,
        } => super::signing::run_operator_verify(super::signing::Verify {
            signed_receipt,
            purpose,
            trust_root,
            key,
            key_ledger,
            key_ref,
            key_id,
            signer,
            subject_ref,
        }),
        Command::Key { command } => super::keys::run(command),
    }
}

fn list(ledger: FilePath) -> Outcome<()> {
    for entry in molten::ledger::list_artifacts(&ledger)? {
        if matches_kind(&entry.artifact_kind) {
            println!("{} {}", entry.artifact_ref, entry.artifact_kind);
        }
    }
    Ok(())
}

fn show(receipt_ref: String, ledger: FilePath) -> Outcome<()> {
    let value = molten::ledger::read_artifact(&ledger, &receipt_ref)?;
    let summary = validate_value(&value)?;
    println!("{summary}");
    Ok(())
}

fn validate_receipt(receipt_ref: String, ledger: FilePath) -> Outcome<()> {
    let value = molten::ledger::read_artifact(&ledger, &receipt_ref)?;
    let summary = validate_value(&value)?;
    println!(
        "receipts validate ok artifact={} kind={} summary={}",
        receipt_ref,
        molten::ledger::artifact_kind(&value),
        summary
    );
    Ok(())
}

fn export(input: ExportInput) -> Outcome<()> {
    let value = molten::ledger::read_artifact(&input.ledger, &input.receipt_ref)?;
    validate_value(&value)?;
    let exported = molten::ledger::export_artifact(&input.ledger, &input.receipt_ref, &input.out)?;
    super::io::emit_named_receipt(input.receipt_out.as_ref(), "receipts export receipt", &exported.receipt_value)?;
    println!(
        "receipts export ok artifact={} kind={} out={} redaction=pass logs=auxiliary",
        exported.artifact_ref,
        exported.artifact_kind,
        input.out.display()
    );
    Ok(())
}

fn validate_value(value: &preserves::IOValue) -> molten::error::Result<String> {
    match molten::ledger::artifact_kind(value) {
        "dogfood-report"
        | "operator-workflow"
        | "operator-checkpoint"
        | "release-gate-receipt"
        | "nix-dogfood-release-evidence"
        | "nix-dogfood-release-verify-receipt"
        | "release-evidence-bundle"
        | "release-evidence-bundle-verify-receipt"
        | "release-promotion-gate-receipt" => molten::operator_dogfood::operator_dogfood_summary(value),
        "signed-receipt" => molten::evidence::signed_receipt_summary(value),
        "operator-step" => {
            let step = molten::operator_dogfood::parse_operator_step(value)?;
            Ok(format!(
                "operator step ref={} name={} decision={} receipt={} (summary is non-normative)",
                step.step_ref,
                step.name,
                step.decision,
                step.receipt_ref.as_deref().unwrap_or("none")
            ))
        }
        kind => Err(molten::error::MoltenError::invalid_harness(format!(
            "unsupported operator receipt kind {kind}; expected dogfood/operator receipt artifact"
        ))),
    }
}

fn matches_kind(kind: &str) -> bool {
    matches!(
        kind,
        "dogfood-report"
            | "operator-workflow"
            | "operator-step"
            | "operator-checkpoint"
            | "release-gate-receipt"
            | "nix-dogfood-release-evidence"
            | "nix-dogfood-release-verify-receipt"
            | "release-evidence-bundle"
            | "release-evidence-bundle-verify-receipt"
            | "release-promotion-gate-receipt"
            | "signed-receipt"
    )
}
