type Command = super::GateCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Check {
            artifact,
            failure_out,
            receipt_out,
        } => run_check(artifact, failure_out, receipt_out),
        Command::ReleaseProfile {
            profile_id,
            tier,
            candidate_ref,
            source_gate_ref,
            policy_ref,
            octet_ref,
            cairn_ref,
            stack_provenance_ref,
            production_profile_ref,
            expected_generated_export_ref,
            actual_generated_export_ref,
            stack_provenance_required,
            accepted_valence_policy_hashes,
            caveats,
            out,
        } => run_release_profile(ReleaseProfileArgs {
            profile_id,
            tier,
            candidate_ref,
            source_gate_ref,
            policy_ref,
            octet_ref,
            cairn_ref,
            stack_provenance_ref,
            production_profile_ref,
            expected_generated_export_ref,
            actual_generated_export_ref,
            stack_provenance_required,
            accepted_valence_policy_hashes,
            caveats,
            out,
        }),
    }
}

struct ReleaseProfileArgs {
    profile_id: String,
    tier: String,
    candidate_ref: Option<String>,
    source_gate_ref: Option<String>,
    policy_ref: Option<String>,
    octet_ref: Option<String>,
    cairn_ref: Option<String>,
    stack_provenance_ref: Option<String>,
    production_profile_ref: Option<String>,
    expected_generated_export_ref: Option<String>,
    actual_generated_export_ref: Option<String>,
    stack_provenance_required: bool,
    accepted_valence_policy_hashes: Vec<String>,
    caveats: Vec<String>,
    out: Option<std::path::PathBuf>,
}

fn run_release_profile(args: ReleaseProfileArgs) -> Outcome<()> {
    let validation =
        molten::prod_release_profile::validate_release_profile(&molten::prod_release_profile::ReleaseProfileInput {
            profile_id: args.profile_id,
            tier: args.tier,
            candidate_ref: args.candidate_ref,
            evidence_refs: molten::prod_release_profile::ReleaseEvidenceRefs {
                source_gate_ref: args.source_gate_ref,
                policy_ref: args.policy_ref,
                octet_ref: args.octet_ref,
                cairn_ref: args.cairn_ref,
                stack_provenance_ref: args.stack_provenance_ref,
                production_profile_ref: args.production_profile_ref,
            },
            freshness: molten::prod_release_profile::ReleaseProfileFreshness {
                expected_generated_export_ref: args.expected_generated_export_ref,
                actual_generated_export_ref: args.actual_generated_export_ref,
            },
            stack_provenance_required: args.stack_provenance_required,
            accepted_valence_policy_hashes: args.accepted_valence_policy_hashes,
            caveats: args.caveats,
        })?;
    super::io::emit_gate_receipt(args.out.as_ref(), &validation.value)?;
    if validation.decision == "deny" {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "release profile denied: {}",
            validation.diagnostics.join(",")
        )));
    }
    eprintln!("release profile pass ref={}", validation.validation_ref);
    Ok(())
}

fn run_check(
    artifact: std::path::PathBuf,
    failure_out: Option<std::path::PathBuf>,
    receipt_out: Option<std::path::PathBuf>,
) -> Outcome<()> {
    let artifact_value = super::io::read_preserves_file_with_failure(&artifact, failure_out.as_ref(), "validate")?;
    let check = match molten::harness::check_value(&artifact_value) {
        Ok(check) => check,
        Err(error) => {
            super::io::write_optional_artifact_failure(failure_out.as_ref(), "validate", &error, &artifact_value)?;
            return Err(error);
        }
    };
    let receipt = molten::harness::receipt_value(&check);
    if let Err(error) = super::io::emit_gate_receipt(receipt_out.as_ref(), &receipt) {
        super::io::write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &artifact_value)?;
        return Err(error);
    }
    Ok(())
}
