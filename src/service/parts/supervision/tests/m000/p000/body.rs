    type TestCase = hegel::TestCase;

    use super::*;

    type ListInput = crate::catalog::ListInput;
    type VisibilityInput = crate::catalog::VisibilityInput;

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("service-supervision-test-ref", vec![
            crate::preserves_rail::string(label),
        ]))
        .expect("test ref")
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn suite_with_attempt(attempt: u64) -> IoValue {
        let mut suite = parse_service_supervision_suite(&supervision_fixture_suite_value().expect("fixture suite"))
            .expect("parse fixture suite");
        suite.restart_attempt = attempt;
        service_supervision_suite_value(&ServiceSupervisionSuiteInput {
            manifest: suite.manifest.value,
            links: suite.links.into_iter().map(|link| link.value).collect(),
            monitors: suite.monitors.into_iter().map(|monitor| monitor.value).collect(),
            restart_policy: suite.restart_policy.value,
            owned_state: suite.owned_state.value,
            restart_attempt: attempt,
            logical_step: 0,
            evidence: suite.evidence,
        })
        .expect("suite with attempt")
    }

    #[test]
    // r[verify molten.sam_service_supervision.spec.supervision]
    // r[verify molten.sam_service_supervision_cleanup.spec.logical_supervision]
    fn failure_notifies_monitors_and_restart_passes() {
        let suite_value = supervision_fixture_suite_value().expect("fixture suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run supervision");
        assert_eq!(run.monitor_notifications.len(), 2);
        assert_eq!(run.scheduled_demands.len(), 1);
        let decision =
            crate::service_records::parse_service_restart_decision(&run.restart_decisions[0]).expect("decision");
        assert_eq!(decision.decision, "pass");
        let lifecycle =
            crate::service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("lifecycle");
        assert_eq!(lifecycle.operation, "fail");
        assert_eq!(lifecycle.supervision_refs.len(), 5);
        replay_service_supervision_report(&run.value).expect("replay supervision report");
        let gate = gate_service_supervision_report(&run.value).expect("gate supervision report");
        assert_eq!(gate.decision, "pass");
        assert_eq!(gate.monitor_count, 2);
        assert_eq!(gate.restart_decision.as_deref(), Some("pass"));
        let receipt = parse_service_supervision_gate_receipt(&gate.value).expect("parse gate receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.report_ref, run.report_ref);
    }

    #[test]
    // r[verify molten.sam_service_supervision_cleanup.spec.bounded_restart]
    fn restart_budget_exhausted_cleans_and_publishes_final_status() {
        let suite_value = suite_with_attempt(2);
        let run = run_service_supervision_suite_value(&suite_value).expect("run supervision");
        assert!(run.scheduled_demands.is_empty());
        assert_eq!(run.cleanup_receipts.len(), 1);
        assert_eq!(run.retractions.len(), 5);
        assert_eq!(run.statuses.len(), 2);
        let decision =
            crate::service_records::parse_service_restart_decision(&run.restart_decisions[0]).expect("decision");
        assert_eq!(decision.decision, "deny");
        assert!(decision.diagnostics.iter().any(|diagnostic| diagnostic.contains("budget")));
    }

    #[test]
    // r[verify molten.sam_service_supervision.spec.cleanup]
    // r[verify molten.sam_service_supervision_cleanup.spec.owned_cleanup]
    // r[verify molten.sam_service_supervision_cleanup.spec.cleanup_replay_retention]
    fn revocation_retracts_owned_state_and_binds_retention() {
        let mut suite = parse_service_supervision_suite(&supervision_fixture_suite_value().expect("fixture suite"))
            .expect("parse fixture suite");
        suite.evidence.revocation_refs = vec![test_ref("revocation")];
        let suite_value = service_supervision_suite_value(&ServiceSupervisionSuiteInput {
            manifest: suite.manifest.value,
            links: suite.links.into_iter().map(|link| link.value).collect(),
            monitors: suite.monitors.into_iter().map(|monitor| monitor.value).collect(),
            restart_policy: suite.restart_policy.value,
            owned_state: suite.owned_state.value,
            restart_attempt: 0,
            logical_step: 0,
            evidence: suite.evidence,
        })
        .expect("revoked suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run revoked supervision");
        assert_eq!(run.cleanup_receipts.len(), 1);
        assert_eq!(run.retention_inputs.len(), 1);
        let cleanup = crate::service_records::parse_service_cleanup_receipt(&run.cleanup_receipts[0]).expect("cleanup");
        assert_eq!(cleanup.decision, "pass");
        assert_eq!(cleanup.revocation_refs.len(), 1);
        assert_eq!(cleanup.retraction_refs.len(), 5);
    }

    #[test]
    fn foreign_state_is_not_deleted() {
        let mut suite = parse_service_supervision_suite(&supervision_fixture_suite_value().expect("fixture suite"))
            .expect("parse fixture suite");
        suite.restart_attempt = 2;
        let mut owned = suite.owned_state.clone();
        owned.foreign_ref_claims = vec![test_ref("foreign")];
        let owned_state = service_owned_state_value(&ServiceOwnedStateInput {
            service_id: owned.service_id,
            manifest_ref: owned.manifest_ref,
            owned_assertion_refs: owned.owned_assertion_refs,
            observer_refs: owned.observer_refs,
            live_ref_refs: owned.live_ref_refs,
            exposed_ref_refs: owned.exposed_ref_refs,
            pending_effect_refs: owned.pending_effect_refs,
            foreign_ref_claims: owned.foreign_ref_claims,
        })
        .expect("owned state with foreign claim");
        let suite_value = service_supervision_suite_value(&ServiceSupervisionSuiteInput {
            manifest: suite.manifest.value,
            links: suite.links.into_iter().map(|link| link.value).collect(),
            monitors: suite.monitors.into_iter().map(|monitor| monitor.value).collect(),
            restart_policy: suite.restart_policy.value,
            owned_state,
            restart_attempt: 2,
            logical_step: 0,
            evidence: suite.evidence,
        })
        .expect("foreign suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run foreign cleanup");
        assert!(run.retractions.is_empty());
        let cleanup = crate::service_records::parse_service_cleanup_receipt(&run.cleanup_receipts[0]).expect("cleanup");
        assert_eq!(cleanup.decision, "deny");
        assert!(cleanup.diagnostics.iter().any(|diagnostic| diagnostic.contains("foreign")));
    }

    #[test]
    fn resource_denial_prevents_restart_and_cleans_owned_state() {
        let mut suite = parse_service_supervision_suite(&supervision_fixture_suite_value().expect("fixture suite"))
            .expect("parse fixture suite");
        suite.evidence.resource_refs.clear();
        let suite_value = service_supervision_suite_value(&ServiceSupervisionSuiteInput {
            manifest: suite.manifest.value,
            links: suite.links.into_iter().map(|link| link.value).collect(),
            monitors: suite.monitors.into_iter().map(|monitor| monitor.value).collect(),
            restart_policy: suite.restart_policy.value,
            owned_state: suite.owned_state.value,
            restart_attempt: 0,
            logical_step: 0,
            evidence: suite.evidence,
        })
        .expect("resource denied suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run resource denied supervision");
        assert!(run.scheduled_demands.is_empty());
        assert_eq!(run.cleanup_receipts.len(), 1);
        let decision =
            crate::service_records::parse_service_restart_decision(&run.restart_decisions[0]).expect("decision");
        assert_eq!(decision.decision, "deny");
        assert!(decision.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource")));
    }

    #[test]
    fn replay_detects_monitor_restart_and_cleanup_divergence() {
        let suite_value = suite_with_attempt(2);
        let run = run_service_supervision_suite_value(&suite_value).expect("run supervision");
        let mut monitor_report = parse_service_supervision_report(&run.value).expect("parse report");
        monitor_report.monitor_notifications.reverse();
        assert!(replay_service_supervision_report(&report_from_parts(&suite_value, &monitor_report)).is_err());

        let mut restart_report = parse_service_supervision_report(&run.value).expect("parse report");
        let decision = crate::service_records::parse_service_restart_decision(&restart_report.restart_decisions[0])
            .expect("decision");
        restart_report.restart_decisions[0] = crate::service_records::service_restart_decision_value(
            &crate::service_records::ServiceRestartDecisionInput {
                decision: decision.decision,
                service_id: decision.service_id,
                manifest_ref: decision.manifest_ref,
                policy_ref: decision.policy_ref,
                attempt: decision.attempt,
                max_attempts: decision.max_attempts,
                window_step: decision.window_step,
                backoff_slot: decision.backoff_slot,
                prior_lifecycle_refs: decision.prior_lifecycle_refs,
                authority_refs: decision.authority_refs,
                resource_refs: decision.resource_refs,
                diagnostics: vec!["tampered restart diagnostic".to_string()],
            },
        )
        .expect("tampered restart decision");
        assert!(replay_service_supervision_report(&report_from_parts(&suite_value, &restart_report)).is_err());

        let mut cleanup_report = parse_service_supervision_report(&run.value).expect("parse report");
        cleanup_report.retractions.pop();
        let tampered = report_from_parts(&suite_value, &cleanup_report);
        assert!(replay_service_supervision_report(&tampered).is_err());
        let gate = gate_service_supervision_report(&tampered).expect("gate tampered report");
        assert_eq!(gate.decision, "deny");
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic.contains("replay failed")));
    }

    fn report_from_parts(suite_value: &IoValue, report: &ServiceSupervisionRun) -> IoValue {
        service_supervision_report_value(ReportValueInput {
            suite_value,
            failure_markers: &report.failure_markers,
            statuses: &report.statuses,
            lifecycle_receipts: &report.lifecycle_receipts,
            monitor_notifications: &report.monitor_notifications,
            restart_decisions: &report.restart_decisions,
            scheduled_demands: &report.scheduled_demands,
            cleanup_receipts: &report.cleanup_receipts,
            retractions: &report.retractions,
            retention_inputs: &report.retention_inputs,
        })
        .expect("report from parts")
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_supervision_artifacts() {
        let suite_value = supervision_fixture_suite_value().expect("fixture suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run supervision");
        let gate = gate_service_supervision_report(&run.value).expect("gate supervision report");
        assert_eq!(crate::ledger::artifact_kind(&suite_value), "service-supervision-suite");
        assert_eq!(crate::ledger::artifact_kind(&run.value), "service-supervision-report");
        assert_eq!(crate::ledger::artifact_kind(&gate.value), "service-supervision-gate-receipt");
        assert_eq!(crate::ledger::artifact_kind(&run.failure_markers[0]), "service-failure");
        assert_eq!(crate::ledger::artifact_kind(&run.monitor_notifications[0]), "service-monitor-notification");

        let denied_suite_value = suite_with_attempt(2);
        let denied = run_service_supervision_suite_value(&denied_suite_value).expect("denied supervision");
        assert_eq!(crate::ledger::artifact_kind(&denied.cleanup_receipts[0]), "service-cleanup-receipt");
        assert_eq!(crate::ledger::artifact_kind(&denied.retractions[0]), "service-retraction");
        assert_eq!(crate::ledger::artifact_kind(&denied.retention_inputs[0]), "service-retention-input");

        let dir = temp_dir("service-supervision-catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let imported =
            crate::ledger::import_artifact(&ledger_root, &run.value).expect("ledger import supervision report");
        assert_eq!(imported.artifact_kind, "service-supervision-report");
        let listed = crate::catalog::list(&registry, Some(&ledger_root), &ListInput {
            kind: Some("service-supervision-report".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("catalog list supervision report");
        assert_eq!(listed.items.len(), 1);
        assert!(
            crate::preserves_rail::to_text(&listed.value)
                .expect("render catalog result")
                .contains("ledger-kind:service-supervision-report")
        );
        let request =
            crate::catalog_mcp::mcp_request_value("catalog.list", vec![crate::preserves_rail::record("kind", vec![
                crate::preserves_rail::string("service-supervision-report"),
            ])])
            .expect("MCP request");
        let mcp =
            crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("MCP list supervision report");
        assert_eq!(mcp.decision, "pass");
        assert!(
            crate::preserves_rail::to_text(&mcp.response_value)
                .expect("render MCP response")
                .contains("service-supervision-report")
        );
    }

    #[test]
    fn malformed_os_parentage_is_not_supervision_evidence() {
        let value = crate::preserves_rail::parse_text(
            "<service-link-v1 \"molten.service.link.v1\" <supervisor-id \"supervisor:web\"> \
             <parent-service \"1234\"> <child-service \"svc:web\"> <propagation \"restart\"> \
             <policy []> <checks [<check \"logical-supervision\" \"pass\">]>>",
        )
        .expect("parse malformed link");
        assert!(crate::service_records::parse_service_link(&value).is_err());
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_cleanup_bounded_and_monitor_order_deterministic(tc: TestCase) {
        let attempt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(3));
        let suite_value = suite_with_attempt(attempt);
        let run = run_service_supervision_suite_value(&suite_value).expect("generated supervision run");
        let replay = replay_service_supervision_report(&run.value).expect("generated replay");
        assert_eq!(replay.decision, "pass");
        let is_restart_denied = attempt >= 2;
        if is_restart_denied {
            assert_eq!(run.cleanup_receipts.len(), 1);
            assert!(run.scheduled_demands.is_empty());
        } else {
            assert!(run.cleanup_receipts.is_empty());
            assert_eq!(run.scheduled_demands.len(), 1);
        }
        let second_run = run_service_supervision_suite_value(&suite_value).expect("rerun generated supervision");
        assert_eq!(run.monitor_notifications, second_run.monitor_notifications);
    }
