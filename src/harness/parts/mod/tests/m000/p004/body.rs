
    #[test]
    fn gate_accepts_report_and_report_repro_bundle() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_check = check_value(&run.report_value).expect("gate accepts report");
        assert_eq!(report_check.artifact_kind, "report");
        assert_eq!(report_check.report_ref, run.report_ref);

        let bundle = repro_bundle_value(&run.report_value).expect("bundle report");
        let parsed_bundle = parse_repro_bundle(&bundle).expect("parse report bundle");
        assert_eq!(parsed_bundle.kind, super::ReproBundleKind::Report);
        let unsealed_error = check_value(&bundle).expect_err("unsealed report bundle lacks redaction preflight");
        assert!(error_contains_any(&unsealed_error, &["redaction preflight", "gate receipt"]));

        let sealed_bundle =
            sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
                .expect("sealed bundle report");
        let parsed_sealed = parse_repro_bundle(&sealed_bundle).expect("parse sealed bundle");
        assert_eq!(parsed_sealed.kind, super::ReproBundleKind::Report);
        assert!(parsed_sealed.gate_receipt_ref.is_some());
        assert!(parsed_sealed.redaction_policy_ref.is_some());
        assert!(parsed_sealed.redaction_gate_ref.is_some());
        let embedded_receipt =
            parse_receipt(parsed_sealed.receipt_value.as_ref().expect("sealed bundle embeds gate receipt"))
                .expect("parse embedded gate receipt");
        assert_eq!(embedded_receipt.artifact_kind, "report");
        assert_eq!(embedded_receipt.report_ref, run.report_ref);
        let sealed_check = check_value(&sealed_bundle).expect("gate accepts sealed report bundle");
        assert_eq!(sealed_check.artifact_kind, "repro-bundle");
        assert_eq!(sealed_check.report_ref, run.report_ref);
        let verify_receipt_value = repro_verify_receipt_value(&sealed_bundle).expect("verify sealed report bundle");
        let verify_receipt = parse_repro_verify_receipt(&verify_receipt_value).expect("parse repro verify receipt");
        assert_eq!(verify_receipt.decision, "pass");
        assert_eq!(verify_receipt.bundle_ref, canonical_hash(&sealed_bundle).expect("sealed bundle hash"));
        assert_eq!(verify_receipt.report_ref, run.report_ref);
    }

    #[test]
    fn sealed_repro_bundle_export_rejects_sensitive_markers() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "secret-payload" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <secret "token">>]>"#,
        )
        .expect("parse secret suite");
        let run = run_suite_value(&suite).expect("run secret suite");
        let error = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect_err("secret marker fails redaction preflight");
        assert!(error.to_string().contains("sensitive marker secret"));
    }

    #[test]
    fn redacted_diagnostic_profile_emits_transform_and_stays_diagnostic_only() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "secret-diagnostic" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <secret "token">>]>"#,
        )
        .expect("parse secret suite");
        let run = run_suite_value(&suite).expect("run secret suite");
        let bundle = repro_bundle_value_with_export_profile(
            &run.report_value,
            &["molten".into(), "test".into(), "repro".into(), "export".into()],
            ReproExportProfile::RedactedDiagnostic,
        )
        .expect("redacted diagnostic bundle");
        let parsed = parse_repro_bundle(&bundle).expect("parse redacted diagnostic bundle");
        assert_eq!(parsed.export_profile.as_deref(), Some("redacted-diagnostic"));
        assert_eq!(parsed.loss_classification.as_deref(), Some("diagnostic-only"));
        assert!(parsed.redaction_transform_receipt_ref.is_some());
        assert!(
            to_text(parsed.report_value.as_ref().expect("redacted report"))
                .expect("redacted report text")
                .contains("redaction-marker-v1")
        );
        let gate_error = check_value(&bundle).expect_err("diagnostic bundle cannot satisfy pass gate");
        assert!(gate_error.to_string().contains("diagnostic-only"));
        let verify_error = repro_verify_receipt_value(&bundle).expect_err("diagnostic bundle cannot verify as pass");
        assert!(verify_error.to_string().contains("diagnostic-only"));
    }

    #[test]
    fn encrypted_private_profile_rejects_malformed_encrypted_ref_marker() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "bad-encrypted" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <encrypted-ref "bad">>]>"#,
        )
        .expect("parse malformed encrypted suite");
        let run = run_suite_value(&suite).expect("run malformed encrypted suite");
        let error = repro_bundle_value_with_export_profile(
            &run.report_value,
            &["molten".into(), "test".into(), "repro".into(), "export".into()],
            ReproExportProfile::EncryptedPrivate,
        )
        .expect_err("malformed encrypted refs fail closed");
        assert!(error.to_string().contains("malformed encrypted-ref"));
    }

    #[test]
    fn redacted_profile_rejects_stale_transform_receipt() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "stale-transform" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <secret "token">>]>"#,
        )
        .expect("parse stale transform suite");
        let run = run_suite_value(&suite).expect("run stale transform suite");
        let bundle = repro_bundle_value_with_export_profile(
            &run.report_value,
            &["molten".into(), "test".into(), "repro".into(), "export".into()],
            ReproExportProfile::RedactedDiagnostic,
        )
        .expect("redacted diagnostic bundle");
        let text = to_text(&bundle).expect("bundle text");
        let tampered = parse_text(&text.replacen("output-bundle", "output-bundle-stale", 1))
            .expect("parse stale transform bundle");
        let error = parse_repro_bundle(&tampered).expect_err("stale transform receipt is rejected");
        assert!(error_contains_any(&error, &["output-bundle", "redaction", "expected"]));
    }

    #[test]
    fn redacted_profile_rejects_missed_sensitive_marker() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "missed-marker" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <secret "token">>]>"#,
        )
        .expect("parse missed marker suite");
        let run = run_suite_value(&suite).expect("run missed marker suite");
        let bundle = repro_bundle_value_with_export_profile(
            &run.report_value,
            &["molten".into(), "test".into(), "repro".into(), "export".into()],
            ReproExportProfile::RedactedDiagnostic,
        )
        .expect("redacted diagnostic bundle");
        let text = to_text(&bundle).expect("bundle text");
        let tampered =
            parse_text(&text.replacen("redaction-marker-v1", "secret", 1)).expect("parse missed-marker bundle");
        let error = parse_repro_bundle(&tampered).expect_err("missed marker is rejected");
        assert!(
            error_contains_any(&error, &[
                "missed sensitive marker",
                "redaction",
                "secret",
                "schema",
                "suite value"
            ]),
            "unexpected missed marker error: {error}"
        );
    }

    #[test]
    fn sealed_repro_bundle_rejects_tampered_redaction_gate() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let bundle = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect("sealed bundle report");
        let bundle_text = to_text(&bundle).expect("render sealed bundle");
        let tampered_text = bundle_text.replacen("redaction-gate-v1", "redaction-gate-v0", 1);
        let tampered_bundle = parse_text(&tampered_text).expect("parse tampered redaction bundle");
        let error = check_value(&tampered_bundle).expect_err("tampered redaction gate fails sealed gate");
        assert!(error_contains_any(&error, &["redaction-gate", "redaction gate"]));
    }

    #[test]
    fn sealed_repro_bundle_rejects_tampered_embedded_report() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let bundle = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect("sealed bundle report");
        let bundle_text = to_text(&bundle).expect("render sealed bundle");
        let report_start = bundle_text.find("<harness-report-v1").expect("embedded report");
        let final_hash_start =
            report_start + bundle_text[report_start..].find(&run.final_state_hash).expect("embedded report final hash");
        let tampered_text = format!(
            "{}{}{}",
            &bundle_text[..final_hash_start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &bundle_text[final_hash_start + run.final_state_hash.len()..]
        );
        let tampered_bundle = parse_text(&tampered_text).expect("parse tampered sealed bundle");
        let error = check_value(&tampered_bundle).expect_err("tampered embedded report fails sealed gate");
        assert!(error_contains_any(&error, &[
            "repro bundle report ref mismatch",
            "repro bundle state refs do not match embedded report",
            "sealed repro bundle embedded gate receipt does not match report",
        ]));
    }

    #[test]
    fn sealed_repro_bundle_rejects_tampered_embedded_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let bundle = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect("sealed bundle report");
        let bundle_text = to_text(&bundle).expect("render sealed bundle");
        let receipt_start = bundle_text.find("<gate-receipt-v1").expect("embedded receipt");
        let decision_start =
            receipt_start + bundle_text[receipt_start..].find("<decision \"pass\">").expect("receipt decision");
        let tampered_text = format!(
            "{}<decision \"fail\">{}",
            &bundle_text[..decision_start],
            &bundle_text[decision_start + "<decision \"pass\">".len()..]
        );
        let tampered_bundle = parse_text(&tampered_text).expect("parse tampered receipt bundle");
        let error = check_value(&tampered_bundle).expect_err("tampered embedded receipt fails sealed gate");
        assert!(error_contains_any(&error, &[
            "sealed repro bundle gate receipt ref mismatch",
            "unsupported gate receipt decision",
        ]));
    }

    #[test]
    fn sealed_repro_bundle_rejects_mismatched_suite_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let bundle = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect("sealed bundle report");
        let bundle_text = to_text(&bundle).expect("render sealed bundle");
        let tampered_text = bundle_text.replacen(
            &canonical_hash(&suite).expect("suite hash"),
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            1,
        );
        let tampered_bundle = parse_text(&tampered_text).expect("parse mismatched suite bundle");
        let error = check_value(&tampered_bundle).expect_err("mismatched suite ref fails sealed gate");
        assert!(error_contains_any(&error, &[
            "artifact refs missing suite ref",
            "repro bundle suite ref does not match embedded report",
            "repro seal suite ref does not match bundle suite ref",
        ]));
    }

    #[test]
    fn gate_receipt_is_canonical_pass_decision_artifact() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let check = check_value(&run.report_value).expect("gate accepts report");
        let receipt_value = receipt_value(&check);
        let receipt = parse_receipt(&receipt_value).expect("parse gate receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.artifact_kind, "report");
        assert_eq!(receipt.report_ref, run.report_ref);
        assert_eq!(receipt.suite_ref, check.suite_ref);
        assert_expected_gate_checks(&receipt.checks);
        let parsed_report = parse_report(&run.report_value).expect("parse report");
        let runtime_predicates = parsed_report
            .observations
            .iter()
            .flat_map(|observation| observation.events.iter())
            .filter(|event| event.collect_simple_record("runtime-predicate-receipt-v1", None).is_some())
            .count();
        assert!(runtime_predicates >= 3);
        assert!(receipt_summary(&receipt_value).expect("receipt summary").contains("decision=pass"));
        let rendered = to_text(&receipt_value).expect("render receipt");
        let reparsed = parse_text(&rendered).expect("reparse receipt");
        assert_eq!(canonical_hash(&receipt_value).unwrap(), canonical_hash(&reparsed).unwrap());
    }
