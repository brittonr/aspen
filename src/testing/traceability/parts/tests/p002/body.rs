    #[test]
    fn proof_readback_groups_requirement_evidence_and_caveats() {
        let positive = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "positive", "positive", 0))
            .expect("positive receipt");
        let negative = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "negative", "negative", 1))
            .expect("negative receipt");
        let coverage = coverage_from_verification_receipts(&[
            ReceiptCoverageSource {
                value: positive.value,
                target_exists: true,
            },
            ReceiptCoverageSource {
                value: negative.value,
                target_exists: true,
            },
        ])
        .expect("coverage");
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage,
            require_receipt_backed: true,
        })
        .expect("manifest");
        let readback = build_proof_readback(&manifest).expect("readback");
        let rendered = render_proof_readback(&readback).expect("rendered readback");
        assert!(rendered.contains("readback is non-normative"));
        assert!(rendered.contains(REQUIREMENT_ID));
    }

    #[hegel::test(test_cases = PROPERTY_CASES)]
    fn hegel_traceability_decision_law_and_deny_monotonicity(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_SALT_MAX));
        let id = format!("molten.testing.trace.generated.{salt}");
        let positive = build_verification_run_receipt(&receipt_input(&id, "positive", "generated-positive", 0))
            .expect("positive receipt");
        let negative = build_verification_run_receipt(&receipt_input(&id, "negative", "generated-negative", 1))
            .expect("negative receipt");
        let coverage = coverage_from_verification_receipts(&[
            ReceiptCoverageSource {
                value: positive.value,
                target_exists: true,
            },
            ReceiptCoverageSource {
                value: negative.value,
                target_exists: true,
            },
        ])
        .expect("coverage");
        let pass = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(&id, "evidence", true)],
            coverage: coverage.clone(),
            require_receipt_backed: true,
        })
        .expect("pass manifest");
        assert_eq!(pass.decision, "pass");

        let mut stale_coverage = coverage;
        stale_coverage.push(CoverageInput {
            requirement_id: format!("molten.testing.trace.generated.stale.{salt}"),
            positive: vec![evidence("stale-positive")],
            negative: vec![evidence("stale-negative")],
            exemption: None,
        });
        let denied = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(&id, "evidence", true)],
            coverage: stale_coverage,
            require_receipt_backed: true,
        })
        .expect("deny manifest");
        assert_eq!(denied.decision, "deny");
        assert!(!denied.summary.stale_reference.is_empty());
    }

    #[hegel::test(test_cases = PROPERTY_CASES)]
    fn hegel_receipt_ref_stability_and_binding_drift(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_SALT_MAX));
        let id = format!("molten.testing.receipt.generated.{salt}");
        let input = receipt_input(&id, "positive", "stable", 0);
        let first = build_verification_run_receipt(&input).expect("first");
        let second = build_verification_run_receipt(&input).expect("second");
        assert_eq!(first.receipt_ref, second.receipt_ref);
        let mut drift = input;
        drift.argv.push(format!("--salt={salt}"));
        let drifted = build_verification_run_receipt(&drift).expect("drifted");
        assert_ne!(first.receipt_ref, drifted.receipt_ref);
    }

    #[hegel::test(test_cases = PROPERTY_CASES)]
    fn hegel_layer_ordering_and_wrong_scope_denial(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_SALT_MAX));
        let subject = local_ref(&format!("subject-{salt}"));
        let other_subject = local_ref(&format!("other-subject-{salt}"));
        let mut core = proof_layer("core", "pure-core", &subject, Vec::new());
        if tc.draw(hegel::generators::booleans()) {
            core.subject_ref = other_subject;
        }
        let manifest = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject.clone(),
            layers: vec![proof_layer("gate", "gate", &subject, vec!["core".to_string()]), core],
        })
        .expect("layered manifest");
        if manifest.layers.iter().any(|layer| layer.subject_ref != subject) {
            assert_eq!(manifest.decision, "deny");
        }
        let rerender = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject.clone(),
            layers: manifest.layers.clone(),
        })
        .expect("rerender layered manifest");
        assert_eq!(manifest.manifest_ref, rerender.manifest_ref);
    }
