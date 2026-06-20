type Command = super::ProvenanceCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        command @ Command::BuildRecord { .. } => build_record(command),
        command @ Command::VerifyBuild { .. } => verify_build(command),
        command @ Command::Record { .. } => record(command),
        Command::Fixture { artifact_ref, out } => fixture(artifact_ref, out),
        command @ Command::Evaluate { .. } => evaluate(command),
        Command::Show { artifact } => show(artifact),
    }
}

fn build_record(command: Command) -> Outcome<()> {
    let Command::BuildRecord {
        expected_artifact_ref,
        source_refs,
        dependency_closure_ref,
        toolchain_refs,
        build_params,
        builder_ref,
        nix_derivation_refs,
        policy_refs,
        evidence_refs,
        out,
    } = command
    else {
        return Err(wrong_handler("build-record"));
    };
    let build_params = super::input::parse_build_params(&build_params)?;
    let value = molten::provenance::provenance_build_record_value(&molten::provenance::ProvenanceBuildRecordInput {
        expected_artifact_ref: &expected_artifact_ref,
        source_refs: &source_refs,
        dependency_closure_ref: &dependency_closure_ref,
        toolchain_refs: &toolchain_refs,
        build_params: &build_params,
        builder_ref: &builder_ref,
        nix_derivation_refs: &nix_derivation_refs,
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("provenance build record ref={reference} expected_artifact={expected_artifact_ref}"),
    );
    Ok(())
}

fn verify_build(command: Command) -> Outcome<()> {
    let Command::VerifyBuild {
        build_record,
        actual_artifact_ref,
        prior_diagnostics,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("verify-build"));
    };
    let build_record_value = super::io::read_preserves_file(&build_record)?;
    let verification =
        molten::provenance::verify_provenance_build(&molten::provenance::ProvenanceBuildVerificationInput {
            build_record_value: &build_record_value,
            actual_artifact_ref: &actual_artifact_ref,
            prior_diagnostics: &prior_diagnostics,
        })?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &verification.receipt_value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "provenance build verification decision={} expected={} actual={} receipt={} record={}",
            verification.decision,
            verification.expected_artifact_ref,
            verification.actual_artifact_ref,
            verification.receipt_ref,
            verification.build_record_ref
        ),
    );
    Ok(())
}

fn record(command: Command) -> Outcome<()> {
    let Command::Record {
        artifact_ref,
        trust_state,
        source_refs,
        dependency_closure_ref,
        toolchain_refs,
        builder_ref,
        review_refs,
        test_refs,
        source_gate_refs,
        policy_refs,
        build_record_refs,
        out,
    } = command
    else {
        return Err(wrong_handler("record"));
    };
    let value = molten::provenance::provenance_record_value(&molten::provenance::ProvenanceRecordInput {
        artifact_ref: &artifact_ref,
        trust_state: &trust_state,
        source_refs: &source_refs,
        dependency_closure_ref: &dependency_closure_ref,
        toolchain_refs: &toolchain_refs,
        builder_ref: &builder_ref,
        review_refs: &review_refs,
        test_refs: &test_refs,
        source_gate_refs: &source_gate_refs,
        policy_refs: &policy_refs,
        build_record_refs: &build_record_refs,
    })?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("provenance record ref={reference} artifact={artifact_ref} trust_state={trust_state}"),
    );
    Ok(())
}

fn fixture(artifact_ref: String, out: Option<FilePath>) -> Outcome<()> {
    let value = molten::provenance::synthetic_reviewed_provenance_record(&artifact_ref)?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("provenance fixture ref={reference} artifact={artifact_ref} trust_state=reviewed"),
    );
    Ok(())
}

fn evaluate(command: Command) -> Outcome<()> {
    let Command::Evaluate {
        operation,
        profile,
        artifact_ref,
        provenance_paths,
        build_verification_paths,
        prior_diagnostics,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("evaluate"));
    };
    let provenance_values = bounded_values(provenance_paths, "provenance evidence")?;
    let build_verification_values = bounded_values(build_verification_paths, "provenance build verification evidence")?;
    let evaluation = molten::provenance::evaluate_provenance(&molten::provenance::ProvenanceEvaluationInput {
        operation: &operation,
        profile: &profile,
        artifact_ref: &artifact_ref,
        provenance_values: &provenance_values,
        build_verification_values: &build_verification_values,
        prior_diagnostics: &prior_diagnostics,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &evaluation.receipt_value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "provenance decision={} operation={} artifact={} receipt={} matched={}",
            evaluation.decision,
            operation,
            artifact_ref,
            evaluation.receipt_ref,
            evaluation.matched_record_ref.as_deref().unwrap_or("none")
        ),
    );
    Ok(())
}

fn bounded_values(paths: Vec<FilePath>, label: &'static str) -> Outcome<Vec<preserves::IOValue>> {
    let mut values = super::input::BoundedItems::new(super::PROVENANCE_CLI_EVIDENCE_LIMIT, label);
    for path in paths {
        values.push(super::io::read_preserves_file(&path)?)?;
    }
    Ok(values.into_vec())
}

fn show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    println!("{}", molten::provenance::provenance_summary(&value)?);
    Ok(())
}

fn wrong_handler(name: &str) -> molten::error::MoltenError {
    molten::error::MoltenError::invalid_harness(format!("provenance {name} handler called with another command"))
}
