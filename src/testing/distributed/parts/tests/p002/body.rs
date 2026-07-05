    #[test]
    fn generated_case_promotion_requires_budget_traceability_and_zero_retry() {
        // r[verify molten.testing.distributed_simulation.generated_case_promotion_budget]
        let promotion = evaluate_generated_case_promotion(&promotion_input()).expect("promotion");
        let rendered = crate::preserves_rail::to_text(&promotion.value).expect("render promotion");

        assert_eq!(promotion.decision, PASS_DECISION);
        assert!(promotion.diagnostics.is_empty());
        assert!(rendered.contains("generated-case-promotion-v1"));
        assert!(rendered.contains("traceability-required"));
    }

    #[test]
    fn generated_case_promotion_denies_retry_only_and_missing_budget_metadata() {
        // r[verify molten.testing.distributed_simulation.generated_case_promotion_budget]
        let mut input = promotion_input();
        input.command_refs = Vec::new();
        input.profile_eligibility = Vec::new();
        input.traceability_refs = Vec::new();
        input.retry_attempts = 1;
        input.variance_refs = Vec::new();
        input.diagnostic_only = true;
        let promotion = evaluate_generated_case_promotion(&input).expect("promotion");

        assert_eq!(promotion.decision, DENY_DECISION);
        assert!(
            promotion
                .diagnostics
                .iter()
                .any(|item| item == "promotion-missing-command:generated-partition-stale")
        );
        assert!(
            promotion
                .diagnostics
                .iter()
                .any(|item| item == "promotion-missing-profile:generated-partition-stale")
        );
        assert!(
            promotion
                .diagnostics
                .iter()
                .any(|item| item == "promotion-missing-traceability:generated-partition-stale")
        );
        assert!(
            promotion
                .diagnostics
                .iter()
                .any(|item| item == "promotion-retry-only-success:generated-partition-stale")
        );
        assert!(
            promotion
                .diagnostics
                .iter()
                .any(|item| item == "promotion-undeclared-variance:generated-partition-stale")
        );
        assert!(
            promotion
                .diagnostics
                .iter()
                .any(|item| item == "promotion-diagnostic-only-release-claim:generated-partition-stale")
        );
    }

    fn traceability_manifest() -> crate::trace_core::TraceabilityManifest {
        let requirement_ids = [
            "molten.testing.distributed_simulation.direct_fault_fixtures",
            "molten.testing.distributed_simulation.fixture_traceability",
            "molten.testing.distributed_ci.profile_wiring_evidence",
        ];
        let requirements = requirement_ids
            .iter()
            .map(|requirement_id| crate::trace_core::RequirementInput {
                id: (*requirement_id).to_string(),
                source: "cairn/changes/distributed-simulation-fixture-hardening/specs/testing-harness/spec.md"
                    .to_string(),
                kind: "evidence".to_string(),
                changed: true,
            })
            .collect::<Vec<_>>();
        let coverage = requirement_ids
            .iter()
            .map(|requirement_id| crate::trace_core::CoverageInput {
                requirement_id: (*requirement_id).to_string(),
                positive: vec![verification_evidence(requirement_id, "positive")],
                negative: vec![verification_evidence(requirement_id, "negative")],
                exemption: None,
            })
            .collect::<Vec<_>>();
        crate::trace_core::build_traceability_manifest(&crate::trace_core::TraceabilityInput {
            requirements,
            coverage,
            require_receipt_backed: false,
        })
        .expect("traceability manifest")
    }

    fn verification_evidence(requirement_id: &str, kind: &str) -> crate::trace_core::VerificationEvidence {
        let artifact_ref = local_ref(&format!("traceability:{requirement_id}:{kind}"));
        crate::trace_core::VerificationEvidence {
            target: format!("src/testing/distributed.rs::{requirement_id}:{kind}"),
            command: format!("cargo test --lib {requirement_id}_{kind}"),
            artifact_ref: artifact_ref.clone(),
            artifact_refs: vec![artifact_ref.clone()],
            target_exists: true,
            artifact_present: true,
            source: "verification-run-receipt".to_string(),
            receipt_ref: Some(artifact_ref),
            expected_decision: expected_trace_decision(kind).to_string(),
        }
    }

    fn expected_trace_decision(kind: &str) -> &'static str {
        if kind == "positive" {
            PASS_DECISION
        } else {
            DENY_DECISION
        }
    }

    fn configured_profile(profile_id: &str) -> CiProfile {
        default_ci_profiles()
            .into_iter()
            .find(|profile| profile.id == profile_id)
            .expect("configured distributed CI profile")
    }

    fn metadata_input(profile: &CiProfile) -> TestMetadataInput {
        TestMetadataInput {
            source_ref: local_ref("source-tree"),
            nix_input_refs: vec![local_ref("nix-inputs")],
            test_binary_ref: local_ref("test-binary"),
            profile_id: profile.id.clone(),
            command: profile.command.clone(),
            expected_artifact_kinds: profile.expected_artifact_kinds.clone(),
            cost_class: profile.cost_class.clone(),
            release_review_status: profile.release_review_status.clone(),
            shard_id: format!("{}-shard", profile.id),
            seed_ref: local_ref("seed"),
            topology_ref: local_ref("topology"),
            fault_plan_ref: local_ref("fault-plan"),
            receipt_refs: vec![local_ref(&format!("receipt:{}", profile.id))],
            variance_refs: vec![local_ref("variance:none")],
            diagnostic_log_refs: vec![local_ref(&format!("log:{}", profile.id))],
        }
    }

    fn metadata_from_profile(profile: &CiProfile) -> TestMetadata {
        build_test_metadata(&metadata_input(profile)).expect("metadata")
    }

    fn metadata(profile_id: &str) -> TestMetadata {
        metadata_from_profile(&configured_profile(profile_id))
    }

    #[test]
    fn simulation_traceability_manifest_names_positive_and_negative_fixture_evidence() {
        // r[verify molten.testing.distributed_simulation.fixture_traceability]
        let manifest = traceability_manifest();
        let direct_entry = manifest
            .entries
            .iter()
            .find(|entry| entry.requirement_id == "molten.testing.distributed_simulation.direct_fault_fixtures")
            .expect("direct fixture traceability entry");

        assert_eq!(manifest.decision, PASS_DECISION);
        assert!(manifest.summary.covered.iter().any(|item| item == &direct_entry.requirement_id));
        assert_eq!(direct_entry.positive[0].expected_decision, PASS_DECISION);
        assert_eq!(direct_entry.negative[0].expected_decision, DENY_DECISION);
        assert!(direct_entry.positive[0].command.contains("cargo test --lib"));
        assert!(direct_entry.negative[0].command.contains("cargo test --lib"));
        assert!(direct_entry.positive[0].receipt_ref.is_some());
        assert!(direct_entry.negative[0].receipt_ref.is_some());
    }

    fn profile_run(profile_id: &str, metadata_ref: String, traceability_ref: String) -> ProfileRun {
        ProfileRun {
            profile_id: profile_id.to_string(),
            decision: PASS_DECISION.to_string(),
            metadata_ref,
            traceability_ref,
            positive_coverage: true,
            negative_coverage: true,
            retry_attempts: 0,
            unavailable: false,
            unsupported_reason: None,
            variance_declared: true,
            required_for_release: false,
        }
    }

    #[test]
    fn ci_matrix_declares_profiles_and_metadata() {
        // r[verify molten.testing.distributed_ci.profile_matrix]
        // r[verify molten.testing.distributed_ci.metadata_binding]
        // r[verify molten.testing.distributed_ci.profile_wiring_evidence]
        let matrix = build_ci_matrix(default_ci_profiles()).expect("matrix");
        let configured_protocol = configured_profile(PROFILE_PROTOCOL);
        let protocol_metadata = metadata_from_profile(&configured_protocol);
        let rendered = crate::preserves_rail::to_text(&matrix.value).expect("render matrix");
        let rendered_metadata = crate::preserves_rail::to_text(&protocol_metadata.value).expect("render metadata");

        assert_eq!(matrix.decision, PASS_DECISION);
        assert_eq!(matrix.profiles.len(), REQUIRED_DISTRIBUTED_PROFILE_COUNT);
        assert!(rendered.contains("vm-fault"));
        assert!(rendered.contains("nix build .#checks.x86_64-linux.nixos-vm-multinode"));
        assert_eq!(protocol_metadata.profile_id, configured_protocol.id);
        assert_eq!(protocol_metadata.command, configured_protocol.command);
        assert_eq!(protocol_metadata.expected_artifact_kinds, configured_protocol.expected_artifact_kinds);
        assert_eq!(protocol_metadata.cost_class, configured_protocol.cost_class);
        assert_eq!(protocol_metadata.release_review_status, configured_protocol.release_review_status);
        assert!(rendered_metadata.contains("diagnostic-logs"));
        assert!(rendered_metadata.contains("artifact-kinds"));
        assert!(rendered_metadata.contains("release-review-status"));
    }

    #[test]
    fn ci_profile_wiring_gate_accepts_configured_matrix_metadata() {
        // r[verify molten.testing.distributed_ci.profile_wiring_evidence]
        let profiles = default_ci_profiles();
        let matrix = build_ci_matrix(profiles.clone()).expect("matrix");
        let traceability = traceability_manifest();
        let metadata = profiles.iter().map(metadata_from_profile).collect::<Vec<_>>();
        let runs = metadata
            .iter()
            .map(|item| profile_run(&item.profile_id, item.metadata_ref.clone(), traceability.manifest_ref.clone()))
            .collect::<Vec<_>>();
        let gate = evaluate_ci_gate(&CiGateInput {
            matrix: &matrix,
            metadata: &metadata,
            traceability_manifest: &traceability,
            runs: &runs,
        })
        .expect("gate");

        assert_eq!(gate.decision, PASS_DECISION);
        assert!(gate.diagnostics.is_empty());
        assert_eq!(gate.metadata_refs.len(), metadata.len());
        for (profile, item) in profiles.iter().zip(metadata.iter()) {
            assert_eq!(item.profile_id, profile.id);
            assert_eq!(item.command, profile.command);
            assert_eq!(item.expected_artifact_kinds, profile.expected_artifact_kinds);
            assert_eq!(item.cost_class, profile.cost_class);
            assert_eq!(item.release_review_status, profile.release_review_status);
        }
    }

    #[test]
    fn ci_profile_wiring_negative_fixtures_report_errors() {
        // r[verify molten.testing.distributed_ci.profile_wiring_evidence]
        let matrix = build_ci_matrix(default_ci_profiles()).expect("matrix");
        let traceability = traceability_manifest();
        let mut protocol_metadata = metadata(PROFILE_PROTOCOL);
        protocol_metadata.command = "cargo test --lib wrong-surface".to_string();
        let mut protocol_run =
            profile_run(PROFILE_PROTOCOL, protocol_metadata.metadata_ref.clone(), traceability.manifest_ref.clone());
        protocol_run.retry_attempts = 1;
        protocol_run.variance_declared = false;
        let vm_metadata = metadata(PROFILE_VM_FAULT);
        let mut vm_run =
            profile_run(PROFILE_VM_FAULT, vm_metadata.metadata_ref.clone(), traceability.manifest_ref.clone());
        vm_run.unavailable = true;
        vm_run.required_for_release = true;
        vm_run.unsupported_reason = Some("no-kvm".to_string());
        let gate = evaluate_ci_gate(&CiGateInput {
            matrix: &matrix,
            metadata: &[protocol_metadata, vm_metadata],
            traceability_manifest: &traceability,
            runs: &[protocol_run, vm_run],
        })
        .expect("gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "metadata-command-mismatch:protocol"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "retry-only-success-denied:protocol"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "undeclared-variance:protocol"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "unavailable-profile-cannot-pass:vm-fault"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "required-profile-unavailable:vm-fault"));
    }
