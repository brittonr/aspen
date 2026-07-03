    use super::*;
    const OBSERVED_QUEUE_DEPTH: u64 = 2;
    const MAX_QUEUE_DEPTH: u64 = 8;
    const OVER_LIMIT_QUEUE_DEPTH: u64 = 13;

    fn reference(label: &str) -> String {
        canonical_hash(&record("prod-readiness-ref", vec![string(label)])).expect("synthetic ref")
    }

    fn refs(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|label| reference(label)).collect()
    }

    fn texts(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|label| (*label).to_string()).collect()
    }

    #[test]
    fn production_ops_receipts_bind_positive_and_denial_evidence() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["operator reviewed"]);
        let profile_ref = reference("profile-export");
        let profile = deployment_profile_value(&DeploymentProfileInput {
            decision: "pass",
            profile_name: "pilot-node",
            schema_id: PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA,
            schema_version: PRODUCTION_PROFILE_SCHEMA_VERSION,
            source_language: PRODUCTION_PROFILE_SOURCE_LANGUAGE,
            profile_identity: "pilot-node",
            profile_ref: &profile_ref,
            state_layout_refs: &base_refs,
            required_adapter_refs: &base_refs,
            source_gate_refs: &base_refs,
            resource_limit_refs: &base_refs,
            redaction_setting_refs: &base_refs,
            live_transport_refs: &base_refs,
            startup_expectation_refs: &base_refs,
            shutdown_expectation_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("deployment profile");
        let backup = backup_restore_drill_value(&BackupRestoreDrillInput {
            decision: "pass",
            drill_name: "backup-restore",
            ledger_refs: &base_refs,
            redb_refs: &base_refs,
            chunk_refs: &base_refs,
            identity_refs: &base_refs,
            retention_pin_refs: &base_refs,
            source_gate_refs: &base_refs,
            restore_verification_refs: &base_refs,
            tamper_denial_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("backup restore");
        let runbook = runbook_check_value(&RunbookCheckInput {
            decision: "pass",
            runbook_name: "startup",
            operation: "init",
            canonical_artifact_refs: &base_refs,
            denial_fixture_refs: &base_refs,
            auxiliary_log_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("runbook check");
        let profile_text = to_text(&profile).expect("profile text");
        let backup_text = to_text(&backup).expect("backup text");
        let runbook_text = to_text(&runbook).expect("runbook text");
        assert!(profile_text.contains("prod-ops-deployment-profile-v1"));
        assert!(backup_text.contains("tampered-backup-denies-restore"));
        assert!(runbook_text.contains("canonical-receipts-not-terminal-output"));
    }

    #[test]
    fn production_ops_pass_denies_missing_or_degraded_evidence() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["queue pressure"]);
        let profile_ref = reference("profile-export");
        let missing_profile = deployment_profile_value(&DeploymentProfileInput {
            decision: "pass",
            profile_name: "pilot-node",
            schema_id: PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA,
            schema_version: PRODUCTION_PROFILE_SCHEMA_VERSION,
            source_language: PRODUCTION_PROFILE_SOURCE_LANGUAGE,
            profile_identity: "pilot-node",
            profile_ref: &profile_ref,
            state_layout_refs: &[],
            required_adapter_refs: &base_refs,
            source_gate_refs: &base_refs,
            resource_limit_refs: &base_refs,
            redaction_setting_refs: &base_refs,
            live_transport_refs: &base_refs,
            startup_expectation_refs: &base_refs,
            shutdown_expectation_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        let degraded_pass = observability_slo_value(&ObservabilitySloInput {
            decision: "pass",
            snapshot_name: "over-limit",
            adapter_health_refs: &base_refs,
            queue_depth: OVER_LIMIT_QUEUE_DEPTH,
            max_queue_depth: MAX_QUEUE_DEPTH,
            control_loop_refs: &base_refs,
            resource_pressure_refs: &base_refs,
            retention_drift_refs: &base_refs,
            source_gate_freshness_refs: &base_refs,
            live_transport_refs: &base_refs,
            import_export_failure_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        let mismatched_metadata = deployment_profile_value(&DeploymentProfileInput {
            decision: "pass",
            profile_name: "pilot-node",
            schema_id: PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA,
            schema_version: PRODUCTION_PROFILE_SCHEMA_VERSION,
            source_language: PRODUCTION_PROFILE_SOURCE_LANGUAGE,
            profile_identity: "other-node",
            profile_ref: &profile_ref,
            state_layout_refs: &base_refs,
            required_adapter_refs: &base_refs,
            source_gate_refs: &base_refs,
            resource_limit_refs: &base_refs,
            redaction_setting_refs: &base_refs,
            live_transport_refs: &base_refs,
            startup_expectation_refs: &base_refs,
            shutdown_expectation_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        let unsupported_metadata = deployment_profile_value(&DeploymentProfileInput {
            decision: "pass",
            profile_name: "pilot-node",
            schema_id: PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA,
            schema_version: PRODUCTION_PROFILE_SCHEMA_VERSION + 1,
            source_language: PRODUCTION_PROFILE_SOURCE_LANGUAGE,
            profile_identity: "pilot-node",
            profile_ref: &profile_ref,
            state_layout_refs: &base_refs,
            required_adapter_refs: &base_refs,
            source_gate_refs: &base_refs,
            resource_limit_refs: &base_refs,
            redaction_setting_refs: &base_refs,
            live_transport_refs: &base_refs,
            startup_expectation_refs: &base_refs,
            shutdown_expectation_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        let tampered_profile_ref = deployment_profile_value(&DeploymentProfileInput {
            decision: "pass",
            profile_name: "pilot-node",
            schema_id: PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA,
            schema_version: PRODUCTION_PROFILE_SCHEMA_VERSION,
            source_language: PRODUCTION_PROFILE_SOURCE_LANGUAGE,
            profile_identity: "pilot-node",
            profile_ref: "not-a-content-ref",
            state_layout_refs: &base_refs,
            required_adapter_refs: &base_refs,
            source_gate_refs: &base_refs,
            resource_limit_refs: &base_refs,
            redaction_setting_refs: &base_refs,
            live_transport_refs: &base_refs,
            startup_expectation_refs: &base_refs,
            shutdown_expectation_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        assert!(missing_profile.is_err());
        assert!(degraded_pass.is_err());
        assert!(mismatched_metadata.is_err());
        assert!(unsupported_metadata.is_err());
        assert!(tampered_profile_ref.is_err());
        observability_slo_value(&ObservabilitySloInput {
            decision: "degraded",
            snapshot_name: "over-limit",
            adapter_health_refs: &base_refs,
            queue_depth: OVER_LIMIT_QUEUE_DEPTH,
            max_queue_depth: MAX_QUEUE_DEPTH,
            control_loop_refs: &base_refs,
            resource_pressure_refs: &base_refs,
            retention_drift_refs: &base_refs,
            source_gate_freshness_refs: &base_refs,
            live_transport_refs: &base_refs,
            import_export_failure_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("degraded snapshot can be emitted");
    }

    #[test]
    fn security_readiness_receipts_require_mapped_drills_and_denials() {
        let base_refs = refs(&["base"]);
        let threats = texts(&["leaked live ticket"]);
        let diagnostics = texts(&["pilot only"]);
        let threat = threat_model_value(&ThreatModelInput {
            decision: "pass",
            model_name: "pilot-threat-model",
            threat_entries: &threats,
            mapped_gate_refs: &base_refs,
            drill_refs: &base_refs,
            negative_suite_refs: &base_refs,
            unresolved_risk_refs: &base_refs,
            pilot_consequence_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("threat model");
        let drill = security_drill_value(&SecurityDrillInput {
            decision: "pass",
            drill_kind: "stale-ticket-denial",
            scenario: "stale live ticket",
            pass_evidence_refs: &base_refs,
            denial_refs: &base_refs,
            cleanup_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("security drill");
        let report = security_readiness_report_value(&SecurityReadinessReportInput {
            decision: "pass",
            report_name: "pilot-security",
            threat_model_refs: &refs(&["threat"]),
            supply_chain_refs: &base_refs,
            drill_refs: &refs(&["drill"]),
            redaction_audit_refs: &base_refs,
            boundary_suite_refs: &base_refs,
            incident_response_refs: &base_refs,
            unresolved_risk_refs: &base_refs,
            pilot_recommendation: "limited-internal-pilot",
            diagnostics: &diagnostics,
        })
        .expect("security readiness report");
        assert!(to_text(&threat).expect("threat text").contains("gate-drill-or-risk-mapped"));
        assert!(to_text(&drill).expect("drill text").contains("stale-ticket-denial"));
        assert!(to_text(&report).expect("report text").contains("pilot-scope-recommendation-explicit"));
    }

    #[test]
    fn security_readiness_denies_unmapped_or_broad_unresolved_risk() {
        let base_refs = refs(&["base"]);
        let threats = texts(&["unmapped threat"]);
        let diagnostics = texts(&["risk remains"]);
        let unmapped = threat_model_value(&ThreatModelInput {
            decision: "pass",
            model_name: "bad-threat-model",
            threat_entries: &threats,
            mapped_gate_refs: &[],
            drill_refs: &[],
            negative_suite_refs: &[],
            unresolved_risk_refs: &[],
            pilot_consequence_refs: &[],
            diagnostics: &diagnostics,
        });
        let broad = security_readiness_report_value(&SecurityReadinessReportInput {
            decision: "pass",
            report_name: "bad-security",
            threat_model_refs: &base_refs,
            supply_chain_refs: &base_refs,
            drill_refs: &base_refs,
            redaction_audit_refs: &base_refs,
            boundary_suite_refs: &base_refs,
            incident_response_refs: &base_refs,
            unresolved_risk_refs: &base_refs,
            pilot_recommendation: BROAD_PRODUCTION_SCOPE,
            diagnostics: &diagnostics,
        });
        assert!(unmapped.is_err());
        assert!(broad.is_err());
    }

    #[test]
    fn release_candidate_binds_matrix_and_scoped_pilot() {
        let base_refs = refs(&["base"]);
        let caveats = texts(&["Octet disabled lint family burn-down remains"]);
        let diagnostics = texts(&["candidate reviewed"]);
        let pilot = pilot_decision_value(&PilotDecisionInput {
            decision: "pass",
            scope: "limited-internal-pilot",
            allowed_workloads: &texts(&["stateless internal jobs"]),
            denied_workloads: &texts(&["customer-critical destructive retention"]),
            rollback_triggers: &texts(&["stale source gate"]),
            stop_conditions: &texts(&["failed dogfood replay"]),
            operator_review_refs: &base_refs,
            caveats: &caveats,
            diagnostics: &diagnostics,
        })
        .expect("pilot decision");
        let pilot_refs = vec![canonical_hash(&pilot).expect("pilot ref")];
        let candidate = release_candidate_gate_value(&ReleaseCandidateGateInput {
            decision: "pass",
            candidate: "aspen-molten-pilot",
            source_ref: &reference("source"),
            rust_validation_refs: &base_refs,
            nextest_refs: &base_refs,
            nix_check_refs: &base_refs,
            cairn_validation_refs: &base_refs,
            octet_refs: &base_refs,
            dogfood_refs: &base_refs,
            bundle_verify_refs: &base_refs,
            promotion_refs: &base_refs,
            export_verify_refs: &base_refs,
            source_gate_status: CONFIGURATION_CLEAN_CAVEAT_STATUS,
            source_gate_caveats: &caveats,
            pilot_decision_refs: &pilot_refs,
            diagnostics: &diagnostics,
        })
        .expect("release candidate gate");
        let candidate_text = to_text(&candidate).expect("candidate text");
        assert!(candidate_text.contains("prod-release-candidate-gate-v1"));
        assert!(candidate_text.contains("source-gate-current-or-limited"));
    }

    #[test]
    fn release_candidate_denies_broad_caveat_or_missing_matrix() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["candidate reviewed"]);
        let broad = pilot_decision_value(&PilotDecisionInput {
            decision: "pass",
            scope: BROAD_PRODUCTION_SCOPE,
            allowed_workloads: &texts(&["all workloads"]),
            denied_workloads: &texts(&["none"]),
            rollback_triggers: &texts(&["none"]),
            stop_conditions: &texts(&["none"]),
            operator_review_refs: &base_refs,
            caveats: &texts(&["source caveat"]),
            diagnostics: &diagnostics,
        });
        let missing_source_caveat = release_candidate_gate_value(&ReleaseCandidateGateInput {
            decision: "pass",
            candidate: "bad-candidate",
            source_ref: &reference("source"),
            rust_validation_refs: &base_refs,
            nextest_refs: &base_refs,
            nix_check_refs: &base_refs,
            cairn_validation_refs: &base_refs,
            octet_refs: &base_refs,
            dogfood_refs: &base_refs,
            bundle_verify_refs: &base_refs,
            promotion_refs: &base_refs,
            export_verify_refs: &base_refs,
            source_gate_status: CONFIGURATION_CLEAN_CAVEAT_STATUS,
            source_gate_caveats: &[],
            pilot_decision_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        assert!(broad.is_err());
        assert!(missing_source_caveat.is_err());
    }
