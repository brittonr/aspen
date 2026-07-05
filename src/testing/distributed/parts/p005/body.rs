fn ci_matrix_value(decision: &str, profiles: &[CiProfile], diagnostics: &[String]) -> Result<IoValue> {
    validate_decision(decision)?;
    validate_strings("distributed matrix diagnostic", diagnostics, MAX_DISTRIBUTED_TEXT)?;
    Ok(record("distributed-ci-matrix-v1", vec![
        string(DISTRIBUTED_CI_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profiles", vec![sequence(profile_values(profiles)?)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("profiles-explicit", status(profiles.len() == REQUIRED_DISTRIBUTED_PROFILE_COUNT)),
            ("artifact-kinds-declared", status(diagnostics.iter().all(|item| !item.contains("artifact-kind")))),
            ("retry-success-not-pass-evidence", PASS_DECISION),
            ("unavailable-is-not-pass", PASS_DECISION),
        ]),
    ]))
}

fn profile_values(profiles: &[CiProfile]) -> Result<Vec<IoValue>> {
    profiles
        .iter()
        .map(|profile| {
            Ok(record("profile", vec![
                record("id", vec![string(&profile.id)]),
                record("purpose", vec![string(&profile.purpose)]),
                record("command", vec![string(&profile.command)]),
                record("artifact-kinds", vec![sequence(profile.expected_artifact_kinds.iter().map(string).collect())]),
                record("evidence-scope", vec![string(&profile.evidence_scope)]),
                record("cost-class", vec![string(&profile.cost_class)]),
                record("release-review-status", vec![string(&profile.release_review_status)]),
            ]))
        })
        .collect()
}

fn validate_metadata_input(input: &TestMetadataInput) -> Result<()> {
    validate_ref(&input.source_ref, "distributed metadata source")?;
    validate_ref_slice("distributed metadata nix input", &input.nix_input_refs)?;
    validate_ref(&input.test_binary_ref, "distributed metadata test binary")?;
    validate_text("distributed metadata profile", &input.profile_id)?;
    validate_text("distributed metadata command", &input.command)?;
    if input.expected_artifact_kinds.is_empty() {
        return Err(MoltenError::invalid_harness("distributed metadata requires artifact kinds"));
    }
    validate_strings("distributed metadata artifact kind", &input.expected_artifact_kinds, MAX_DISTRIBUTED_TEXT)?;
    validate_cost_class(&input.cost_class)?;
    validate_release_status(&input.release_review_status)?;
    validate_text("distributed metadata shard", &input.shard_id)?;
    validate_ref(&input.seed_ref, "distributed metadata seed")?;
    validate_ref(&input.topology_ref, "distributed metadata topology")?;
    validate_ref(&input.fault_plan_ref, "distributed metadata fault plan")?;
    validate_ref_slice("distributed metadata receipt", &input.receipt_refs)?;
    validate_ref_slice("distributed metadata variance", &input.variance_refs)?;
    validate_ref_slice("distributed metadata diagnostic log", &input.diagnostic_log_refs)?;
    if input.receipt_refs.is_empty() {
        return Err(MoltenError::invalid_harness("distributed metadata requires receipt refs"));
    }
    if input.variance_refs.is_empty() {
        return Err(MoltenError::invalid_harness("distributed metadata requires variance refs"));
    }
    Ok(())
}

fn test_metadata_value(input: &TestMetadataInput) -> Result<IoValue> {
    Ok(record("distributed-test-metadata-v1", vec![
        string(DISTRIBUTED_TEST_METADATA_SCHEMA),
        record("source", vec![string(&input.source_ref)]),
        record("nix-inputs", vec![refs_sequence(&input.nix_input_refs)]),
        record("test-binary", vec![string(&input.test_binary_ref)]),
        record("profile", vec![string(&input.profile_id)]),
        record("command", vec![string(&input.command)]),
        record("artifact-kinds", vec![sequence(input.expected_artifact_kinds.iter().map(string).collect())]),
        record("cost-class", vec![string(&input.cost_class)]),
        record("release-review-status", vec![string(&input.release_review_status)]),
        record("shard", vec![string(&input.shard_id)]),
        record("seed", vec![string(&input.seed_ref)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("fault-plan", vec![string(&input.fault_plan_ref)]),
        record("receipts", vec![refs_sequence(&input.receipt_refs)]),
        record("variance", vec![refs_sequence(&input.variance_refs)]),
        record("diagnostic-logs", vec![refs_sequence(&input.diagnostic_log_refs)]),
        checks_value(&[
            ("source-bound", PASS_DECISION),
            ("profile-command-bound", PASS_DECISION),
            ("profile-artifacts-bound", PASS_DECISION),
            ("profile-cost-bound", PASS_DECISION),
            ("profile-release-status-bound", PASS_DECISION),
            ("profile-and-shard-bound", PASS_DECISION),
            ("variance-declared", PASS_DECISION),
            ("logs-diagnostic-only", PASS_DECISION),
        ]),
    ]))
}

fn gate_diagnostics(input: &CiGateInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.matrix.decision != PASS_DECISION {
        diagnostics.push_bounded("distributed-matrix-denied".to_string())?;
    }
    if input.traceability_manifest.decision != PASS_DECISION {
        diagnostics.push_bounded("distributed-traceability-denied".to_string())?;
    }
    let metadata_refs = input.metadata.iter().map(|metadata| metadata.metadata_ref.as_str()).collect::<OrderedSet<_>>();
    let profile_ids = input.matrix.profiles.iter().map(|profile| profile.id.as_str()).collect::<OrderedSet<_>>();
    let mut metadata_profile_ids = OrderedSet::new();
    for metadata in input.metadata {
        validate_metadata_surface(metadata)?;
        if !metadata_profile_ids.insert(metadata.profile_id.as_str()) {
            diagnostics.push_bounded(format!("duplicate-metadata-profile:{}", metadata.profile_id))?;
        }
        match input.matrix.profiles.iter().find(|profile| profile.id == metadata.profile_id) {
            Some(profile) => collect_metadata_mismatch_diagnostics(metadata, profile, &mut diagnostics)?,
            None => diagnostics.push_bounded(format!("metadata-profile-not-in-matrix:{}", metadata.profile_id))?,
        }
    }
    for run in input.runs {
        validate_profile_run(run)?;
        if !profile_ids.contains(run.profile_id.as_str()) {
            diagnostics.push_bounded(format!("run-profile-not-in-matrix:{}", run.profile_id))?;
        }
        if !metadata_refs.contains(run.metadata_ref.as_str()) {
            diagnostics.push_bounded(format!("run-metadata-ref-missing:{}", run.profile_id))?;
        }
        if run.traceability_ref != input.traceability_manifest.manifest_ref {
            diagnostics.push_bounded(format!("run-traceability-ref-mismatch:{}", run.profile_id))?;
        }
        if !run.positive_coverage {
            diagnostics.push_bounded(format!("missing-positive-coverage:{}", run.profile_id))?;
        }
        if !run.negative_coverage {
            diagnostics.push_bounded(format!("missing-negative-coverage:{}", run.profile_id))?;
        }
        if run.retry_attempts > 0 && run.decision == PASS_DECISION {
            diagnostics.push_bounded(format!("retry-only-success-denied:{}", run.profile_id))?;
        }
        if run.unavailable && run.decision == PASS_DECISION {
            diagnostics.push_bounded(format!("unavailable-profile-cannot-pass:{}", run.profile_id))?;
        }
        if run.unavailable && run.required_for_release {
            diagnostics.push_bounded(format!("required-profile-unavailable:{}", run.profile_id))?;
        }
        if !run.variance_declared {
            diagnostics.push_bounded(format!("undeclared-variance:{}", run.profile_id))?;
        }
    }
    for profile in &input.matrix.profiles {
        if profile.release_review_status == RELEASE_REQUIRED
            && !input.runs.iter().any(|run| run.profile_id == profile.id)
        {
            diagnostics.push_bounded(format!("missing-required-profile-run:{}", profile.id))?;
        }
    }
    Ok(diagnostics)
}

fn validate_metadata_surface(metadata: &TestMetadata) -> Result<()> {
    validate_text("distributed metadata profile", &metadata.profile_id)?;
    validate_text("distributed metadata command", &metadata.command)?;
    if metadata.expected_artifact_kinds.is_empty() {
        return Err(MoltenError::invalid_harness("distributed metadata requires artifact kinds"));
    }
    validate_strings("distributed metadata artifact kind", &metadata.expected_artifact_kinds, MAX_DISTRIBUTED_TEXT)?;
    validate_cost_class(&metadata.cost_class)?;
    validate_release_status(&metadata.release_review_status)?;
    validate_text("distributed metadata shard", &metadata.shard_id)?;
    validate_ref(&metadata.metadata_ref, "distributed metadata ref")
}

fn collect_metadata_mismatch_diagnostics(
    metadata: &TestMetadata,
    profile: &CiProfile,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<()> {
    if metadata.command != profile.command {
        diagnostics.push_bounded(format!("metadata-command-mismatch:{}", metadata.profile_id))?;
    }
    if metadata.expected_artifact_kinds != profile.expected_artifact_kinds {
        diagnostics.push_bounded(format!("metadata-artifact-kinds-mismatch:{}", metadata.profile_id))?;
    }
    if metadata.cost_class != profile.cost_class {
        diagnostics.push_bounded(format!("metadata-cost-class-mismatch:{}", metadata.profile_id))?;
    }
    if metadata.release_review_status != profile.release_review_status {
        diagnostics.push_bounded(format!("metadata-release-status-mismatch:{}", metadata.profile_id))?;
    }
    Ok(())
}

fn validate_profile_run(run: &ProfileRun) -> Result<()> {
    validate_text("profile run id", &run.profile_id)?;
    validate_decision(&run.decision)?;
    validate_ref(&run.metadata_ref, "profile run metadata")?;
    validate_ref(&run.traceability_ref, "profile run traceability")?;
    if let Some(reason) = &run.unsupported_reason {
        validate_text("profile run unsupported reason", reason)?;
    }
    Ok(())
}

fn ci_gate_value(
    decision: &str,
    matrix_ref: &str,
    traceability_ref: &str,
    metadata_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_decision(decision)?;
    validate_ref(matrix_ref, "distributed CI matrix")?;
    validate_ref(traceability_ref, "distributed traceability")?;
    validate_ref_slice("distributed metadata", metadata_refs)?;
    validate_strings("distributed CI gate diagnostic", diagnostics, MAX_DISTRIBUTED_TEXT)?;
    Ok(record("distributed-ci-gate-v1", vec![
        string(DISTRIBUTED_CI_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("matrix", vec![string(matrix_ref)]),
        record("traceability", vec![string(traceability_ref)]),
        record("metadata", vec![refs_sequence(metadata_refs)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("positive-coverage-required", status(!diagnostics.iter().any(|item| item.contains("positive")))),
            ("negative-coverage-required", status(!diagnostics.iter().any(|item| item.contains("negative")))),
            ("zero-retry-pass-required", status(!diagnostics.iter().any(|item| item.contains("retry")))),
            ("unavailable-not-pass", status(!diagnostics.iter().any(|item| item.contains("unavailable")))),
        ]),
    ]))
}

fn collect_composite_case_diagnostics(case: &CompositeFaultCase, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    validate_text("composite case id", &case.case_id)?;
    validate_text("composite invariant", &case.invariant_name)?;
    validate_decision(&case.expected_decision)?;
    validate_cost_class(&case.cost_class)?;
    validate_strings("composite profile", &case.profile_eligibility, MAX_DISTRIBUTED_TEXT)?;
    validate_strings("composite caveat", &case.caveats, MAX_DISTRIBUTED_TEXT)?;
    if case.profile_eligibility.is_empty() {
        diagnostics.push_bounded(format!("composite-case-missing-profile:{}", case.case_id))?;
    }
    if case.caveats.is_empty() {
        diagnostics.push_bounded(format!("composite-case-missing-caveat:{}", case.case_id))?;
    }
    Ok(())
}

fn composite_fault_case_value(case: &CompositeFaultCase) -> Result<IoValue> {
    let topology_ref = canonical_ref(&topology_value(&case.simulation.topology)?)?;
    let scheduler_ref = canonical_ref(&scheduler_profile_value(&case.simulation.scheduler)?)?;
    let seed_ref = canonical_ref(&seed_value(&case.simulation.seed)?)?;
    let fault_plan_ref = canonical_ref(&fault_plan_value(&case.simulation.fault_plan)?)?;
    let command_ids = case.simulation.commands.iter().map(|command| command.operation_id.clone()).collect::<Vec<_>>();
    Ok(record("composite-fault-case-v1", vec![
        string(COMPOSITE_FAULT_CASE_SCHEMA),
        record("id", vec![string(&case.case_id)]),
        record("invariant", vec![string(&case.invariant_name)]),
        record("expected-decision", vec![string(&case.expected_decision)]),
        record("topology", vec![string(topology_ref)]),
        record("scheduler", vec![string(scheduler_ref)]),
        record("seed", vec![string(seed_ref)]),
        record("fault-plan", vec![string(fault_plan_ref)]),
        record("commands", vec![sequence(command_ids.iter().map(string).collect())]),
        record("profiles", vec![sequence(case.profile_eligibility.iter().map(string).collect())]),
        record("cost-class", vec![string(&case.cost_class)]),
        record("caveats", vec![sequence(case.caveats.iter().map(string).collect())]),
        checks_value(&[
            ("named-case", PASS_DECISION),
            ("deterministic-inputs-bound", PASS_DECISION),
            ("retry-success-not-pass-evidence", PASS_DECISION),
        ]),
    ]))
}

