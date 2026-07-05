fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::refs_sequence(refs)
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::checks_value(checks)
}

pub fn default_ci_profiles() -> Vec<CiProfile> {
    vec![
        CiProfile {
            id: PROFILE_FAST.to_string(),
            purpose: "pure core, unit, parser, and receipt validation checks".to_string(),
            command: "cargo nextest run --profile deterministic".to_string(),
            expected_artifact_kinds: vec!["libtest".to_string(), "junit".to_string()],
            evidence_scope: "no platform or transport claims".to_string(),
            cost_class: COST_FAST.to_string(),
            release_review_status: RELEASE_REQUIRED.to_string(),
        },
        CiProfile {
            id: PROFILE_PROTOCOL.to_string(),
            purpose: "deterministic simulation, model fixtures, and drift checks".to_string(),
            command: "cargo test --lib distributed".to_string(),
            expected_artifact_kinds: vec![
                "distributed-test-run-v1".to_string(),
                "distributed-fault-plan-v1".to_string(),
            ],
            evidence_scope: "simulated distributed invariants".to_string(),
            cost_class: COST_FAST.to_string(),
            release_review_status: RELEASE_REQUIRED.to_string(),
        },
        CiProfile {
            id: PROFILE_CLI.to_string(),
            purpose: "CLI receipt and traceability workflow checks".to_string(),
            command: "nix build .#checks.x86_64-linux.requirement-traceability-gate".to_string(),
            expected_artifact_kinds: vec!["requirement-traceability-gate-v1".to_string()],
            evidence_scope: "local process and receipt behavior".to_string(),
            cost_class: COST_MEDIUM.to_string(),
            release_review_status: RELEASE_REQUIRED.to_string(),
        },
        CiProfile {
            id: PROFILE_VM_SMOKE.to_string(),
            purpose: "two-node NixOS platform smoke topology".to_string(),
            command: "nix build .#checks.x86_64-linux.nixos-vm-multinode".to_string(),
            expected_artifact_kinds: vec![
                "nixos-vm-test-run-v1".to_string(),
                "nixos-vm-evidence-validation-v1".to_string(),
            ],
            evidence_scope: "platform integration smoke evidence".to_string(),
            cost_class: COST_HEAVY.to_string(),
            release_review_status: RELEASE_REQUIRED_WHEN_SUPPORTED.to_string(),
        },
        CiProfile {
            id: PROFILE_VM_FAULT.to_string(),
            purpose: "executable VM network, restart, and state-root fault checks".to_string(),
            command: "nix build .#checks.x86_64-linux.nixos-vm-multinode".to_string(),
            expected_artifact_kinds: vec!["nixos-vm-fault-receipt-v1".to_string()],
            evidence_scope: "bounded executable platform fault evidence".to_string(),
            cost_class: COST_HEAVY.to_string(),
            release_review_status: RELEASE_REQUIRED_WHEN_SUPPORTED.to_string(),
        },
        CiProfile {
            id: PROFILE_SOAK.to_string(),
            purpose: "dogfood, pilot, and production-shaped evidence review".to_string(),
            command: "nix build .#checks.x86_64-linux.dogfood-local-node".to_string(),
            expected_artifact_kinds: vec![
                "prod-soak-run-v1".to_string(),
                "operator-release-gate-receipt-v1".to_string(),
            ],
            evidence_scope: "constrained pilot/readiness review only".to_string(),
            cost_class: COST_HEAVY.to_string(),
            release_review_status: RELEASE_PILOT_SCOPE.to_string(),
        },
    ]
}

pub fn build_ci_matrix(profiles: Vec<CiProfile>) -> Result<CiMatrix> {
    ensure_count_at_most(profiles.len(), MAX_DISTRIBUTED_PROFILES, "distributed CI profiles")?;
    let diagnostics = matrix_diagnostics(&profiles)?;
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let value = ci_matrix_value(&decision, &profiles, &diagnostics)?;
    let matrix_ref = canonical_ref(&value)?;
    Ok(CiMatrix {
        decision,
        profiles,
        diagnostics,
        matrix_ref,
        value,
    })
}

pub fn build_test_metadata(input: &TestMetadataInput) -> Result<TestMetadata> {
    validate_metadata_input(input)?;
    let value = test_metadata_value(input)?;
    let metadata_ref = canonical_ref(&value)?;
    Ok(TestMetadata {
        profile_id: input.profile_id.clone(),
        command: input.command.clone(),
        expected_artifact_kinds: input.expected_artifact_kinds.clone(),
        cost_class: input.cost_class.clone(),
        release_review_status: input.release_review_status.clone(),
        shard_id: input.shard_id.clone(),
        metadata_ref,
        value,
    })
}

pub fn evaluate_ci_gate(input: &CiGateInput<'_>) -> Result<CiGate> {
    let mut diagnostics = gate_diagnostics(input)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let metadata_refs = input.metadata.iter().map(|metadata| metadata.metadata_ref.clone()).collect::<Vec<_>>();
    let value = ci_gate_value(
        &decision,
        &input.matrix.matrix_ref,
        &input.traceability_manifest.manifest_ref,
        &metadata_refs,
        &diagnostics,
    )?;
    let gate_ref = canonical_ref(&value)?;
    Ok(CiGate {
        decision,
        diagnostics,
        matrix_ref: input.matrix.matrix_ref.clone(),
        traceability_ref: input.traceability_manifest.manifest_ref.clone(),
        metadata_refs,
        gate_ref,
        value,
    })
}

pub fn evaluate_composite_fault_suite(cases: &[CompositeFaultCase]) -> Result<CompositeFaultSuite> {
    ensure_count_at_most(cases.len(), MAX_DISTRIBUTED_PROFILES, "composite fault cases")?;
    let mut diagnostics = Vec::new();
    if cases.is_empty() {
        diagnostics.push_bounded("composite-suite-empty".to_string())?;
    }
    let mut case_refs = Vec::with_capacity(cases.len());
    let mut run_refs = Vec::with_capacity(cases.len());
    for case in cases {
        collect_composite_case_diagnostics(case, &mut diagnostics)?;
        let case_value = composite_fault_case_value(case)?;
        case_refs.push(canonical_ref(&case_value)?);
        let run = run_simulation(&case.simulation)?;
        run_refs.push(run.receipt_ref.clone());
        if run.decision != case.expected_decision {
            diagnostics.push_bounded(format!("composite-case-decision-mismatch:{}", case.case_id))?;
        }
        if case.expected_decision == DENY_DECISION && run.denied_operation_ids.is_empty() {
            diagnostics.push_bounded(format!("composite-case-missing-denial:{}", case.case_id))?;
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let value = composite_fault_suite_value(&decision, &case_refs, &run_refs, &diagnostics)?;
    let suite_ref = canonical_ref(&value)?;
    Ok(CompositeFaultSuite {
        decision,
        diagnostics,
        case_refs,
        run_refs,
        suite_ref,
        value,
    })
}

pub fn evaluate_generated_case_promotion(input: &GeneratedCasePromotionInput) -> Result<GeneratedCasePromotion> {
    let mut diagnostics = generated_case_promotion_diagnostics(input)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
    .to_string();
    let value = generated_case_promotion_value(input, &decision, &diagnostics)?;
    let promotion_ref = canonical_ref(&value)?;
    Ok(GeneratedCasePromotion {
        decision,
        diagnostics,
        promotion_ref,
        value,
    })
}

fn matrix_diagnostics(profiles: &[CiProfile]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let mut ids = OrderedSet::new();
    for profile in profiles {
        validate_profile(profile, &mut diagnostics)?;
        if !ids.insert(profile.id.as_str()) {
            diagnostics.push_bounded(format!("duplicate-profile:{}", profile.id))?;
        }
    }
    for required in required_profile_ids() {
        if !ids.contains(required) {
            diagnostics.push_bounded(format!("missing-profile:{required}"))?;
        }
    }
    Ok(diagnostics)
}

fn validate_profile(profile: &CiProfile, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    validate_text("distributed profile id", &profile.id)?;
    validate_text("distributed profile purpose", &profile.purpose)?;
    validate_text("distributed profile command", &profile.command)?;
    validate_text("distributed profile evidence scope", &profile.evidence_scope)?;
    validate_cost_class(&profile.cost_class)?;
    validate_release_status(&profile.release_review_status)?;
    if profile.expected_artifact_kinds.is_empty() {
        diagnostics.push_bounded(format!("profile-missing-artifact-kind:{}", profile.id))?;
    }
    validate_strings("distributed profile artifact kind", &profile.expected_artifact_kinds, MAX_DISTRIBUTED_TEXT)
}

fn validate_cost_class(value: &str) -> Result<()> {
    match value {
        COST_FAST | COST_MEDIUM | COST_HEAVY => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported distributed profile cost class {other}"))),
    }
}

fn validate_release_status(value: &str) -> Result<()> {
    match value {
        RELEASE_REQUIRED | RELEASE_REQUIRED_WHEN_SUPPORTED | RELEASE_PILOT_SCOPE => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported distributed profile release status {other}"))),
    }
}

fn required_profile_ids() -> [&'static str; REQUIRED_DISTRIBUTED_PROFILE_COUNT] {
    [
        PROFILE_FAST,
        PROFILE_PROTOCOL,
        PROFILE_CLI,
        PROFILE_VM_SMOKE,
        PROFILE_VM_FAULT,
        PROFILE_SOAK,
    ]
}

