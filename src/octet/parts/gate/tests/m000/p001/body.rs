
    #[test]
    fn baseline_check_denies_new_warning() {
        let dir = temp_dir("baseline-new-warning");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        write_artifacts(&dir, noncritical_status_json(2), noncritical_summary_two(), object_corpus_json());
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("new or increased")));
    }

    #[test]
    fn baseline_check_allows_removed_warning() {
        let dir = temp_dir("baseline-removed-warning");
        write_artifacts(&dir, noncritical_status_json(2), noncritical_summary_two(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "pass");
        let receipt_text = to_text(&evaluation.receipt_value).expect("receipt text");
        assert!(receipt_text.contains("removed-findings"));
        assert!(receipt_text.contains("bool_naming"));
    }

    #[test]
    fn baseline_check_denies_expired_baseline() {
        let dir = temp_dir("baseline-expired");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "2026-01-01T00:00:00Z")).expect("write baseline");
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("expired")));
    }

    #[test]
    fn baseline_check_denies_unreviewed_critical_warning() {
        let dir = temp_dir("baseline-critical");
        write_artifacts(&dir, critical_status_json(), critical_summary(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("unreviewed critical")));
    }

    #[test]
    fn baseline_check_accepts_exact_review_manifest_for_critical_warning() {
        let dir = temp_dir("baseline-critical-reviewed");
        write_artifacts(&dir, critical_status_json(), critical_summary(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        let parsed = parse_warning_baseline(&baseline.baseline_value).expect("parse baseline");
        let finding_key = parsed.findings.keys().next().expect("finding key").clone();
        let review = build_octet_review_manifest(&OctetReviewManifestInput {
            profile: QUARANTINE_PROFILE.to_string(),
            expires_at: "9999-01-01T00:00:00Z".to_string(),
            finding_keys: vec![finding_key],
            rationale: "temporary test review".to_string(),
        })
        .expect("build review");
        let evaluation = check_octet_warning_baseline(&baseline_check_input_with_reviews(
            &dir,
            &baseline.baseline_value,
            "2026-05-31T00:00:00Z",
            vec![review.review_value],
        ))
        .expect("check baseline");

        assert_eq!(evaluation.decision, "pass");
        let receipt_text = to_text(&evaluation.receipt_value).expect("receipt text");
        assert!(receipt_text.contains("review-refs"));
    }

    fn input(dir: &Path) -> OctetGateInput {
        OctetGateInput {
            artifacts_dir: dir.to_path_buf(),
            profile: STRICT_PROFILE.to_string(),
        }
    }

    fn receipt_findings_ref(value: &IoValue) -> String {
        let fields = value.collect_simple_record("octet-gate-receipt-v1", Some(15)).expect("gate receipt");
        let findings_value = value_to_iovalue(&fields[6]);
        let findings = findings_value.collect_simple_record("findings", Some(1)).expect("findings");
        let some = findings[0].collect_simple_record("some", Some(1)).expect("some findings ref");
        some[0].as_string().expect("findings ref string").into_owned()
    }

    fn baseline_input(dir: &Path, expires_at: &str) -> OctetWarningBaselineInput {
        OctetWarningBaselineInput {
            artifacts_dir: dir.to_path_buf(),
            created_at: "2026-05-31T00:00:00Z".to_string(),
            expires_at: expires_at.to_string(),
            target_next: None,
        }
    }

    fn baseline_check_input(dir: &Path, baseline_value: &IoValue, as_of: &str) -> OctetBaselineCheckInput {
        baseline_check_input_with_reviews(dir, baseline_value, as_of, Vec::new())
    }

    fn baseline_check_input_with_reviews(
        dir: &Path,
        baseline_value: &IoValue,
        as_of: &str,
        review_values: Vec<IoValue>,
    ) -> OctetBaselineCheckInput {
        OctetBaselineCheckInput {
            artifacts_dir: dir.to_path_buf(),
            baseline_value: baseline_value.clone(),
            profile: QUARANTINE_PROFILE.to_string(),
            as_of: as_of.to_string(),
            review_values,
        }
    }

    const COMMAND_TEXT: &str = "cargo octet check --artifact-dir target/octet";

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("octet-test-ref", vec![string(label)])).expect("test ref")
    }

    fn write_artifacts(dir: &Path, status: impl AsRef<str>, summary: &str, object_corpus: &str) {
        fs::create_dir_all(dir).expect("create temp dir");
        fs::write(dir.join(COMMAND_NAME), format!("{COMMAND_TEXT}\n")).expect("write command");
        fs::write(dir.join(STATUS_NAME), status.as_ref()).expect("write status");
        fs::write(dir.join(SUMMARY_NAME), summary).expect("write summary");
        fs::write(dir.join(OBJECT_CORPUS_RECEIPT_NAME), object_corpus).expect("write object corpus");
    }

    fn warning_status_json() -> String {
        status_json("warning-only", 3, 3, 0, 1, SUPPORTED_OCTET_TOOL_VERSION, current_metadata())
    }

    fn clean_status_json() -> String {
        status_json("clean", 0, 0, 0, 0, SUPPORTED_OCTET_TOOL_VERSION, current_metadata())
    }

    fn noncritical_status_json(total: u64) -> String {
        status_json("warning-only", total, total, 0, 0, SUPPORTED_OCTET_TOOL_VERSION, current_metadata())
    }

    fn critical_status_json() -> String {
        status_json("warning-only", 1, 1, 0, 0, SUPPORTED_OCTET_TOOL_VERSION, current_metadata())
    }

    fn stale_status_json() -> String {
        status_json("clean", 0, 0, 0, 0, SUPPORTED_OCTET_TOOL_VERSION, ExpectedMetadata {
            config_hash: "b3:stale-config".to_string(),
            profile_hash: "b3:stale-profile".to_string(),
        })
    }

    fn unsupported_tool_version_status_json() -> String {
        status_json("clean", 0, 0, 0, 0, "9.9.9", current_metadata())
    }

    fn current_metadata() -> ExpectedMetadata {
        expected_metadata_for_command(COMMAND_TEXT).expect("compute current octet metadata")
    }

    fn status_json(
        status: &str,
        total_findings: u64,
        warning_findings: u64,
        error_findings: u64,
        autofixable_findings: u64,
        tool_version: &str,
        metadata: ExpectedMetadata,
    ) -> String {
        format!(
            r#"{{
  "status": "{status}",
  "exit_code": 0,
  "output_format": "human",
  "metadata": {{
    "tool_name": "cargo-octet",
    "tool_version": "{tool_version}",
    "rustc_version": "rustc 1.96.0-nightly",
    "toolchain": "nightly-2026-03-21-x86_64-unknown-linux-gnu",
    "profile_hash": "{}",
    "config_hash": "{}"
  }},
  "total_findings": {total_findings},
  "warning_findings": {warning_findings},
  "error_findings": {error_findings},
  "autofixable_findings": {autofixable_findings},
  "cargo_process_exit": {{"classification": "success", "code": 0}}
}}"#,
            metadata.profile_hash, metadata.config_hash
        )
    }

    fn warning_summary() -> &'static str {
        "--- octet summary ---\nStatus: warning-only\nFindings: 3\nWarnings: 3\nErrors: 0\n\nBy lint:\n  no_unwrap 1\n  function_length 2\n\nIndex:\n"
    }

    fn clean_summary() -> &'static str {
        "--- octet summary ---\nStatus: clean\nFindings: 0\nWarnings: 0\nErrors: 0\n\nBy lint:\n\nIndex:\n"
    }

    fn noncritical_summary_one() -> &'static str {
        "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  function_length 1\n\nIndex:\n  F1 function_length molten src/example.rs:10\n"
    }

    fn noncritical_summary_two() -> &'static str {
        "--- octet summary ---\nStatus: warning-only\nFindings: 2\nWarnings: 2\nErrors: 0\n\nBy lint:\n  function_length 1\n  bool_naming 1\n\nIndex:\n  F1 function_length molten src/example.rs:10\n  F2 bool_naming molten src/example.rs:20\n"
    }

    fn critical_summary() -> &'static str {
        "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  no_unwrap 1\n\nIndex:\n  F1 no_unwrap molten src/example.rs:30\n"
    }

    fn object_corpus_json() -> &'static str {
        r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":3,"source_paths":["src/job/dag.rs","src/main.rs","src/node/runtime.rs"],"object_set_hash":"b3:test-object-set","pure_cache_blocked_count":3}"#
    }

    fn object_corpus_with_replay_inventory_json() -> &'static str {
        r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":1,"source_paths":["src/main.rs"],"object_set_hash":"b3:test-object-set","pure_cache_blocked_count":1,"replay":{"command":"cargo octet object corpus receipt --output RECEIPT.json src/job/dag.rs src/main.rs src/node/runtime.rs"}}"#
    }

    fn object_corpus_without_critical_inventory_json() -> &'static str {
        r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":1,"source_paths":["src/main.rs"],"object_set_hash":"b3:test-object-set","pure_cache_blocked_count":1,"replay":{"command":"cargo octet object corpus receipt --output RECEIPT.json src/main.rs"}}"#
    }

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nanos = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-octet-gate-{label}-{}-{nanos}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        dir
    }
