    use super::*;

    const REQUIREMENT_ID: &str = "molten.testing.trace.fixture";
    const NEGATIVE_ID: &str = "molten.testing.trace.negative";
    const PROPERTY_CASES: u64 = 16;
    const PROPERTY_SALT_MAX: u64 = 1_000_000;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn requirement(id: &str, kind: &str, changed: bool) -> RequirementInput {
        RequirementInput {
            id: id.to_string(),
            source: format!("cairn/specs/testing-harness/spec.md#{id}"),
            kind: kind.to_string(),
            changed,
        }
    }

    fn evidence(label: &str) -> VerificationEvidence {
        compatibility_evidence(format!("tests/{label}.rs"), format!("cargo test {label}"), local_ref(label), true)
    }

    fn receipt_input(requirement_id: &str, coverage_kind: &str, label: &str, exit_status: i64) -> VerificationRunInput {
        VerificationRunInput {
            requirement_id: requirement_id.to_string(),
            coverage_kind: coverage_kind.to_string(),
            target: format!("tests/{label}.rs"),
            argv: vec!["cargo".to_string(), "test".to_string(), label.to_string()],
            profile_ref: local_ref("profile"),
            toolchain_refs: vec![local_ref("toolchain")],
            exit_status,
            stdout_ref: local_ref(&format!("{label}-stdout")),
            stderr_ref: local_ref(&format!("{label}-stderr")),
            artifact_refs: vec![local_ref(&format!("{label}-artifact"))],
        }
    }

    fn proof_obligation(id: &str, class: &str, subject_ref: &str, decision: &str) -> ProofObligationInput {
        ProofObligationInput {
            id: id.to_string(),
            class: class.to_string(),
            subject_ref: subject_ref.to_string(),
            prerequisite_refs: vec![local_ref(&format!("{id}-pre"))],
            receipt_refs: vec![local_ref(&format!("{id}-receipt"))],
            decision: decision.to_string(),
            requirement_ids: vec![REQUIREMENT_ID.to_string()],
            coverage_kind: Some(if decision == "pass" { "positive" } else { "negative" }.to_string()),
            caveats: vec!["evidence-only".to_string()],
        }
    }

    fn proof_layer(id: &str, role: &str, subject_ref: &str, child_ids: Vec<String>) -> ProofLayerInput {
        ProofLayerInput {
            id: id.to_string(),
            role: role.to_string(),
            subject_ref: subject_ref.to_string(),
            decision: if role == "operator-readback" { "deny" } else { "pass" }.to_string(),
            child_ids,
            evidence_refs: vec![local_ref(&format!("{id}-evidence"))],
            caveats: vec!["evidence-only".to_string()],
        }
    }

    fn deny_case(class: &str) -> DenyPathCaseInput {
        let state_ref = local_ref("same-state");
        DenyPathCaseInput {
            class: class.to_string(),
            fixture_ref: local_ref(&format!("{class}-fixture")),
            expected_decision: "deny".to_string(),
            before_state_ref: Some(state_ref.clone()),
            after_state_ref: Some(state_ref),
            no_mutation_ref: Some(local_ref(&format!("{class}-no-mutation"))),
        }
    }

    #[test]
    fn complete_positive_and_negative_coverage_passes() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                positive: vec![evidence("positive")],
                negative: vec![evidence("negative")],
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "pass");
        assert_eq!(manifest.summary.covered, vec![REQUIREMENT_ID.to_string()]);
        assert_eq!(manifest.summary.compatibility_only, vec![REQUIREMENT_ID.to_string()]);
        let gate = traceability_gate_value(&manifest).expect("traceability gate");
        assert!(
            crate::preserves_rail::to_text(&gate)
                .expect("render gate")
                .contains("requirement-traceability-gate-v1")
        );
    }

    #[test]
    fn missing_negative_coverage_denies_changed_requirement() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(NEGATIVE_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: NEGATIVE_ID.to_string(),
                positive: vec![evidence("positive-only")],
                negative: Vec::new(),
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "deny");
        assert_eq!(manifest.summary.missing_negative, vec![NEGATIVE_ID.to_string()]);
    }

    #[test]
    fn stale_requirement_id_is_reported() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: "molten.testing.trace.deleted".to_string(),
                positive: vec![evidence("stale-positive")],
                negative: vec![evidence("stale-negative")],
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "deny");
        assert!(manifest.summary.stale_reference.iter().any(|id| id == "molten.testing.trace.deleted"));
    }

    #[test]
    fn missing_artifact_ref_is_stale() {
        let mut bad = evidence("missing-artifact");
        bad.artifact_present = false;
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                positive: vec![evidence("positive")],
                negative: vec![bad],
                exemption: None,
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "deny");
        assert_eq!(manifest.summary.stale_reference, vec![REQUIREMENT_ID.to_string()]);
    }

    #[test]
    fn documentation_requirement_can_be_exempted() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "documentation", true)],
            coverage: vec![CoverageInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                positive: Vec::new(),
                negative: Vec::new(),
                exemption: Some(CoverageExemption {
                    class: "documentation-only".to_string(),
                    evidence: "README.md#Testing".to_string(),
                }),
            }],
            require_receipt_backed: false,
        })
        .expect("traceability manifest");
        assert_eq!(manifest.decision, "pass");
        assert_eq!(manifest.summary.exempt, vec![REQUIREMENT_ID.to_string()]);
    }

    #[test]
    fn requirement_ids_are_extracted_from_markdown_sources() {
        let requirements = requirements_from_sources(&[SpecSource {
            source: "cairn/specs/testing-harness/spec.md".to_string(),
            markdown: "r[molten.testing.trace.fixture] text\nr[molten.testing.trace.negative] text".to_string(),
            changed: false,
            default_kind: "evidence".to_string(),
        }])
        .expect("extract requirements");
        assert_eq!(requirements.len(), [REQUIREMENT_ID, NEGATIVE_ID].len());
        assert!(requirements.iter().any(|requirement| requirement.id == REQUIREMENT_ID));
        assert!(requirements.iter().any(|requirement| requirement.id == NEGATIVE_ID));
    }

