type Artifacts = super::command::Artifacts;
type Command = super::OctetCommand;
type Outcome<T> = molten::error::Result<T>;
type Remediation = super::command::Remediation;
type Review = super::command::Review;
type SourceGate = super::command::SourceGate;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Gate {
            artifacts,
            profile,
            receipt_out,
        } => {
            let evaluation = molten::octet_gate::evaluate_octet_gate(&molten::octet_gate::OctetGateInput {
                artifacts_dir: artifacts.clone(),
                profile,
            })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "octet gate receipt", &evaluation.receipt_value)?;
            if evaluation.decision != "pass" {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "octet gate denied receipt={} artifacts={}",
                    evaluation.receipt_ref,
                    artifacts.display()
                )));
            }
            println!("octet gate pass receipt={}", evaluation.receipt_ref);
            Ok(())
        }
        Command::SourceGate { command } => run_source_gate(command),
        Command::Baseline { command } => super::baseline::run(command),
        Command::Review { command } => run_review(command),
        Command::Artifacts { command } => run_artifacts(command),
        Command::Remediation { command } => run_remediation(command),
    }
}

fn run_remediation(command: Remediation) -> Outcome<()> {
    match command {
        Remediation::Plan {
            artifacts,
            lib_artifacts,
            focused_object_corpus,
            receipt_out,
        } => {
            let plan = molten::octet_remediation::build_plan(&molten::octet_remediation::PlanInput {
                artifacts_dir: artifacts,
                lib_artifacts_dir: lib_artifacts,
                focused_object_corpus,
            })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "octet remediation plan", &plan.value)?;
            println!("octet remediation plan receipt={}", plan.plan_ref);
            Ok(())
        }
    }
}

fn run_source_gate(command: SourceGate) -> Outcome<()> {
    match command {
        SourceGate::Validate {
            consumer,
            subject,
            gate_receipt,
            source_scope,
            receipt_out,
        } => {
            let receipt_value = super::io::read_preserves_file(&gate_receipt)?;
            let validation =
                molten::octet_gate::validate_octet_source_gate(&molten::octet_gate::OctetSourceGateValidationInput {
                    consumer,
                    subject_ref: subject,
                    receipt_value: Some(receipt_value),
                    source_scope,
                })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "octet source gate validation", &validation.value)?;
            if validation.decision != "pass" {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "octet source gate validation denied receipt={}",
                    validation.validation_ref
                )));
            }
            println!("octet source gate validation pass receipt={}", validation.validation_ref);
            Ok(())
        }
    }
}

fn run_artifacts(command: Artifacts) -> Outcome<()> {
    match command {
        Artifacts::Import {
            artifacts,
            ledger,
            receipt_out,
        } => {
            let imported =
                molten::octet_gate::import_octet_artifacts_to_ledger(&molten::octet_gate::OctetArtifactLedgerInput {
                    artifacts_dir: artifacts,
                    ledger_root: ledger.clone(),
                })?;
            super::io::emit_named_receipt(
                receipt_out.as_ref(),
                "octet artifact ledger receipt",
                &imported.receipt_value,
            )?;
            println!(
                "octet artifacts import decision={} receipt={} imported={} ledger={}",
                imported.decision,
                imported.receipt_ref,
                imported.imported_refs.len(),
                ledger.display()
            );
            Ok(())
        }
    }
}

fn run_review(command: Review) -> Outcome<()> {
    match command {
        Review::Write {
            out,
            profile,
            expires_at,
            finding_keys,
            rationale,
        } => {
            let review =
                molten::octet_gate::build_octet_review_manifest(&molten::octet_gate::OctetReviewManifestInput {
                    profile,
                    expires_at,
                    finding_keys,
                    rationale,
                })?;
            super::io::write_file(&out, &molten::preserves_rail::to_text(&review.review_value)?)?;
            println!("octet review manifest {} written to {}", review.review_ref, out.display());
            Ok(())
        }
    }
}
