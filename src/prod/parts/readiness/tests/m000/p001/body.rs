
    #[test]
    fn incident_boundary_redaction_and_supply_chain_positive_paths_emit_receipts() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["reviewed"]);
        let boundary = boundary_negative_suite_value(&BoundaryNegativeSuiteInput {
            decision: "pass",
            suite_name: "boundary-negative",
            preserves_parser_refs: &base_refs,
            receipt_validator_refs: &base_refs,
            source_gate_refs: &base_refs,
            repro_bundle_refs: &base_refs,
            node_ingress_refs: &base_refs,
            provenance_refs: &base_refs,
            plugin_hostcall_refs: &base_refs,
            malformed_denial_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("boundary suite");
        let redaction = redaction_audit_value(&RedactionAuditInput {
            decision: "pass",
            audit_name: "redaction",
            surface_refs: &base_refs,
            redaction_refs: &base_refs,
            reveal_gate_refs: &base_refs,
            plaintext_denial_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("redaction audit");
        let supply = supply_chain_review_value(&SupplyChainReviewInput {
            decision: "pass",
            review_name: "supply-chain",
            release_refs: &base_refs,
            source_gate_refs: &base_refs,
            provenance_refs: &base_refs,
            build_verify_refs: &base_refs,
            signed_keyring_refs: &base_refs,
            sensitive_artifact_refs: &base_refs,
            mismatch_denial_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("supply chain review");
        let incident = incident_response_drill_value(&IncidentResponseDrillInput {
            decision: "pass",
            incident_kind: "leaked-ticket",
            scenario: "leaked peer ticket",
            detection_refs: &base_refs,
            containment_refs: &base_refs,
            recovery_refs: &base_refs,
            next_step_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("incident response");
        assert!(to_text(&boundary).expect("boundary text").contains("malformed-input-denies"));
        assert!(to_text(&redaction).expect("redaction text").contains("plaintext-secret-export-denied"));
        assert!(to_text(&supply).expect("supply text").contains("stale-sensitive-artifact-denies"));
        assert!(to_text(&incident).expect("incident text").contains("leaked-ticket"));
    }

    #[test]
    fn observability_pass_accepts_in_bound_queue() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["healthy"]);
        let receipt = observability_slo_value(&ObservabilitySloInput {
            decision: "pass",
            snapshot_name: "healthy",
            adapter_health_refs: &base_refs,
            queue_depth: OBSERVED_QUEUE_DEPTH,
            max_queue_depth: MAX_QUEUE_DEPTH,
            control_loop_refs: &base_refs,
            resource_pressure_refs: &base_refs,
            retention_drift_refs: &base_refs,
            source_gate_freshness_refs: &base_refs,
            live_transport_refs: &base_refs,
            import_export_failure_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("observability receipt");
        assert!(to_text(&receipt).expect("receipt text").contains("queue-depth-within-slo"));
    }
