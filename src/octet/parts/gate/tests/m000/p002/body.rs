    #[test]
    fn object_corpus_with_malformed_object_set_hash_denies() {
        let dir = temp_dir("malformed-object-set-hash");
        write_artifacts(
            &dir,
            clean_status_json(),
            clean_summary(),
            object_corpus_with_malformed_object_set_hash_json(),
        );
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("object_set_hash fingerprint")));
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
