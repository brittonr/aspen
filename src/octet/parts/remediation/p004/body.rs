
#[cfg(test)]
mod tests {
    use super::*;

    fn artifact_kind(value: &IoValue) -> &'static str {
        crate::ledger::artifact_kind(value)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    #[test]
    fn remediation_plan_captures_workspace_and_lib_metrics() {
        let workspace = temp_dir("workspace-lib-metrics-workspace");
        let lib = temp_dir("workspace-lib-metrics-lib");
        write_artifacts(&workspace, status_json(2), summary_text(), object_corpus_json()).expect("write workspace");
        write_artifacts(&lib, status_json(2), summary_text(), object_corpus_json()).expect("write lib");

        let plan = build_plan(&PlanInput {
            artifacts_dir: workspace,
            lib_artifacts_dir: Some(lib),
            focused_object_corpus: None,
        })
        .expect("build remediation plan");

        let text = to_text(&plan.value).expect("render plan");
        crate::preserves_rail::validate_content_ref(&plan.plan_ref).expect("plan ref is canonical");
        assert!(text.contains("octet-remediation-plan-v1"));
        assert!(text.contains("source-gate-and-admission"));
        assert!(text.contains("critical-deny-classes"));
        assert_eq!(artifact_kind(&plan.value), "octet-remediation-plan");
    }

    #[test]
    fn remediation_plan_reports_status_index_mismatch() {
        let workspace = temp_dir("status-index-mismatch-workspace");
        write_artifacts(&workspace, status_json(3), summary_text(), object_corpus_json()).expect("write workspace");

        let plan = build_plan(&PlanInput {
            artifacts_dir: workspace,
            lib_artifacts_dir: None,
            focused_object_corpus: None,
        })
        .expect("build remediation plan");

        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic.contains("status reports 3")));
        assert!(to_text(&plan.value).expect("render plan").contains("focused-object-corpus-bound"));
    }

    #[test]
    fn source_scope_classifies_inventory_path_as_actionable() {
        let inventory = source_inventory("src/main.rs");
        let finding = source_finding(MODULE_FILE_COUNT_LINT, "src/main.rs:1");

        let classification = classify_source_scope_finding(&finding, &inventory);

        assert_eq!(classification.classification, CLASS_MOLTEN_OWNED_SOURCE);
        assert_eq!(classification.decision, DECISION_ACTIONABLE);
    }

    #[test]
    fn source_scope_classifies_absent_workspace_source_as_external() {
        let inventory = source_inventory("src/main.rs");
        let finding = source_finding(UNDERSCORE_MODULE_LINT, "<WORKSPACE>/src/address_lookup.rs:1");

        let classification = classify_source_scope_finding(&finding, &inventory);

        assert_eq!(classification.classification, CLASS_GENERATED_REMAP_SOURCE);
        assert_eq!(classification.decision, DECISION_IGNORE_EXTERNAL);
    }

    #[test]
    fn source_scope_blocks_unknown_uninventoried_paths() {
        let inventory = source_inventory("src/main.rs");
        let finding = source_finding(MODULE_FILE_COUNT_LINT, "generated/address_lookup.rs:1");

        let classification = classify_source_scope_finding(&finding, &inventory);

        assert_eq!(classification.classification, CLASS_UNKNOWN_SOURCE);
        assert_eq!(classification.decision, DECISION_BLOCKED);
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        let path = std::env::temp_dir().join(format!("molten-octet-remediation-{name}-{}", std::process::id()));
        let remove_error = match fs::remove_dir_all(&path) {
            Ok(()) => None,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => Some(error.to_string()),
        };
        assert!(remove_error.is_none(), "remove temp dir before test: {remove_error:?}");
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn write_artifacts(dir: &Path, status: String, summary: &'static str, object_corpus: &'static str) -> Result<()> {
        fs::write(dir.join(STATUS_NAME), status)?;
        fs::write(dir.join(SUMMARY_NAME), summary)?;
        fs::write(dir.join(OBJECT_CORPUS_RECEIPT_NAME), object_corpus)?;
        Ok(())
    }

    fn status_json(total_findings: u64) -> String {
        format!(
            r#"{{"status":"warning-only","exit_code":0,"output_format":"human","metadata":{{"tool_name":"cargo-octet","tool_version":"0.1.0","rustc_version":"rustc test","toolchain":"nightly-test","profile_name":"workspace-metadata","profile_hash":"b3:profile","config_hash":"b3:config"}},"total_findings":{total_findings},"warning_findings":{total_findings},"error_findings":0,"autofixable_findings":0}}"#
        )
    }

    fn source_inventory(path: &str) -> std::collections::BTreeSet<String> {
        std::collections::BTreeSet::from([path.to_string()])
    }

    fn source_finding(lint: &str, location: &str) -> FindingIndexEntry {
        FindingIndexEntry {
            lint: lint.to_string(),
            crate_name: "molten".to_string(),
            location: location.to_string(),
            file: location_file(location),
            count: 1,
        }
    }

    fn summary_text() -> &'static str {
        r#"--- octet summary ---
Status: warning-only
Findings: 2
Warnings: 2
Errors: 0
Autofixable: 0

By crate:
  molten             2

By effect:
  failure            1
  resource           1

By lint:
  no_unwrap          1
  unbounded_collection_growth 1

Index:
  F1     no_unwrap                     molten  src/node/runtime.rs:230
  F2     unbounded_collection_growth   molten  src/job/dag.rs:1753
"#
    }

    fn object_corpus_json() -> &'static str {
        r#"{"object_count":2,"source_paths":["src/job/dag.rs","src/main.rs","src/node/runtime.rs"],"object_set_hash":"b3:objects","pure_cache_blocked_count":2}"#
    }
}
