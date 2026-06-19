type Command = super::ProvenanceCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::BuildRecord {
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
        } => {
            let build_params = super::input::parse_build_params(&build_params)?;
            let value =
                molten::provenance::provenance_build_record_value(&molten::provenance::ProvenanceBuildRecordInput {
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
        Command::VerifyBuild {
            build_record,
            actual_artifact_ref,
            prior_diagnostics,
            receipt_out,
        } => {
            let build_record_value = super::io::read_preserves_file(&build_record)?;
            let verification =
                molten::provenance::verify_provenance_build(&molten::provenance::ProvenanceBuildVerificationInput {
                    build_record_value: &build_record_value,
                    actual_artifact_ref: &actual_artifact_ref,
                    prior_diagnostics: &prior_diagnostics,
                })?;
            let is_written_to_file =
                super::io::write_optional_preserves(receipt_out.as_ref(), &verification.receipt_value)?;
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
        Command::Record {
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
        } => {
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
        Command::Fixture { artifact_ref, out } => {
            let value = molten::provenance::synthetic_reviewed_provenance_record(&artifact_ref)?;
            let reference = molten::preserves_rail::canonical_hash(&value)?;
            let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
            super::io::print_or_log_summary(
                is_written_to_file,
                &format!("provenance fixture ref={reference} artifact={artifact_ref} trust_state=reviewed"),
            );
            Ok(())
        }
        Command::Evaluate {
            operation,
            profile,
            artifact_ref,
            provenance_paths,
            build_verification_paths,
            prior_diagnostics,
            receipt_out,
        } => {
            let mut provenance_values =
                super::input::BoundedItems::new(super::PROVENANCE_CLI_EVIDENCE_LIMIT, "provenance evidence");
            for path in provenance_paths {
                provenance_values.push(super::io::read_preserves_file(&path)?)?;
            }
            let provenance_values = provenance_values.into_vec();
            let mut build_verification_values = super::input::BoundedItems::new(
                super::PROVENANCE_CLI_EVIDENCE_LIMIT,
                "provenance build verification evidence",
            );
            for path in build_verification_paths {
                build_verification_values.push(super::io::read_preserves_file(&path)?)?;
            }
            let build_verification_values = build_verification_values.into_vec();
            let evaluation = molten::provenance::evaluate_provenance(&molten::provenance::ProvenanceEvaluationInput {
                operation: &operation,
                profile: &profile,
                artifact_ref: &artifact_ref,
                provenance_values: &provenance_values,
                build_verification_values: &build_verification_values,
                prior_diagnostics: &prior_diagnostics,
            })?;
            let is_written_to_file =
                super::io::write_optional_preserves(receipt_out.as_ref(), &evaluation.receipt_value)?;
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
        Command::Show { artifact } => {
            let value = super::io::read_preserves_file(&artifact)?;
            println!("{}", molten::provenance::provenance_summary(&value)?);
            Ok(())
        }
    }
}
