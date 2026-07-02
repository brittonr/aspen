
    #[test]
    fn time_random_effects_use_deterministic_local_handler_receipts() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        validate_report_value(&run.report_value).expect("validate report");
        replay_report_value(&run.report_value).expect("replay report");
        let report = parse_report(&run.report_value).expect("parse report");
        let report_text = to_text(&run.report_value).expect("render report");
        assert_eq!(report.replay_status, "deterministic");
        assert_eq!(report.profile, "local-deterministic");
        assert!(report_text.contains("time-random-handler-receipt-v1"));
        assert!(report_text.contains("local-deterministic"));
        assert!(report_text.contains("deny-by-default-bypassed-only-by-local-test-handler"));
    }

    #[test]
    fn report_binds_deterministic_handler_profile_identity() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let replay = replay_report_value(&run.report_value).expect("replay report");

        assert_eq!(report.profile, "local-deterministic");
        assert_eq!(report.replay_status, "deterministic");
        assert_eq!(replay.expected_report_ref, run.report_ref);
        assert_eq!(replay.actual_report_ref, run.report_ref);
    }

    #[test]
    fn turn_journal_refs_are_stable_under_replay() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let replay = replay_report_value(&run.report_value).expect("replay report");
        let parsed_suite = parse_suite(&report.suite_value).expect("parse embedded suite");
        let replayed_run = run_suite_with_effect_log(&parsed_suite, &report.effect_log).expect("replay run");
        let replay_report = parse_report(&replayed_run.report_value).expect("parse replay report");
        let journal_refs = turn_journal_refs(&report);
        let replay_journal_refs = turn_journal_refs(&replay_report);
        let rendered = to_text(&run.report_value).expect("render report");

        assert_eq!(replay.expected_report_ref, replay.actual_report_ref);
        assert_eq!(journal_refs.len(), report.observations.len());
        assert_eq!(journal_refs, replay_journal_refs);
        assert!(rendered.contains("turn-journal-v1"));
        assert!(rendered.contains("scheduler-key"));
        assert!(rendered.contains("effect-refs"));
        assert!(rendered.contains("receipt-refs"));
    }

    fn turn_journal_refs(report: &super::schema::Report) -> Vec<String> {
        report
            .observations
            .iter()
            .flat_map(|observation| observation.events.iter())
            .filter(|event| event.collect_simple_record("turn-journal-v1", None).is_some())
            .map(|event| canonical_hash(event).expect("journal ref"))
            .collect()
    }

    #[test]
    fn capability_missing_effect_grant_suppresses_effect_request() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "capability-deny-clock" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" []>
              [<clock "producer">]>"#,
        )
        .expect("parse missing clock grant suite");
        let run = run_suite_value(&suite).expect("missing effect grant is recorded denial evidence");
        validate_report_value(&run.report_value).expect("validate missing effect grant denial");
        let report_text = to_text(&run.report_value).expect("render missing effect grant report");
        assert!(report_text.contains("missing capability grant"));
        assert!(!report_text.contains("effect-request"));
        assert!(!report_text.contains("effect-response"));
    }

    #[test]
    fn capability_grant_allows_authorized_send() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "capability-allow-send" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "send" "consumer" #f>
              ]>
              [<send "producer" "consumer" "hello">]>"#,
        )
        .expect("parse send grant suite");
        let run = run_suite_value(&suite).expect("authorized send runs");
        validate_report_value(&run.report_value).expect("validate authorized send");
        let report_text = to_text(&run.report_value).expect("render authorized send report");
        assert!(report_text.contains("<authorized #t>"));
        assert!(report_text.contains("message-delivered"));
    }

    #[test]
    fn policy_denied_send_leaves_no_message_and_denied_clock_has_no_effect_response() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-send-and-clock" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "send" "consumer" #f>
                <grant "producer" "clock" #f #f>
              ]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "send" "consumer" #f "producer cannot send to consumer">
                <deny "producer" "clock" #f #f "producer cannot read clock">
              ]>
              [<send "producer" "consumer" "hello"> <clock "producer">]>"#,
        )
        .expect("parse deny send/clock suite");
        let run = run_suite_value(&suite).expect("policy denial report");
        replay_report_value(&run.report_value).expect("replay deny send/clock suite");
        let report_text = to_text(&run.report_value).expect("render deny send/clock report");
        assert!(report_text.contains("producer cannot send to consumer"));
        assert!(report_text.contains("producer cannot read clock"));
        assert!(!report_text.contains("message-delivered"));
        assert!(!report_text.contains("effect-request"));
        assert!(!report_text.contains("effect-response"));
    }

    #[test]
    fn tampered_policy_decision_reports_policy_divergence_with_failure_diagnostics() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "policy-divergence" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "producer" "clock" #f #f>]>
              [<clock "producer">]>"#,
        )
        .expect("parse policy divergence suite");
        let run = run_suite_value(&suite).expect("run policy divergence suite");
        let report_text = to_text(&run.report_value).expect("render policy divergence report");
        let tampered_text = report_text.replacen("admission-decision-v1", "admission-decision-tampered", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered policy decision report");
        let error = replay_report_value(&tampered_report).expect_err("tampered policy decision diverges");
        match &error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "policy-decision");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected policy-decision divergence, got {other}"),
        }
        let failure = failure_value("replay", &error, Vec::new());
        let parsed_failure = parse_failure(&failure).expect("parse policy divergence failure");
        assert_eq!(parsed_failure.kind, "policy-decision");
        let diagnostics = parsed_failure
            .diagnostics
            .iter()
            .map(|diagnostic| to_text(diagnostic).expect("render diagnostic"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(diagnostics.contains("<step 0>"));
        assert!(diagnostics.contains("<detail \"event differs\">"));
    }

    #[test]
    fn report_validation_requires_admission_decision_evidence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("admission-decision-v1", "admission-decision-missing", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing admission report");
        let error = validate_report_value(&tampered_report).expect_err("missing admission evidence fails closed");
        assert!(error.to_string().contains("missing admission decision"));
    }

    #[test]
    fn report_validation_recomputes_admission_decision() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-assert-validation" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "producer" "assert" #f "service.ready">]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse deny assert validation suite");
        let run = run_suite_value(&suite).expect("run deny assert validation suite");
        let report_text = to_text(&run.report_value).expect("render deny assert validation report");
        let tampered_text = report_text.replacen(
            "<decision \"deny\" \"producer cannot assert readiness\">",
            "<decision \"allow\" \"default-allow\">",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered decision report");
        let error = validate_report_value(&tampered_report).expect_err("tampered decision fails validation");
        assert!(error.to_string().contains("admission decision mismatch"));
    }

    #[test]
    fn report_validation_requires_policy_gate_preflight() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("policy-gate-v1"));
        let tampered_text = report_text.replacen("policy-gate-v1", "policy-gate-missing", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing policy gate report");
        let error = validate_report_value(&tampered_report).expect_err("missing policy preflight fails closed");
        assert!(error_contains_any(&error, &["policy gate", "actor-registry"]));
    }

    #[test]
    fn report_validation_rejects_tampered_policy_gate_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let marker = "<policy-ref \"";
        let start = report_text.find(marker).expect("policy ref marker") + marker.len();
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered policy gate report");
        let error = validate_report_value(&tampered_report).expect_err("tampered policy gate ref fails validation");
        assert!(error_contains_any(&error, &[
            "policy gate ref mismatch",
            "Nickel source policy ref does not match policy gate ref",
        ]));
    }

    #[test]
    fn report_validation_requires_basalt_policy_preflight_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("basalt-preflight"));
        let tampered_text = report_text.replacen("basalt-preflight", "basalt-context", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing Basalt preflight report");
        let error = validate_report_value(&tampered_report).expect_err("missing Basalt preflight fails closed");
        assert!(error.to_string().contains("basalt-preflight"));
    }

    #[test]
    fn report_validation_rejects_tampered_basalt_policy_preflight_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("<reason \"accepted\">", "<reason \"tampered\">", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Basalt preflight report");
        let error = validate_report_value(&tampered_report).expect_err("tampered Basalt preflight fails closed");
        assert!(error.to_string().contains("unsupported Basalt policy preflight reason tampered"));
    }

    #[test]
    fn report_validation_rejects_tampered_nickel_policy_export() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("export-json"));
        let tampered_text = report_text.replacen(
            "molten.harness.policy.nickel-static.v1",
            "molten.harness.policy.nickel-static.tampered",
            2,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Nickel export report");
        let error = validate_report_value(&tampered_report).expect_err("tampered Nickel export fails closed");
        assert!(error_contains_any(&error, &[
            "Nickel policy export JSON does not match source normalization",
            "Nickel policy export ref mismatch",
            "unsupported Nickel source schema",
        ]));
    }

    #[test]
    fn report_validation_requires_capability_gate_preflight() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("capability-gate-v1"));
        let tampered_text = report_text.replacen("capability-gate-v1", "capability-gate-missing", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing capability gate report");
        let error = validate_report_value(&tampered_report).expect_err("missing capability preflight fails closed");
        assert!(error_contains_any(&error, &["capability gate", "actor-registry"]));
    }
