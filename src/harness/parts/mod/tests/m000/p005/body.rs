
    const EXPECTED_GATE_CHECKS: &[&str] = &[
        "budget",
        "explicit-budget-fixture",
        "no-default-resource-policy",
        "resource-policy-preflight",
        "nickel-resource-policy",
        "nickel-resource-export",
        "basalt-resource-receipt",
        "budget-usage-binding",
        "actor-registry",
        "explicit-actor-registry",
        "no-inferred-actors",
        "executor-boundary",
        "admission-policy",
        "policy-preflight",
        "nickel-static-policy",
        "nickel-policy-source",
        "nickel-export-normalization",
        "basalt-policy-gate",
        "basalt-preflight-receipt",
        "basalt-receipt-binding",
        "steel-predicate-review",
        "explicit-capability-fixture",
        "no-implicit-authority",
        "capability-context",
        "capability-grants",
        "basalt-authority-receipt",
        "capability-proofset-binding",
        "grant-ref-binding",
        "deny-without-capability",
        "authority-ref-binding",
        "admission-decisions",
        "deny-rollback",
        "denied-effect-suppression",
        "runtime-predicate-receipts",
        "assertion-visibility-predicate",
        "turn-commit-rollback-predicate",
        "observe-delivery-predicate",
        "executor-conformance-suite-binding",
        "cross-kind-hostcall-conformance",
        "chain-continuity",
        "chain-anchor-descent",
        "chain-checkpoint-freshness",
        "chain-predicate-receipts",
        "turn-journal-chains",
        "turn-journal-input-binding",
        "turn-journal-admission-binding",
        "turn-journal-state-binding",
        "turn-journal-no-global-head",
    ];

    fn assert_expected_gate_checks(checks: &[String]) {
        for expected in EXPECTED_GATE_CHECKS {
            assert!(checks.iter().any(|check| check == expected), "missing gate check {expected}");
        }
    }

    #[test]
    fn gate_rejects_failure_and_failure_repro_bundle_as_pass_evidence() {
        let error = MoltenError::invalid_harness("synthetic preflight failure");
        let failure = failure_value("preflight", &error, Vec::new());
        let gate_error = check_value(&failure).expect_err("failure cannot satisfy gate");
        assert!(gate_error.to_string().contains("cannot satisfy pass evidence gate"));

        let failure_bundle = failure_repro_bundle_value(&failure).expect("failure bundle");
        let parsed_bundle = parse_repro_bundle(&failure_bundle).expect("parse failure bundle");
        assert_eq!(parsed_bundle.kind, super::ReproBundleKind::Failure);
        let gate_error = check_value(&failure_bundle).expect_err("failure bundle cannot satisfy gate");
        assert!(gate_error.to_string().contains("cannot satisfy pass evidence gate"));
    }

    #[test]
    fn boundary_coverage_identifies_unexercised_policy_denial_gate() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let coverage = boundary_coverage_value(&run.report_value).expect("boundary coverage");
        let coverage_text = to_text(&coverage).expect("render coverage");

        assert!(coverage_text.contains("harness-boundary-coverage-v1"));
        assert!(coverage_text.contains("<boundary \"envelope-routes\" \"exercised\">"));
        assert!(coverage_text.contains("<boundary \"policy-gates\" \"exercised\">"));
        assert!(coverage_text.contains("<boundary \"policy-denials\" \"unexercised\">"));
        assert!(coverage_text.contains("<unexercised ["));
        assert!(coverage_text.contains("\"policy-denials\""));
    }

    #[test]
    fn golden_trace_update_receipt_binds_report_trace_state_and_reason() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let reviewer_ref =
            canonical_hash(&parse_text("<reviewer \"alice\">").expect("reviewer")).expect("reviewer ref");
        let receipt = golden_trace_update_receipt_value(None, &run.report_value, "bug-fix", &reviewer_ref)
            .expect("golden update receipt");
        let receipt_text = to_text(&receipt).expect("render golden receipt");

        assert!(receipt_text.contains("golden-trace-update-receipt-v1"));
        assert!(receipt_text.contains("<reason \"bug-fix\">"));
        assert!(receipt_text.contains("reviewed-update-receipt"));
        assert!(receipt_text.contains(&run.report_ref));
        assert!(receipt_text.contains(&run.final_state_hash));
        validate_golden_trace_update_receipt(&receipt, &run.report_value).expect("validate golden update receipt");
    }

    #[test]
    fn harness_run_receipt_binds_suite_steps_adapter_status_and_exports() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let export_ref = canonical_hash(&run.report_value).expect("report export ref");
        let receipt = run_receipt_value(&run.report_value, &[&export_ref]).expect("harness run receipt");
        let receipt_text = to_text(&receipt).expect("render harness run receipt");

        assert!(receipt_text.contains("harness-run-receipt-v1"));
        assert!(receipt_text.contains("suite-start-bound"));
        assert!(receipt_text.contains("step-results-bound"));
        assert!(receipt_text.contains("adapter-fixture-decision-bound"));
        assert!(receipt_text.contains("final-status-bound"));
        assert!(receipt_text.contains(&export_ref));
        validate_harness_run_receipt(&receipt, &run.report_value, &[&export_ref]).expect("validate run receipt");
    }

    #[test]
    fn deterministic_multipeer_receipt_is_stable_for_same_seed_and_partition_profile() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let events = [
            "deliver",
            "partition",
            "drop",
            "reorder",
            "reconnect",
            "gossip",
            "doc",
            "blob",
            "resource-limit",
        ];
        let receipt_a = deterministic_multipeer_receipt_value(&run.report_value, 42, "seeded", &events)
            .expect("multi-peer receipt a");
        let receipt_b = deterministic_multipeer_receipt_value(&run.report_value, 42, "seeded", &events)
            .expect("multi-peer receipt b");
        let receipt_text = to_text(&receipt_a).expect("render multi-peer receipt");

        assert_eq!(
            canonical_hash(&receipt_a).expect("receipt a hash"),
            canonical_hash(&receipt_b).expect("receipt b hash")
        );
        assert!(receipt_text.contains("deterministic-multipeer-receipt-v1"));
        assert!(receipt_text.contains("<replay \"stable\">"));
        assert!(receipt_text.contains("partition-replay-stable"));
        assert!(receipt_text.contains("no-live-unrecorded-peer-io"));
        validate_deterministic_multipeer_receipt(&receipt_a, &run.report_value, 42, "seeded", &events)
            .expect("validate deterministic multi-peer receipt");
    }

    #[test]
    fn deterministic_multipeer_receipt_rejects_live_unrecorded_peer_delivery() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let error = deterministic_multipeer_receipt_value(&run.report_value, 42, "seeded", &["deliver", "live"])
            .expect_err("live event fails");
        assert!(error.to_string().contains("live or unrecorded peer delivery"), "{error}");
    }

    #[test]
    fn upgrade_replay_receipt_accepts_stable_replay() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let receipt = upgrade_replay_receipt_value(&run.report_value, &run.report_value, None, None)
            .expect("stable upgrade replay receipt");
        let receipt_text = to_text(&receipt).expect("render upgrade replay receipt");

        assert!(receipt_text.contains("upgrade-replay-receipt-v1"));
        assert!(receipt_text.contains("<outcome \"stable\">"));
        validate_upgrade_replay_receipt(&receipt, &run.report_value, &run.report_value)
            .expect("validate stable upgrade replay receipt");
    }

    #[test]
    fn upgrade_replay_receipt_requires_explained_trace_drift() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let drifted_text = report_text.replacen(
            &run.final_state_hash,
            "blake3:1111111111111111111111111111111111111111111111111111111111111111",
            1,
        );
        let drifted_report = parse_text(&drifted_text).expect("parse drifted report");
        let error = upgrade_replay_receipt_value(&run.report_value, &drifted_report, None, None)
            .expect_err("unexplained drift fails");
        assert!(error.to_string().contains("trace drift requires migration receipt"), "{error}");

        let diagnostic_ref =
            canonical_hash(&parse_text("<compatibility-diagnostic \"intentional\">").expect("diagnostic"))
                .expect("diagnostic ref");
        let explained = upgrade_replay_receipt_value(&run.report_value, &drifted_report, None, Some(&diagnostic_ref))
            .expect("diagnosed drift passes");
        let explained_text = to_text(&explained).expect("render explained upgrade receipt");
        assert!(explained_text.contains("<outcome \"diagnosed\">"));
        validate_upgrade_replay_receipt(&explained, &run.report_value, &drifted_report)
            .expect("validate diagnosed upgrade replay receipt");
    }

    #[test]
    fn golden_trace_update_receipt_rejects_unclassified_reason() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let reviewer_ref =
            canonical_hash(&parse_text("<reviewer \"alice\">").expect("reviewer")).expect("reviewer ref");
        let error = golden_trace_update_receipt_value(None, &run.report_value, "because", &reviewer_ref)
            .expect_err("unclassified golden reason fails");
        assert!(error.to_string().contains("unsupported golden trace update reason"), "{error}");
    }

    #[test]
    fn non_replayable_exploratory_pass_report_cannot_satisfy_gate() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let exploratory_text = report_text.replacen("\"deterministic\"", "\"non-replayable\"", 1);
        let exploratory_report = parse_text(&exploratory_text).expect("parse exploratory report");

        let error =
            check_value(&exploratory_report).expect_err("non-replayable pass report cannot satisfy deterministic gate");
        assert!(error.to_string().contains("unsupported evidence replay status non-replayable"), "{error}");
    }

    #[test]
    fn actor_executor_registry_marks_placeholders() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-future" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "module" "wasm">]>
              [<clock "module">]>"#,
        )
        .expect("parse wasm actor suite");
        let parsed = parse_suite(&suite).expect("parse suite");
        let executors = actor_executor_registry(&parsed.actors);
        assert_eq!(executors.len(), 1);
        assert_eq!(executors[0].executor_kind, super::ActorExecutorKind::WasmPlaceholder);
        assert!(!executors[0].supported);
    }

    #[test]
    fn unsupported_actor_kind_fails_for_now() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-future" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "module" "wasm">]>
              [<clock "module">]>"#,
        )
        .expect("parse wasm actor suite");
        let error = run_suite_value(&suite).expect_err("wasm actor kind requires explicit preflight");
        assert!(error.to_string().contains("missing Wasm executor preflight fixture"));
    }

    #[test]
    fn adapter_and_remote_proxy_kinds_remain_fail_closed_without_preflight() {
        for kind in ["adapter", "remote-proxy"] {
            let suite = parse_text(&format!(
                r#"<harness-suite-v1 "molten.harness.suite.v1" "{kind}-disabled" 1
                  <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
                  <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "subject" "{kind}">]>
                  <capabilities-v1 "molten.harness.capabilities.v1" [<grant "subject" "assert" #f "service.ready">]>
                  [<assert "subject" "service.ready">]>"#,
            ))
            .expect("parse disabled executor suite");
            let error = run_suite_value(&suite).expect_err("adapter-like executor kind remains disabled");
            assert!(
                error.to_string().contains(&format!(
                    "executor kind {kind} requires executor adapter preflight and remains disabled in local harness"
                )),
                "{error}"
            );
        }
    }
