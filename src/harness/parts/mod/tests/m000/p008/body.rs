
    #[test]
    fn reviewed_steel_executor_hostcall_suite_runs_with_preflight_receipt() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-hostcall" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) input)">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel actor suite");
        let parsed = parse_suite(&suite).expect("parse suite");
        let executors = actor_executor_registry(&parsed.actors);
        assert_eq!(executors[0].executor_kind, super::ActorExecutorKind::SteelReviewed);
        assert!(executors[0].supported);
        let run = run_suite_value(&suite).expect("reviewed steel hostcall suite runs");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("steel-review-receipt-v1"));
        assert!(report_text.contains("steel-execution-receipt-v1"));
        assert!(report_text.contains("steel-vm-executed"));
        assert!(report_text.contains("effect-manifest-bound"));
        assert!(report_text.contains("effect-request-admitted"));
        assert!(report_text.contains("declared-effect-id-required"));
        assert!(report_text.contains("steel-source-ref-binding"));
        assert!(report_text.contains("steel-callable-review"));
        validate_report_value(&run.report_value).expect("steel report validates");
    }

    #[test]
    fn steel_actor_without_reviewed_executor_fixture_fails_closed() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-missing-preflight" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "steel">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel actor suite");
        let error = run_suite_value(&suite).expect_err("steel actor needs reviewed executor fixture");
        assert!(error.to_string().contains("missing reviewed Steel executor preflight fixture"));
    }

    #[test]
    fn steel_executor_preflight_rejects_ambient_source_tokens() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-ambient" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(open-input-file \"/tmp/ambient\")">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel actor suite");
        let error = run_suite_value(&suite).expect_err("ambient steel source fails preflight");
        assert!(error.to_string().contains("forbidden ambient IO token open-input-file"));
    }

    #[test]
    fn steel_executor_preflight_rejects_undeclared_hostcall() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-hostcall-deny" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) input)">
                  <callable "main">
                  <allowed-hostcalls ["send"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel actor suite");
        let error = run_suite_value(&suite).expect_err("undeclared Steel hostcall fails preflight");
        assert!(error.to_string().contains("hostcall operation assert is not allowed by Steel executor preflight"));
    }

    #[test]
    fn steel_execution_receipt_tamper_reports_replay_divergence() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-execution-tamper" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) input)">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel tamper suite");
        let run = run_suite_value(&suite).expect("run steel tamper suite");
        let report_text = to_text(&run.report_value).expect("render steel report");
        assert!(report_text.contains("resource-bounded"));
        let tampered_text = report_text.replacen("steel-vm-executed", "steel-vm-stale", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Steel execution report");
        let error = replay_report_value(&tampered_report).expect_err("tampered Steel execution receipt diverges");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "steel-execution");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected steel-execution divergence, got {other}"),
        }
    }

    #[test]
    fn steel_executor_resource_limits_fail_closed() {
        let oversized_source = "x".repeat(9 * 1024);
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-resource" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "{oversized_source}">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        ))
        .expect("parse steel resource suite");
        let error = run_suite_value(&suite).expect_err("oversized Steel source fails resource preflight");
        assert!(error.to_string().contains("exceeds deterministic resource limit"), "{error}");

        let recursive = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-recursive" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) (main input))">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse recursive Steel suite");
        let error = run_suite_value(&recursive).expect_err("recursive Steel source fails resource preflight");
        assert!(error.to_string().contains("recursive callable"), "{error}");

        let unbounded_output = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-unbounded-output" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) (make-string 9000 #\\a))">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse unbounded Steel output suite");
        let error = run_suite_value(&unbounded_output).expect_err("unbounded Steel output fails resource preflight");
        assert!(error.to_string().contains("unbounded resource token make-string"), "{error}");
    }

    #[test]
    fn report_tampered_actor_registry_fails_validation() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let registry_start = report_text.rfind("<actor-registry-v1").expect("report actor registry");
        let native_offset = report_text[registry_start..].find("\"native\"").expect("native actor kind");
        let start = registry_start + native_offset;
        let tampered_text = format!("{}\"steel\"{}", &report_text[..start], &report_text[start + "\"native\"".len()..]);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = validate_report_value(&tampered_report).expect_err("tampered actor registry should fail");
        assert!(error.to_string().contains("report actor registry does not match embedded suite actor registry"));
    }

    #[test]
    fn runtime_snapshot_hashes_are_canonical_and_seed_bound() {
        let left = core::RuntimeState::new(7);
        let right = core::RuntimeState::new(7);
        let other_seed = core::RuntimeState::new(8);
        let left_ref = canonical_hash(&snapshot_value(&left.snapshot())).expect("left snapshot ref");
        let right_ref = canonical_hash(&snapshot_value(&right.snapshot())).expect("right snapshot ref");
        let other_ref = canonical_hash(&snapshot_value(&other_seed.snapshot())).expect("other snapshot ref");

        assert_eq!(left_ref, right_ref);
        assert_ne!(left_ref, other_ref);
    }

    #[test]
    fn replay_preserves_initial_and_final_state_hashes() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let parsed_suite = parse_suite(&report.suite_value).expect("parse embedded suite");
        let replayed = run_suite_with_effect_log(&parsed_suite, &report.effect_log).expect("replay with effect log");
        let replayed_report = parse_report(&replayed.report_value).expect("parse replay report");

        assert_eq!(report.initial_state_hash, replayed_report.initial_state_hash);
        assert_eq!(report.final_state_hash, replayed_report.final_state_hash);
        assert_eq!(run.initial_state_hash, replayed.initial_state_hash);
        assert_eq!(run.final_state_hash, replayed.final_state_hash);
    }

    #[test]
    fn runtime_transition_module_is_deterministic_without_io() {
        let steps = [
            core::CoreStep::Observe {
                actor: "consumer".into(),
                pattern: core::RuntimeValue::string("service.ready").expect("runtime test value"),
            },
            core::CoreStep::Assert {
                actor: "producer".into(),
                value: core::RuntimeValue::string("service.ready").expect("runtime test value"),
            },
            core::CoreStep::Clock {
                actor: "producer".into(),
            },
            core::CoreStep::Random {
                actor: "producer".into(),
                upper: 100,
            },
        ];
        let mut left = core::RuntimeState::new(7);
        let mut right = core::RuntimeState::new(7);
        for step in &steps {
            assert_eq!(left.apply_step(step), right.apply_step(step));
            assert_eq!(left.snapshot(), right.snapshot());
        }
    }

    #[test]
    fn tampered_observation_reports_first_trace_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("message-delivered", "message-tampered", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered report must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "trace");
                assert_eq!(divergence.step, Some(2));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn record_profile_captures_clock_random_effect_log() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let observed_effect_log = effect_log_from_observations(&report.observations).expect("observed effect log");
        let rendered_log = to_text(&super::schema::effect_log_value(&report.effect_log)).expect("render effect log");

        assert_eq!(report.effect_log, observed_effect_log);
        assert_eq!(report.effect_log.len(), 2);
        assert!(rendered_log.contains("<effect-request \"clock\" \"producer\" 0>"));
        assert!(rendered_log.contains("<effect-response \"clock\" \"producer\" 0"));
        assert!(rendered_log.contains("<effect-request \"random\" \"producer\" 1 100>"));
        assert!(rendered_log.contains("<effect-response \"random\" \"producer\" 1 100"));

        let parsed_suite = parse_suite(&report.suite_value).expect("parse embedded suite");
        let replayed = run_suite_with_effect_log(&parsed_suite, &report.effect_log).expect("replay with recorded log");
        assert_eq!(run.report_ref, replayed.report_ref);
    }
