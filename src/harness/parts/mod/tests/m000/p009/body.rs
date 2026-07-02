
    #[test]
    fn replay_profile_injects_recorded_clock_random_effects() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let parsed_suite = parse_suite(&report.suite_value).expect("parse embedded suite");
        let replayed = run_suite_with_effect_log(&parsed_suite, &report.effect_log).expect("replay with effect log");
        let rendered = to_text(&replayed.report_value).expect("render replayed report");

        assert_eq!(run.report_ref, replayed.report_ref);
        assert_eq!(run.final_state_hash, replayed.final_state_hash);
        assert!(rendered.contains("time-random-handler-receipt-v1"));
        assert!(rendered.contains("deny-by-default-bypassed-only-by-local-test-handler"));
    }

    #[test]
    fn tampered_effect_response_reports_effect_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("effect-response", "effect-response-tampered", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered report must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "effect-response");
                assert_eq!(divergence.step, Some(3));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn tampered_effect_log_request_reports_effect_request_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen(
            "<effect-entry 1 <effect-request \"random\" \"producer\" 1 100>",
            "<effect-entry 1 <effect-request \"random\" \"producer\" 1 99>",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered effect log must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "effect-request");
                assert_eq!(divergence.step, Some(4));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn missing_effect_log_entry_reports_effect_log_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.find("<effect-log-v1").expect("effect log start");
        let relative_end = report_text[start..].find("]>").expect("effect log end") + 2;
        let end = start + relative_end;
        let tampered_text = format!(
            "{}<effect-log-v1 \"molten.harness.effect-log.v1\" []>{}",
            &report_text[..start],
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("missing effect log must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "effect-log");
                assert_eq!(divergence.step, Some(3));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn tampered_runtime_predicate_reports_runtime_predicate_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen(
            "molten.trellis-runtime.turn-commit-rollback.v1",
            "molten.trellis-runtime.turn-commit-rollback-tampered.v1",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered runtime predicate must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "runtime-predicate");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn report_validation_rejects_missing_runtime_predicate_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("runtime-predicate-receipt-v1", "runtime-predicate-missing-v1", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error =
            validate_report_value(&tampered_report).expect_err("missing runtime predicate must fail validation");
        assert!(error.to_string().contains("runtime predicate"));
    }

    #[test]
    fn tampered_final_state_reports_state_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen(
            &run.final_state_hash,
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered report must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "final-state");
                assert_eq!(divergence.step, None);
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn report_validation_rejects_missing_evidence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen(&run.final_state_hash, "missing-final-state", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = validate_report_value(&tampered_report).expect_err("missing evidence should fail closed");
        assert!(error.to_string().contains("expected canonical content ref"));
    }

    #[test]
    fn too_many_steps_fails_resource_budget() {
        let steps = (0..65).map(|_| "<send \"a\" \"b\" \"m\">".to_string()).collect::<Vec<_>>().join(" ");
        let suite = parse_text(&format!(
            "<harness-suite-v1 \"molten.harness.suite.v1\" \"too-many-steps\" 1 \
             <budget-v1 \"molten.harness.budget.v1\" <limits 64 16 256 65536>> \
             <actor-registry-v1 \"molten.harness.actor-registry.v1\" [<actor \"a\" \"native\"> <actor \"b\" \"native\">]> \
             <capabilities-v1 \"molten.harness.capabilities.v1\" [<grant #f \"send\" #f #f>]> [{steps}]>"
        ))
        .expect("parse suite");
        let error = run_suite_value(&suite).expect_err("step budget should fail");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "resource");
                assert_eq!(divergence.detail, "suite step count exceeds budget");
            }
            other => panic!("expected resource divergence, got {other}"),
        }
    }

    #[test]
    fn too_many_effects_fails_resource_budget() {
        let steps = (0..17).map(|_| "<clock \"a\">".to_string()).collect::<Vec<_>>().join(" ");
        let suite = parse_text(&format!(
            "<harness-suite-v1 \"molten.harness.suite.v1\" \"too-many-effects\" 1 \
             <budget-v1 \"molten.harness.budget.v1\" <limits 64 16 256 65536>> \
             <actor-registry-v1 \"molten.harness.actor-registry.v1\" [<actor \"a\" \"native\">]> \
             <capabilities-v1 \"molten.harness.capabilities.v1\" [<grant \"a\" \"clock\" #f #f>]> [{steps}]>"
        ))
        .expect("parse suite");
        let error = run_suite_value(&suite).expect_err("effect budget should fail");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "resource");
                assert_eq!(divergence.step, Some(16));
                assert_eq!(divergence.detail, "effect count exceeds budget");
            }
            other => panic!("expected resource divergence, got {other}"),
        }
    }

    #[test]
    fn report_validation_requires_budget_evidence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.rfind("budget-v1").expect("report budget marker");
        let tampered_text = format!("{}budget-v0{}", &report_text[..start], &report_text[start + "budget-v1".len()..]);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = validate_report_value(&tampered_report).expect_err("bad budget evidence should fail");
        assert!(error.to_string().contains("expected <budget-v1"));
    }

    #[test]
    fn report_validation_requires_budget_gate_preflight() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("budget-gate-v1"));
        let tampered_text = report_text.replacen("budget-gate-v1", "budget-gate-missing", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing budget gate report");
        let error = validate_report_value(&tampered_report).expect_err("missing resource preflight fails closed");
        assert!(error_contains_any(&error, &["budget gate", "actor-registry"]));
    }

    #[test]
    fn report_validation_rejects_tampered_budget_gate_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let marker = "<budget-ref \"";
        let start = report_text.find(marker).expect("budget ref marker") + marker.len();
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered budget gate report");
        let error = validate_report_value(&tampered_report).expect_err("tampered budget gate ref fails validation");
        assert!(error_contains_any(&error, &[
            "budget gate ref mismatch",
            "Nickel resource policy budget ref does not match budget gate ref",
            "Basalt resource preflight budget ref does not match budget gate ref",
        ]));
    }

    #[test]
    fn report_validation_rejects_tampered_nickel_resource_export() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("export-json"));
        let tampered_text = report_text.replacen(
            "molten.harness.budget.nickel-static.v1",
            "molten.harness.budget.nickel-static.tampered",
            2,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered resource export report");
        let error = validate_report_value(&tampered_report).expect_err("tampered resource export fails closed");
        assert!(error_contains_any(&error, &[
            "Nickel resource policy export JSON does not match source normalization",
            "Nickel resource policy export ref mismatch",
            "unsupported Nickel resource policy schema",
        ]));
    }

    #[test]
    fn report_validation_rejects_tampered_basalt_resource_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let budget_gate_start = report_text.find("budget-gate-v1").expect("budget gate");
        let reason_relative = report_text[budget_gate_start..].find("<reason \"accepted\">").expect("resource reason");
        let start = budget_gate_start + reason_relative;
        let tampered_text = format!(
            "{}<reason \"tampered\">{}",
            &report_text[..start],
            &report_text[start + "<reason \"accepted\">".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered resource receipt report");
        let error = validate_report_value(&tampered_report).expect_err("tampered resource receipt fails closed");
        assert!(error.to_string().contains("unsupported Basalt resource preflight reason tampered"));
    }

    #[test]
    fn old_suite_shape_with_explicit_fixtures_uses_standard_budget() {
        let suite_value = parse_text(OLD_SHAPE_TWO_ACTOR_SUITE).expect("parse old shape suite");
        let suite = parse_suite(&suite_value).expect("parse old shape suite schema");
        assert_eq!(suite.budget.max_steps, 64);
        assert_eq!(suite.budget.max_effects, 16);
        assert!(suite.budget_explicit);
        assert!(suite.actors_explicit);
        assert!(suite.capabilities_explicit);
        let run = run_suite_value(&suite_value).expect("old shape suite should run with explicit standard budget");
        validate_report_value(&run.report_value).expect("old shape report validates with explicit budget evidence");
    }

    #[test]
    fn omitted_budget_fixture_fails_closed() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "default-budget" 1
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "clock" #f #f>]>
              [<clock "a">]>"#,
        )
        .expect("parse omitted budget suite");
        let parsed = parse_suite(&suite).expect("parse omitted budget suite schema");
        assert!(!parsed.budget_explicit);
        assert!(parsed.actors_explicit);
        assert!(parsed.capabilities_explicit);
        let error = run_suite_value(&suite).expect_err("default budget cannot execute evidence-bearing suites");
        assert!(error.to_string().contains("missing explicit budget fixture"));
    }
