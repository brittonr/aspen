
    #[test]
    fn executor_hostcall_boundary_evidence_is_recorded_and_gated() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("executor-preflights-v1"));
        assert!(report_text.contains("executor-preflight-v1"));
        assert!(report_text.contains("allowed-hostcalls"));
        assert!(report_text.contains("actor-input-v1"));
        assert!(report_text.contains("hostcall-request-v1"));
        assert!(report_text.contains("handler-binding-ref"));
        assert!(report_text.contains("handle-ref"));
        assert!(report_text.contains("effect-manifest-ref"));
        assert!(report_text.contains("handler-profile-ref"));
        assert!(report_text.contains("effect-request-ref"));
        assert!(report_text.contains("effect-binding-receipt-ref"));
        assert!(report_text.contains("hostcall-decision-v1"));
        assert!(report_text.contains("actor-output-v1"));
        let gate = check_value(&run.report_value).expect("gate report");
        let gate_receipt = receipt_value(&gate);
        let receipt_text = to_text(&gate_receipt).expect("render gate receipt");
        assert!(receipt_text.contains("deterministic-replay-verify-v1"));
        assert!(receipt_text.contains("deterministic-replay-verify"));
        assert!(receipt_text.contains("no-divergence"));
        let receipt = parse_receipt(&gate_receipt).expect("parse gate receipt");
        assert!(receipt.checks.iter().any(|check| check == "executor-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "executor-kind-binding"));
        assert!(receipt.checks.iter().any(|check| check == "allowed-hostcall-binding"));
        assert!(receipt.checks.iter().any(|check| check == "no-unsupported-executor-fallback"));
        assert!(receipt.checks.iter().any(|check| check == "executor-conformance-suite-binding"));
        assert!(receipt.checks.iter().any(|check| check == "cross-kind-hostcall-conformance"));
        assert!(receipt.checks.iter().any(|check| check == "executor-execution-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "executor-output-ref-binding"));
        assert!(receipt.checks.iter().any(|check| check == "steel-executor-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "steel-review-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "steel-vm-execution"));
        assert!(receipt.checks.iter().any(|check| check == "steel-resource-bounds"));
        assert!(receipt.checks.iter().any(|check| check == "adapter-executor-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "remote-proxy-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-executor-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-inspection-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-execution-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "wasmtime-no-wasi"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-fuel-memory-bounds"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-abi-byte-bounds"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-guest-memory-bounds"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-preserves-abi-ready"));
        assert!(receipt.checks.iter().any(|check| check == "executor-hostcall-boundary"));
        assert!(receipt.checks.iter().any(|check| check == "hostcall-admission-binding"));
        assert!(receipt.checks.iter().any(|check| check == "hostcall-replay"));
        assert!(receipt.checks.iter().any(|check| check == "effect-handler-binding"));
        assert!(receipt.checks.iter().any(|check| check == "effect-handle-binding"));
        assert!(receipt.checks.iter().any(|check| check == "handle-not-authority"));
        assert!(receipt.checks.iter().any(|check| check == "hostcall-handle-replay"));
        assert!(receipt.checks.iter().any(|check| check == "no-ambient-executor-io"));
    }

    #[test]
    fn hostcall_effect_handles_disambiguate_same_kind_requests() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let mut handle_refs = std::collections::BTreeSet::new();
        for observation in &report.observations {
            let request = observation.events[2]
                .collect_simple_record("hostcall-request-v1", None)
                .expect("hostcall request carries handle refs");
            let handle_ref = request[10].collect_simple_record("handle-ref", Some(1)).expect("handle-ref record")[0]
                .as_string()
                .expect("handle-ref string")
                .into_owned();
            crate::preserves_rail::validate_content_ref(&handle_ref).expect("handle ref is canonical");
            handle_refs.insert(handle_ref);
        }
        assert_eq!(handle_refs.len(), report.observations.len());
    }

    #[test]
    fn report_validation_rejects_tampered_hostcall_handle_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("handle-ref", "handle-ref-tampered", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered handle report");
        let error = validate_report_value(&tampered_report).expect_err("tampered handle ref fails validation");
        assert!(error.to_string().contains("hostcall-request evidence mismatch"), "{error}");
        let replay_error = replay_report_value(&tampered_report).expect_err("tampered handle ref diverges");
        assert!(replay_error.to_string().contains("hostcall-request"), "{replay_error}");
    }

    #[test]
    fn gate_receipt_requires_executor_resource_checks() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let gate = check_value(&run.report_value).expect("gate report");
        let receipt_value = receipt_value(&gate);
        let receipt_text = to_text(&receipt_value).expect("render receipt");
        assert!(receipt_text.contains("executor-execution-receipts"));
        let tampered_text = receipt_text.replacen("executor-execution-receipt-binding", "executor-execution-stale", 1);
        let tampered = parse_text(&tampered_text).expect("parse tampered gate receipt");
        let error = parse_receipt(&tampered).expect_err("missing executor execution binding check fails");
        assert!(error.to_string().contains("executor-execution-receipt-binding"), "{error}");

        let tampered_replay_text =
            receipt_text.replacen("<divergence \"none\">", "<divergence \"effect-response\">", 1);
        let tampered_replay = parse_text(&tampered_replay_text).expect("parse tampered replay receipt");
        let error = parse_receipt(&tampered_replay).expect_err("tampered generic replay receipt fails");
        assert!(error.to_string().contains("replay verify ref"), "{error}");
    }

    #[test]
    fn gate_receipt_binds_chain_continuity_anchor_and_checkpoint_evidence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let gate = check_value(&run.report_value).expect("gate report");
        let receipt_value = receipt_value(&gate);
        let receipt_text = to_text(&receipt_value).expect("render receipt");
        assert!(receipt_text.contains("chain-evidence"));
        assert!(receipt_text.contains("chain-link-v1"));
        assert!(receipt_text.contains("chain-verify-receipt-v1"));
        assert!(receipt_text.contains("chain-checkpoint-v1"));
        let receipt = parse_receipt(&receipt_value).expect("parse chained gate receipt");
        assert!(receipt.checks.iter().any(|check| check == "chain-continuity"));
        assert!(receipt.checks.iter().any(|check| check == "chain-anchor-descent"));
        assert!(receipt.checks.iter().any(|check| check == "chain-checkpoint-freshness"));
        assert!(receipt.checks.iter().any(|check| check == "chain-predicate-receipts"));

        let missing_check = parse_text(&receipt_text.replacen("chain-continuity", "chain-stale", 1))
            .expect("parse missing chain check receipt");
        let error = parse_receipt(&missing_check).expect_err("missing chain continuity check fails");
        assert!(error.to_string().contains("chain-continuity"), "{error}");

        let tampered_predicate = parse_text(&receipt_text.replacen(
            "molten.chain.checkpoint_covers_range.v1",
            "molten.chain.segment_no_gap.v1",
            1,
        ))
        .expect("parse tampered range predicate receipt");
        let error = parse_receipt(&tampered_predicate).expect_err("tampered range predicate fails");
        assert!(error_contains_any(&error, &["range predicate", "checkpoint"]), "{error}");
    }

    #[test]
    fn gate_receipt_binds_actor_scoped_turn_journals() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let gate = check_value(&run.report_value).expect("gate report");
        let receipt_value = receipt_value(&gate);
        let receipt_text = to_text(&receipt_value).expect("render receipt");
        assert!(receipt_text.contains("turn-journals"));
        assert!(receipt_text.contains("turn-journal"));
        assert!(receipt_text.contains("harness-turn-journal"));
        let receipt = parse_receipt(&receipt_value).expect("parse turn journal gate receipt");
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-chains"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-input-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-admission-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-state-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-no-global-head"));

        let global_scope = parse_text(&receipt_text.replacen("harness-turn-journal", "harness-global-journal", 1))
            .expect("parse global turn journal tamper");
        let error = parse_receipt(&global_scope).expect_err("global turn journal scope fails");
        assert!(error_contains_any(&error, &["not global", "turn journal"]), "{error}");

        let start = receipt_text.find("turn-journals").expect("turn journals text");
        let admission = start + receipt_text[start..].find("\"admission\"").expect("admission context");
        let missing_admission = format!(
            "{}\"missing-admission\"{}",
            &receipt_text[..admission],
            &receipt_text[admission + "\"admission\"".len()..]
        );
        let missing_admission = parse_text(&missing_admission).expect("parse missing admission tamper");
        let error = parse_receipt(&missing_admission).expect_err("missing admission context fails");
        assert!(error.to_string().contains("admission"), "{error}");
    }

    #[test]
    fn report_validation_rejects_tampered_executor_preflight() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.find("executor-preflight-v1").expect("executor preflight");
        let relative = report_text[start..].find("allowed-hostcalls").expect("allowed hostcalls");
        let absolute = start + relative;
        let tampered_text = format!(
            "{}tampered-hostcalls{}",
            &report_text[..absolute],
            &report_text[absolute + "allowed-hostcalls".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered executor preflight report");
        let error = validate_report_value(&tampered_report).expect_err("tampered preflight fails validation");
        assert!(error_contains_any(&error, &["executor preflight", "allowed-hostcalls"]));
    }

    #[test]
    fn report_validation_rejects_tampered_hostcall_decision() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.find("hostcall-decision-v1").expect("hostcall decision");
        let relative = report_text[start..].find("admission-binding").expect("hostcall admission binding check");
        let absolute = start + relative;
        let tampered_text = format!(
            "{}tampered-binding{}",
            &report_text[..absolute],
            &report_text[absolute + "admission-binding".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered hostcall report");
        let error = validate_report_value(&tampered_report).expect_err("tampered hostcall decision fails validation");
        assert!(error.to_string().contains("hostcall-decision evidence mismatch"));
        let replay_error = replay_report_value(&tampered_report).expect_err("tampered hostcall decision diverges");
        assert!(replay_error.to_string().contains("hostcall-decision"));
    }

    #[test]
    fn suite_steps_accept_canonical_preserves_payloads() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "preserves-payloads" 9
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "consumer" "observe" #f #f>
                <grant "producer" "assert" #f #f>
                <grant "producer" "send" "consumer" #f>
              ]>
              [
                <observe "consumer" <service-ready "db" 1>>
                <assert "producer" <service-ready "db" 1>>
                <send "producer" "consumer" <payload [1 2 3] <ok>>>
              ]>"#,
        )
        .expect("parse Preserves-valued suite");
        let run = run_suite_value(&suite).expect("run Preserves-valued suite");
        replay_report_value(&run.report_value).expect("replay Preserves-valued suite");
        validate_report_value(&run.report_value).expect("validate Preserves-valued report");
        let report_text = to_text(&run.report_value).expect("render Preserves-valued report");
        assert!(report_text.contains("<service-ready \"db\" 1>"));
        assert!(report_text.contains("<payload [1, 2, 3] <ok>>"));
    }

    #[test]
    fn policy_denied_assert_rolls_back_without_committing_assertion() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-assert" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "consumer" "observe" #f "service.ready">
                <grant "producer" "assert" #f "service.ready">
              ]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">
              ]>
              [<observe "consumer" "service.ready"> <assert "producer" "service.ready">]>"#,
        )
        .expect("parse deny assert suite");
        let run = run_suite_value(&suite).expect("policy denial is recorded evidence, not ambient failure");
        replay_report_value(&run.report_value).expect("replay deny assert suite");
        let report_text = to_text(&run.report_value).expect("render deny assert report");
        assert!(report_text.contains("admission-decision-v1"));
        assert!(report_text.contains("<decision \"deny\" \"producer cannot assert readiness\">"));
        assert!(report_text.contains("<turn-rolled-back \"producer\" \"producer cannot assert readiness\">"));
        assert!(!report_text.contains("assertion-committed"));
        assert!(!report_text.contains("assertion-observed"));
    }

    #[test]
    fn capability_missing_send_grant_denies_delivery() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "capability-deny-send" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" []>
              [<send "producer" "consumer" "hello">]>"#,
        )
        .expect("parse missing send grant suite");
        let run = run_suite_value(&suite).expect("missing grant is recorded denial evidence");
        validate_report_value(&run.report_value).expect("validate missing grant denial");
        replay_report_value(&run.report_value).expect("replay missing grant denial");
        let report_text = to_text(&run.report_value).expect("render missing grant report");
        assert!(report_text.contains("missing capability grant"));
        assert!(report_text.contains("<authorized #f>"));
        assert!(!report_text.contains("message-delivered"));
    }
