    use super::*;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    #[test]
    fn strict_profile_denies_warning_only_status() {
        let dir = temp_dir("warning-only");
        write_artifacts(&dir, warning_status_json(), warning_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("warning-only")));
        assert!(to_text(&evaluation.receipt_value).expect("receipt text").contains("octet-gate-receipt-v1"));
    }

    #[test]
    fn strict_profile_passes_clean_status_with_required_artifacts() {
        let dir = temp_dir("clean");
        write_artifacts(&dir, clean_status_json(), clean_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "pass");
        assert!(evaluation.diagnostics.is_empty());
        assert!(to_text(&evaluation.receipt_value).expect("receipt text").contains("fingerprint <some \"blake3:"));
    }

    #[test]
    fn strict_profile_accepts_replay_inventory_for_shell_critical_paths() {
        let dir = temp_dir("clean-replay-critical-paths");
        write_artifacts(&dir, clean_status_json(), clean_summary(), object_corpus_with_replay_inventory_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "pass");
        assert!(evaluation.diagnostics.is_empty());
        assert!(to_text(&evaluation.receipt_value).expect("receipt text").contains("fingerprint <some \"blake3:"));
    }

    #[test]
    fn strict_profile_denies_missing_replay_inventory_critical_paths() {
        let dir = temp_dir("missing-replay-critical-paths");
        write_artifacts(&dir, clean_status_json(), clean_summary(), object_corpus_without_critical_inventory_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("required critical paths")));
    }

    #[test]
    fn source_gate_validation_accepts_clean_strict_receipt() {
        let gate = synthetic_clean_octet_gate_receipt_for_tests().expect("clean gate fixture");
        let validation = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "node-startup".to_string(),
            subject_ref: test_ref("node-config"),
            receipt_value: Some(gate),
            source_scope: Vec::new(),
        })
        .expect("validate source gate");

        assert_eq!(validation.decision, "pass");
        assert!(to_text(&validation.value).expect("validation text").contains("octet-source-gate-validation-v1"));
        assert_eq!(crate::ledger::artifact_kind(&validation.value), "octet-source-gate-validation");
    }

    #[test]
    fn source_gate_validation_denies_warning_or_missing_receipts() {
        let dir = temp_dir("source-gate-warning");
        write_artifacts(&dir, warning_status_json(), warning_summary(), object_corpus_json());
        let gate = evaluate_octet_gate(&input(&dir)).expect("warning gate");
        let denied = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "job-remote-admission".to_string(),
            subject_ref: test_ref("job"),
            receipt_value: Some(gate.receipt_value),
            source_scope: Vec::new(),
        })
        .expect("validate warning gate");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("decision is deny")));

        let missing = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "upgrade-plan".to_string(),
            subject_ref: test_ref("plan"),
            receipt_value: None,
            source_scope: Vec::new(),
        })
        .expect("validate missing gate");
        assert_eq!(missing.decision, "deny");
    }

    #[test]
    fn source_gate_validation_denies_stale_or_tampered_receipt() {
        let stale = octet_gate_receipt_value(OctetGateReceiptInput {
            decision: "pass",
            policy_ref: &test_ref("policy"),
            command_ref: Some("blake3:test-command"),
            status_ref: Some("blake3:test-status"),
            summary_ref: Some("blake3:test-summary"),
            structured_findings_ref: Some("blake3:test-findings"),
            object_corpus_ref: Some("blake3:test-object-corpus"),
            fingerprint_evidence_ref: Some("blake3:test-fingerprint"),
            config_hash: Some("b3:stale-config"),
            profile_hash: Some("b3:stale-profile"),
            toolchain: Some("toolchain"),
            counts: &FindingCounts::default(),
            diagnostics: &[],
            checks: &[
                Check {
                    name: "profile-supported",
                    status: "pass",
                },
                Check {
                    name: "strict-status-clean",
                    status: "pass",
                },
                Check {
                    name: "no-critical-findings",
                    status: "pass",
                },
                Check {
                    name: "object-corpus-critical-paths",
                    status: "pass",
                },
                Check {
                    name: "object-corpus-fingerprint",
                    status: "pass",
                },
                Check {
                    name: "fingerprint-evidence-bound",
                    status: "pass",
                },
            ],
        });
        let stale_validation = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "node-startup".to_string(),
            subject_ref: test_ref("node"),
            receipt_value: Some(stale),
            source_scope: Vec::new(),
        })
        .expect("validate stale gate");
        assert_eq!(stale_validation.decision, "deny");
        assert!(stale_validation.diagnostics.iter().any(|diagnostic| diagnostic.contains("config hash")));

        let tampered = parse_text(
            &to_text(&synthetic_clean_octet_gate_receipt_for_tests().expect("clean fixture"))
                .expect("clean text")
                .replace("blake3:test-fingerprint", "not-a-ref"),
        )
        .expect("tampered parse");
        let tampered_validation = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "node-startup".to_string(),
            subject_ref: test_ref("node"),
            receipt_value: Some(tampered),
            source_scope: Vec::new(),
        })
        .expect("validate tampered gate");
        assert_eq!(tampered_validation.decision, "deny");
    }

    #[test]
    fn missing_status_denies_with_receipt() {
        let dir = temp_dir("missing-status");
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(dir.join(COMMAND_NAME), "cargo octet check --artifact-dir target/octet\n").expect("write command");
        fs::write(dir.join(SUMMARY_NAME), clean_summary()).expect("write summary");
        fs::write(dir.join(OBJECT_CORPUS_RECEIPT_NAME), object_corpus_json()).expect("write object corpus");

        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains(STATUS_NAME)));
    }

    #[test]
    fn malformed_status_denies_with_receipt() {
        let dir = temp_dir("malformed-status");
        write_artifacts(&dir, "{", clean_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("malformed status.json")));
    }

    #[test]
    fn unsupported_tool_version_denies_clean_status() {
        let dir = temp_dir("unsupported-tool-version");
        write_artifacts(&dir, unsupported_tool_version_status_json(), clean_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(
            evaluation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("unsupported cargo-octet version"))
        );
    }

    #[test]
    fn missing_object_corpus_denies_clean_status() {
        let dir = temp_dir("missing-object-corpus");
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(dir.join(COMMAND_NAME), "cargo octet check --artifact-dir target/octet\n").expect("write command");
        fs::write(dir.join(STATUS_NAME), clean_status_json()).expect("write status");
        fs::write(dir.join(SUMMARY_NAME), clean_summary()).expect("write summary");

        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains(OBJECT_CORPUS_RECEIPT_NAME)));
    }

    #[test]
    fn stale_status_metadata_denies_clean_status() {
        let dir = temp_dir("stale-status");
        write_artifacts(&dir, stale_status_json(), clean_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale octet config hash")));
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale octet profile hash")));
    }

    #[test]
    fn malformed_object_corpus_denies() {
        let dir = temp_dir("bad-object-corpus");
        write_artifacts(&dir, clean_status_json(), clean_summary(), r#"{"schema":"wrong"}"#);
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("object corpus")));
    }

    #[test]
    fn object_corpus_without_fingerprint_or_critical_paths_denies() {
        let dir = temp_dir("incomplete-object-corpus");
        write_artifacts(
            &dir,
            clean_status_json(),
            clean_summary(),
            r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":1}"#,
        );
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("object_set_hash fingerprint")));
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("required critical paths")));
    }

    #[test]
    fn noncanonical_command_denies() {
        let dir = temp_dir("bad-command");
        write_artifacts(&dir, clean_status_json(), clean_summary(), object_corpus_json());
        fs::write(dir.join(COMMAND_NAME), "cargo check\n").expect("write bad command");
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("noncanonical")));
    }

    #[test]
    fn imports_raw_octet_artifacts_and_derived_evidence_to_ledger() {
        let dir = temp_dir("artifact-ledger-import");
        let ledger_root = dir.join("ledger");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let imported = import_octet_artifacts_to_ledger(&OctetArtifactLedgerInput {
            artifacts_dir: dir.clone(),
            ledger_root: ledger_root.clone(),
        })
        .expect("import octet artifacts");

        assert_eq!(imported.decision, "pass");
        let kinds = crate::ledger::list_artifacts(&ledger_root)
            .expect("list ledger")
            .into_iter()
            .map(|entry| {
                let value =
                    crate::ledger::read_artifact(&ledger_root, &entry.artifact_ref).expect("read ledger artifact");
                crate::ledger::artifact_kind(&value).to_string()
            })
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "octet-command-artifact"));
        assert!(kinds.iter().any(|kind| kind == "octet-status-artifact"));
        assert!(kinds.iter().any(|kind| kind == "octet-summary-artifact"));
        assert!(kinds.iter().any(|kind| kind == "octet-object-corpus-artifact"));
        assert!(kinds.iter().any(|kind| kind == "octet-structured-findings"));
        assert!(kinds.iter().any(|kind| kind == "octet-fingerprint-evidence"));
    }

    #[test]
    fn gate_receipt_binds_structured_findings_ref_to_summary_index() {
        let dir = temp_dir("structured-findings-ref");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let first = evaluate_octet_gate(&input(&dir)).expect("first gate");
        let changed_summary = "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  function_length 1\n\nIndex:\n  F1 function_length molten src/changed.rs:99\n";
        write_artifacts(&dir, noncritical_status_json(1), changed_summary, object_corpus_json());
        let second = evaluate_octet_gate(&input(&dir)).expect("second gate");

        assert_ne!(receipt_findings_ref(&first.receipt_value), receipt_findings_ref(&second.receipt_value));
    }

    #[test]
    fn baseline_check_passes_identical_noncritical_warning() {
        let dir = temp_dir("baseline-pass");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "pass");
    }
