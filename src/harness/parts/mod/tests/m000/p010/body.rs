
    #[test]
    fn omitted_actor_registry_fixture_fails_closed() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "inferred-actors" 1
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "clock" #f #f>]>
              [<clock "a">]>"#,
        )
        .expect("parse inferred actor suite");
        let parsed = parse_suite(&suite).expect("parse inferred actor suite schema");
        assert!(!parsed.actors_explicit);
        assert_eq!(parsed.actors.len(), 1);
        let error = run_suite_value(&suite).expect_err("inferred actors cannot execute evidence-bearing suites");
        assert!(error.to_string().contains("missing explicit actor registry fixture"));
    }

    #[test]
    fn explicit_empty_actor_registry_keeps_unknown_actor_failure() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "empty-registry" 1
              <actor-registry-v1 "molten.harness.actor-registry.v1" []>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "clock" #f #f>]>
              [<clock "a">]>"#,
        )
        .expect("parse empty actor registry suite");
        let parsed = parse_suite(&suite).expect("parse empty actor registry suite schema");
        assert!(parsed.actors_explicit);
        let error = run_suite_value(&suite).expect_err("explicit empty registry cannot cover actor steps");
        assert!(error.to_string().contains("unknown actor a"));
    }

    #[test]
    fn report_validation_rejects_legacy_inferred_actor_registry() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = super::schema::parse_report(&run.report_value).expect("parse report");
        let legacy_suite_value = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "two-actor" 7
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "consumer" "observe" #f "service.ready">
                <grant "producer" "assert" #f "service.ready">
                <grant "producer" "send" "consumer" #f>
                <grant "producer" "clock" #f #f>
                <grant "producer" "random" #f #f>
                <grant "producer" "retract" #f "service.ready">
              ]>
              [
                <observe "consumer" "service.ready">
                <assert "producer" "service.ready">
                <send "producer" "consumer" "hello">
                <clock "producer">
                <random "producer" 100>
                <retract "producer" "service.ready">
              ]>"#,
        )
        .expect("parse legacy inferred actor suite");
        let legacy_suite = parse_suite(&legacy_suite_value).expect("parse legacy inferred actor suite schema");
        assert!(!legacy_suite.actors_explicit);
        let legacy_report = super::schema::report_value(super::schema::ReportValueInput {
            suite: &legacy_suite,
            suite_ref: canonical_hash(&legacy_suite_value).expect("legacy suite hash"),
            initial_state_hash: report.initial_state_hash,
            final_state_hash: report.final_state_hash,
            policy_gate: report.policy_gate.expect("policy gate").value,
            capability_gate: report.capability_gate.expect("capability gate").value,
            budget_gate: report.budget_gate.expect("budget gate").value,
            observations: report.observations.iter().map(|observation| observation.value.clone()).collect(),
            effect_log: report.effect_log,
            budget: &legacy_suite.budget,
            usage: &report.budget.usage,
        });
        let error = validate_report_value(&legacy_report).expect_err("legacy inferred actor report fails validation");
        assert!(error.to_string().contains("missing explicit actor registry fixture"));
    }

    #[test]
    fn report_validation_rejects_legacy_default_budget() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = super::schema::parse_report(&run.report_value).expect("parse report");
        let legacy_suite_value = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "two-actor" 7
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "consumer" "observe" #f "service.ready">
                <grant "producer" "assert" #f "service.ready">
                <grant "producer" "send" "consumer" #f>
                <grant "producer" "clock" #f #f>
                <grant "producer" "random" #f #f>
                <grant "producer" "retract" #f "service.ready">
              ]>
              [
                <observe "consumer" "service.ready">
                <assert "producer" "service.ready">
                <send "producer" "consumer" "hello">
                <clock "producer">
                <random "producer" 100>
                <retract "producer" "service.ready">
              ]>"#,
        )
        .expect("parse legacy default budget suite");
        let legacy_suite = parse_suite(&legacy_suite_value).expect("parse legacy default budget suite schema");
        assert!(!legacy_suite.budget_explicit);
        assert!(legacy_suite.actors_explicit);
        assert!(legacy_suite.capabilities_explicit);
        let legacy_report = super::schema::report_value(super::schema::ReportValueInput {
            suite: &legacy_suite,
            suite_ref: canonical_hash(&legacy_suite_value).expect("legacy suite hash"),
            initial_state_hash: report.initial_state_hash,
            final_state_hash: report.final_state_hash,
            policy_gate: report.policy_gate.expect("policy gate").value,
            capability_gate: report.capability_gate.expect("capability gate").value,
            budget_gate: report.budget_gate.expect("budget gate").value,
            observations: report.observations.iter().map(|observation| observation.value.clone()).collect(),
            effect_log: report.effect_log,
            budget: &legacy_suite.budget,
            usage: &report.budget.usage,
        });
        let error = validate_report_value(&legacy_report).expect_err("legacy default budget report fails validation");
        assert!(error.to_string().contains("missing explicit budget fixture"));
    }

    #[test]
    fn omitted_capability_fixture_fails_closed() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "implicit-authority" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              [<clock "a">]>"#,
        )
        .expect("parse implicit authority suite");
        let parsed = parse_suite(&suite).expect("parse implicit authority suite schema");
        assert!(parsed.budget_explicit);
        assert!(parsed.actors_explicit);
        assert!(!parsed.capabilities_explicit);
        let error = run_suite_value(&suite).expect_err("implicit authority cannot execute evidence-bearing suites");
        assert!(error.to_string().contains("missing explicit capability fixture"));
    }

    #[test]
    fn suite_configured_effect_budget_fails() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "low-effect-budget" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 1 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "clock" #f #f>]>
              [<clock "a"> <clock "a">]>"#,
        )
        .expect("parse low effect budget suite");
        let error = run_suite_value(&suite).expect_err("configured effect budget should fail");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "resource");
                assert_eq!(divergence.step, Some(1));
                assert_eq!(divergence.detail, "effect count exceeds budget");
            }
            other => panic!("expected resource divergence, got {other}"),
        }
    }

    #[test]
    fn report_budget_limits_must_match_embedded_suite() {
        let suite = parse_text(OLD_SHAPE_TWO_ACTOR_SUITE).expect("parse old shape suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.rfind("<limits 64 16 256 65536>").expect("report budget limits");
        let tampered_text = format!(
            "{}<limits 64 15 256 65536>{}",
            &report_text[..start],
            &report_text[start + "<limits 64 16 256 65536>".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = validate_report_value(&tampered_report).expect_err("budget mismatch should fail");
        assert!(error.to_string().contains("report budget limits do not match embedded suite budget"));
    }

    #[test]
    fn suite_parser_rejects_unknown_step() {
        let suite = parse_text(r#"<harness-suite-v1 "molten.harness.suite.v1" "bad" 1 [<unknown>] >"#)
            .expect("parse suite text");
        let error = parse_suite(&suite).expect_err("unknown step should fail");
        assert!(error.to_string().contains("unknown harness step"));
    }
