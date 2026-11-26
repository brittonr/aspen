    use super::*;

    const REQUIREMENT_ID: &str = "molten.testing.hardening.fixture";
    const SECOND_REQUIREMENT_ID: &str = "molten.testing.hardening.negative";
    const PASS_COUNT: u64 = 12;
    const FAIL_COUNT: u64 = 1;
    const SKIP_COUNT: u64 = 0;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn requirement(id: &str, changed: bool) -> crate::requirement_traceability::RequirementInput {
        crate::requirement_traceability::RequirementInput {
            id: id.to_string(),
            source: "cairn/specs/testing-harness/spec.md".to_string(),
            kind: "evidence".to_string(),
            changed,
        }
    }

    fn matrix_entry(id: &str, kind: &str) -> EvidenceMatrixEntryInput {
        EvidenceMatrixEntryInput {
            requirement_id: id.to_string(),
            coverage_kind: kind.to_string(),
            evidence_scope: "cli".to_string(),
            target: format!("tests/{kind}.rs"),
            command: format!("cargo test {kind}"),
            artifact_refs: vec![local_ref(&format!("{id}-{kind}"))],
            receipt_ref: Some(local_ref(&format!("{id}-{kind}-receipt"))),
            caveats: vec!["evidence-only".to_string()],
        }
    }

    fn all_tamper_cases(family: &str) -> Vec<TamperCaseInput> {
        required_tamper_mutations()
            .iter()
            .map(|mutation| TamperCaseInput {
                family: family.to_string(),
                mutation: (*mutation).to_string(),
                fixture_ref: local_ref(&format!("{family}-{mutation}")),
                expected_diagnostic: format!("deny-{mutation}"),
                decision: DECISION_DENY.to_string(),
                pass_evidence_ref: None,
            })
            .collect()
    }

    fn replay_run(role: &str, label: &str) -> ReplaySmokeRunInput {
        ReplaySmokeRunInput {
            role: role.to_string(),
            report_ref: local_ref(&format!("{label}-report")),
            final_state_ref: local_ref(&format!("{label}-state")),
            effect_log_ref: local_ref(&format!("{label}-effects")),
            trace_ref: local_ref(&format!("{label}-trace")),
            diagnostics: Vec::new(),
        }
    }

    fn semantic_profile(id: &str, scope: &str, cost: &str) -> SemanticProfileInput {
        let mut profile = reviewed_nextest_profile_rows()
            .into_iter()
            .find(|profile| profile.profile_id == id)
            .unwrap_or_else(|| {
                semantic_profile_row(ProfileRowInput {
                    profile_id: id,
                    evidence_scope: scope,
                    filter_expression: "package(molten) & test(/fixture/)",
                    retry_policy: "zero-retry",
                    expected_junit_path: "target/nextest/fixture/junit.xml",
                    cost_class: cost,
                    caveats: &["evidence-only"],
                    excluded_partitions: &[],
                    platform_required: false,
                    platform_available: true,
                })
            });
        profile.evidence_scope = scope.to_string();
        profile.cost_class = cost.to_string();
        profile
    }

    // r[verify molten.testing.boundary_coverage.gate]
    // r[verify molten.testing.boundary_coverage.positive_negative]
    // r[verify molten.testing.boundary_coverage.exemptions]
    // r[verify molten.testing.evidence_matrix.checked_in_manifest]
    // r[verify molten.testing.evidence_matrix.changed_requirement_gate]
    // r[verify molten.testing.evidence_matrix.receipt_backed_entries]
    // r[verify molten.testing.evidence_matrix.exemptions]
    // r[verify molten.testing.ci_run_receipt.canonical_receipt]
    // r[verify molten.testing.ci_run_receipt.junit_view_only]
    // r[verify molten.testing.ci_run_receipt.nix_nextest_binding]
    // r[verify molten.testing.ci_run_receipt.deny_on_missing_metadata]
    // r[verify molten.testing.tamper_matrix.generated_cases]
    // r[verify molten.testing.tamper_matrix.coverage]
    // r[verify molten.testing.tamper_matrix.fail_closed]
    // r[verify molten.testing.hegel_counterexample.replay_fixture]
    // r[verify molten.testing.hegel_counterexample.promotion]
    // r[verify molten.testing.hegel_counterexample.redaction]
    // r[verify molten.testing.replay_smoke.all_evidence_suites]
    // r[verify molten.testing.replay_smoke.fresh_rerun]
    // r[verify molten.testing.replay_smoke.non_replayable_excluded]
    // r[verify molten.testing.nextest_profiles.semantic_partitions]
    // r[verify molten.testing.nextest_profiles.config_readback]
    // r[verify molten.testing.nextest_profiles.deterministic_exclusion]
    // r[verify molten.testing.nextest_profiles.positive_negative_coverage]
    // r[verify molten.testing.cli_receipt_first.normative_artifacts]
    // r[verify molten.testing.cli_receipt_first.stdout_diagnostic_only]
    // r[verify molten.testing.cli_receipt_first.negative_fail_closed]
    #[test]
    fn boundary_coverage_gate_passes_complete_positive_and_negative_boundaries() {
        let input = BoundaryCoverageGateInput {
            report_ref: local_ref("report"),
            suite_ref: local_ref("suite"),
            required: vec![
                BoundaryRequirementInput {
                    class: "policy-pass".to_string(),
                    polarity: "positive".to_string(),
                    requirement_id: REQUIREMENT_ID.to_string(),
                },
                BoundaryRequirementInput {
                    class: "policy-denial".to_string(),
                    polarity: "negative".to_string(),
                    requirement_id: REQUIREMENT_ID.to_string(),
                },
            ],
            observed: vec![
                BoundaryObservationInput {
                    class: "policy-pass".to_string(),
                    polarity: "positive".to_string(),
                    requirement_id: REQUIREMENT_ID.to_string(),
                    evidence_ref: local_ref("policy-pass"),
                },
                BoundaryObservationInput {
                    class: "policy-denial".to_string(),
                    polarity: "negative".to_string(),
                    requirement_id: REQUIREMENT_ID.to_string(),
                    evidence_ref: local_ref("policy-denial"),
                },
            ],
            exemptions: Vec::new(),
        };
        let gate = build_boundary_coverage_gate(&input).expect("boundary gate");
        assert_eq!(gate.decision, DECISION_PASS);
        assert!(gate.missing_classes.is_empty());
    }

    #[test]
    fn boundary_coverage_gate_denies_missing_denial_and_bad_exemption() {
        let input = BoundaryCoverageGateInput {
            report_ref: local_ref("report"),
            suite_ref: local_ref("suite"),
            required: vec![BoundaryRequirementInput {
                class: "policy-denial".to_string(),
                polarity: "negative".to_string(),
                requirement_id: REQUIREMENT_ID.to_string(),
            }],
            observed: vec![BoundaryObservationInput {
                class: "unsupported".to_string(),
                polarity: "positive".to_string(),
                requirement_id: REQUIREMENT_ID.to_string(),
                evidence_ref: "not-a-ref".to_string(),
            }],
            exemptions: vec![BoundaryCoverageExemptionInput {
                class: "policy-denial".to_string(),
                reason: "vm-unavailable".to_string(),
                evidence_ref: "missing".to_string(),
                scope: "local".to_string(),
                caveat: "pass".to_string(),
            }],
        };
        let gate = build_boundary_coverage_gate(&input).expect("boundary gate");
        assert_eq!(gate.decision, DECISION_DENY);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic.contains("unsupported-boundary-class")));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic.contains("exemption-without-evidence")));
    }

    #[test]
    fn evidence_matrix_manifest_accepts_positive_and_negative_entries() {
        let entries = vec![
            matrix_entry(REQUIREMENT_ID, "positive"),
            matrix_entry(REQUIREMENT_ID, "negative"),
        ];
        let manifest = build_evidence_matrix_manifest(&EvidenceMatrixInput {
            requirements: vec![requirement(REQUIREMENT_ID, true)],
            entries: entries.clone(),
            exemptions: Vec::new(),
        })
        .expect("evidence matrix");
        assert_eq!(manifest.decision, DECISION_PASS);
        assert!(manifest.missing_positive.is_empty());
        assert!(manifest.missing_negative.is_empty());
        assert_checked_in_row("evidence_matrix_manifest_accepts_positive_and_negative_entries", &entries, &manifest);
    }

    /// The checked-in `tests/evidence-matrix.ncl` row for this test binds the
    /// manifest this test actually computes and the input artifact refs.
    fn assert_checked_in_row(test: &str, entries: &[EvidenceMatrixEntryInput], manifest: &EvidenceMatrixManifest) {
        let row = crate::capability_qualification::matrix::entry(&format!("hardening_core::tests::{test} "));
        let mut artifact_refs: Vec<_> = entries.iter().flat_map(|entry| entry.artifact_refs.clone()).collect();
        artifact_refs.sort();
        assert_eq!(row.artifact_refs, artifact_refs, "matrix artifact refs for {test}");
        assert_eq!(row.receipt_ref, manifest.manifest_ref, "matrix receipt ref for {test}");
    }

    // r[verify molten.testing.evidence_matrix_native_producers]
    #[test]
    fn evidence_matrix_manifest_denies_placeholder_refs() {
        let mut positive = matrix_entry(REQUIREMENT_ID, "positive");
        positive.receipt_ref =
            Some("blake3:0000000000000000000000000000000000000000000000000000000000000102".to_string());
        let mut negative = matrix_entry(REQUIREMENT_ID, "negative");
        negative.artifact_refs =
            vec!["blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()];
        let manifest = build_evidence_matrix_manifest(&EvidenceMatrixInput {
            requirements: vec![requirement(REQUIREMENT_ID, true)],
            entries: vec![positive, negative],
            exemptions: vec![EvidenceMatrixExemptionInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                reason: "reviewed-placeholder-control".to_string(),
                evidence_ref: "blake3:0000000000000000000000000000000000000000000000000000000000000222".to_string(),
                scope: "integration".to_string(),
                review_note: "placeholder exemption evidence must deny".to_string(),
            }],
        })
        .expect("evidence matrix");
        assert_eq!(manifest.decision, DECISION_DENY);
        let placeholders =
            manifest.diagnostics.iter().filter(|diagnostic| diagnostic.starts_with("placeholder-ref:")).count();
        assert_eq!(placeholders, 3);
        assert!(!is_placeholder_ref(&local_ref("actual artifact bytes")));
    }

    // r[verify molten.testing.evidence_matrix_native_producers]
    #[test]
    fn evidence_matrix_manifest_denies_one_command_aliased_across_rows() {
        let aliased = "cargo test --test collaborationnative one_ignored_rail -- --ignored";
        let mut entries = vec![
            matrix_entry(REQUIREMENT_ID, "positive"),
            matrix_entry(REQUIREMENT_ID, "negative"),
            matrix_entry(SECOND_REQUIREMENT_ID, "positive"),
        ];
        for entry in &mut entries {
            entry.command = aliased.to_string();
            entry.target = format!("{}-{}", entry.requirement_id, entry.coverage_kind);
        }
        let manifest = build_evidence_matrix_manifest(&EvidenceMatrixInput {
            requirements: vec![requirement(REQUIREMENT_ID, true), requirement(SECOND_REQUIREMENT_ID, false)],
            entries,
            exemptions: Vec::new(),
        })
        .expect("evidence matrix");
        assert_eq!(manifest.decision, DECISION_DENY);
        assert!(manifest.diagnostics.iter().any(|diagnostic| diagnostic == &format!("aliased-command:{aliased}")));
        // Control: distinct commands for the same rows are not aliasing.
        let distinct = build_evidence_matrix_manifest(&EvidenceMatrixInput {
            requirements: vec![requirement(REQUIREMENT_ID, true)],
            entries: vec![matrix_entry(REQUIREMENT_ID, "positive"), matrix_entry(REQUIREMENT_ID, "negative")],
            exemptions: Vec::new(),
        })
        .expect("evidence matrix");
        assert!(!distinct.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("aliased-command:")));
    }

    #[test]
    fn evidence_matrix_manifest_denies_missing_negative_and_stale_id() {
        let entries = vec![
            matrix_entry(REQUIREMENT_ID, "positive"),
            matrix_entry(SECOND_REQUIREMENT_ID, "negative"),
        ];
        let manifest = build_evidence_matrix_manifest(&EvidenceMatrixInput {
            requirements: vec![requirement(REQUIREMENT_ID, true)],
            entries: entries.clone(),
            exemptions: Vec::new(),
        })
        .expect("evidence matrix");
        assert_eq!(manifest.decision, DECISION_DENY);
        assert_eq!(manifest.missing_negative, vec![REQUIREMENT_ID.to_string()]);
        assert!(manifest.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale-requirement-id")));
        assert_checked_in_row("evidence_matrix_manifest_denies_missing_negative_and_stale_id", &entries, &manifest);
    }
