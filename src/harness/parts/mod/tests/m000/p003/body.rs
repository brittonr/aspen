
    #[test]
    fn report_validation_rejects_tampered_capability_gate_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let marker = "<capability-ref \"";
        let start = report_text.find(marker).expect("capability ref marker") + marker.len();
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered capability gate report");
        let error = validate_report_value(&tampered_report).expect_err("tampered capability gate ref fails validation");
        assert!(error_contains_any(&error, &[
            "capability gate ref mismatch",
            "authority contract normalized capability ref does not match capability gate ref",
            "Basalt authority preflight capability ref does not match capability gate ref",
        ]));
    }

    #[test]
    fn report_validation_requires_basalt_authority_preflight_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("basalt-authority-preflight"));
        let tampered_text = report_text.replacen("basalt-authority-preflight", "basalt-authority-context", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing Basalt authority report");
        let error =
            validate_report_value(&tampered_report).expect_err("missing Basalt authority preflight fails closed");
        assert!(error.to_string().contains("basalt-authority-preflight"));
    }

    #[test]
    fn report_validation_rejects_tampered_basalt_authority_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let capability_gate_start = report_text.find("capability-gate-v1").expect("capability gate");
        let reason_relative =
            report_text[capability_gate_start..].find("<reason \"accepted\">").expect("authority reason");
        let start = capability_gate_start + reason_relative;
        let tampered_text = format!(
            "{}<reason \"tampered\">{}",
            &report_text[..start],
            &report_text[start + "<reason \"accepted\">".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Basalt authority report");
        let error = validate_report_value(&tampered_report).expect_err("tampered Basalt authority fails closed");
        assert!(error.to_string().contains("unsupported Basalt authority preflight reason tampered"));
    }

    #[test]
    fn report_validation_rejects_non_empty_ucan_proofset_until_supported() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let empty_proofset = "<ucan-proofset-v1 \"molten.harness.ucan-proofset.v1\" []>";
        assert!(report_text.contains(empty_proofset));
        let tampered_text = report_text.replacen(
            empty_proofset,
            "<ucan-proofset-v1 \"molten.harness.ucan-proofset.v1\" [<proof \"unchecked\">]>",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse non-empty UCAN proofset report");
        let error = validate_report_value(&tampered_report).expect_err("non-empty UCAN proofset fails closed");
        assert!(error.to_string().contains("UCAN proof refs require matching UCAN verification receipts"));
    }

    #[test]
    fn report_validation_rejects_tampered_capability_grant_ref_binding() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let grant_refs_start = report_text.find("<grant-refs [").expect("grant refs");
        let hash_relative = report_text[grant_refs_start..].find("blake3:").expect("first grant ref");
        let start = grant_refs_start + hash_relative;
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered grant ref report");
        let error = validate_report_value(&tampered_report).expect_err("tampered grant ref fails closed");
        assert!(error_contains_any(&error, &[
            "capability gate grant refs do not match embedded capabilities",
            "capability gate evidence does not match embedded authority preflight",
        ]));
    }

    #[test]
    fn report_validation_rejects_tampered_admission_authority() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let gate_ref_marker = "<capability-ref \"";
        let admission_start = report_text.find("admission-decision-v1").expect("admission decision");
        let authority_relative =
            report_text[admission_start..].find(gate_ref_marker).expect("admission capability ref");
        let start = admission_start + authority_relative + gate_ref_marker.len();
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered authority report");
        let error = validate_report_value(&tampered_report).expect_err("tampered authority binding fails validation");
        assert!(error.to_string().contains("capability authority mismatch"));
        let error = replay_report_value(&tampered_report).expect_err("tampered authority binding diverges on replay");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "capability-decision");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected capability-decision divergence, got {other}"),
        }
    }

    #[test]
    fn unreviewed_steel_predicate_policy_fails_preflight() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-predicate-denied" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <policy-v1 "molten.harness.policy.v1" [
                <steel-predicate "unchecked-dynamic-callable">
              ]>
              [<clock "producer">]>"#,
        )
        .expect("parse suite with unreviewed predicate record");
        let error = parse_suite(&suite).expect_err("unreviewed Steel predicate cannot enter local policy fixture");
        assert!(error.to_string().contains("Steel predicates require reviewed callable receipts"));
    }

    #[test]
    fn report_validation_rejects_committed_action_after_denial() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-commit-validation" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "producer" "assert" #f "service.ready">]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse deny commit validation suite");
        let run = run_suite_value(&suite).expect("run deny commit validation suite");
        let report_text = to_text(&run.report_value).expect("render deny commit validation report");
        let tampered_text = report_text.replacen(
            "<turn-rolled-back \"producer\" \"producer cannot assert readiness\">",
            "<turn-rolled-back \"producer\" \"producer cannot assert readiness\">, <assertion-committed \"producer\" \"service.ready\">",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse denied commit report");
        let error = validate_report_value(&tampered_report).expect_err("denied commit fails validation");
        assert!(error.to_string().contains("denied turn committed action"));
    }

    #[test]
    fn report_validation_rejects_effect_response_after_denial() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-effect-validation" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "producer" "clock" #f #f>]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "clock" #f #f "producer cannot read clock">
              ]>
              [<clock "producer">]>"#,
        )
        .expect("parse deny effect validation suite");
        let run = run_suite_value(&suite).expect("run deny effect validation suite");
        let report_text = to_text(&run.report_value).expect("render deny effect validation report");
        let tampered_text = report_text.replacen(
            "<turn-rolled-back \"producer\" \"producer cannot read clock\">",
            "<turn-rolled-back \"producer\" \"producer cannot read clock\">, <effect-request \"clock\" \"producer\" 0>, <effect-response \"clock\" \"producer\" 0 0>",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse denied effect report");
        let error = validate_report_value(&tampered_report).expect_err("denied effect fails validation");
        assert!(error.to_string().contains("denied effect emitted effect request/response"));
    }

    #[test]
    fn unknown_actor_in_step_fails() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "unknown-actor" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              [<send "producer" "missing" "hello">]>"#,
        )
        .expect("parse unknown actor suite");
        let error = run_suite_value(&suite).expect_err("unknown actor should fail");
        assert!(error.to_string().contains("unknown actor missing"));
    }

    #[test]
    fn canonical_failure_artifact_records_unknown_actor() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "unknown-actor" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              [<send "producer" "missing" "hello">]>"#,
        )
        .expect("parse unknown actor suite");
        let error = run_suite_value(&suite).expect_err("unknown actor should fail");
        let failure_value = suite_failure_value("preflight", &error, &suite).expect("failure value");
        let failure = parse_failure(&failure_value).expect("parse failure artifact");
        assert_eq!(failure.phase, "preflight");
        assert_eq!(failure.kind, "invalid-harness");
        assert!(failure.message.contains("unknown actor missing"));
        assert!(failure.diagnostics.iter().any(|diagnostic| {
            to_text(diagnostic)
                .expect("render diagnostic")
                .contains(&canonical_hash(&suite).expect("suite hash"))
        }));
        let rendered = to_text(&failure_value).expect("render failure");
        let reparsed = parse_text(&rendered).expect("reparse failure");
        assert_eq!(canonical_hash(&failure_value).unwrap(), canonical_hash(&reparsed).unwrap());
    }

    #[test]
    fn canonical_failure_artifact_records_resource_divergence() {
        let steps = (0..17).map(|_| "<clock \"a\">".to_string()).collect::<Vec<_>>().join(" ");
        let suite = parse_text(&format!(
            "<harness-suite-v1 \"molten.harness.suite.v1\" \"too-many-effects\" 1 \
             <budget-v1 \"molten.harness.budget.v1\" <limits 64 16 256 65536>> \
             <actor-registry-v1 \"molten.harness.actor-registry.v1\" [<actor \"a\" \"native\">]> \
             <capabilities-v1 \"molten.harness.capabilities.v1\" [<grant \"a\" \"clock\" #f #f>]> [{steps}]>"
        ))
        .expect("parse suite");
        let error = run_suite_value(&suite).expect_err("effect budget should fail");
        let failure_value = suite_failure_value("execute", &error, &suite).expect("failure value");
        let failure = parse_failure(&failure_value).expect("parse failure artifact");
        assert_eq!(failure.phase, "execute");
        assert_eq!(failure.kind, "resource");
        assert!(failure.message.contains("effect count exceeds budget"));
        let rendered = failure
            .diagnostics
            .iter()
            .map(|diagnostic| to_text(diagnostic).expect("render diagnostic"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("<step 16>"));
        assert!(rendered.contains("<expected \"16\">"));
        assert!(rendered.contains("<actual \"17\">"));
    }

    #[test]
    fn canonical_failure_artifact_is_not_a_passing_report() {
        let error = MoltenError::invalid_harness("synthetic preflight failure");
        let failure = failure_value("preflight", &error, Vec::new());
        let report_error = validate_report_value(&failure).expect_err("failure artifacts are diagnostic evidence only");
        assert!(report_error.to_string().contains("expected <harness-report-v1"));
    }
