
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
