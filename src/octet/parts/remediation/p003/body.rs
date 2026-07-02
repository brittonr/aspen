
fn remediation_plan_value(input: &PlanValueInput<'_>) -> IoValue {
    record("octet-remediation-plan-v1", vec![
        string(crate::preserves_rail::OCTET_REMEDIATION_PLAN_SCHEMA),
        record("workspace", vec![run_metrics_value(input.workspace)]),
        record("lib", vec![optional_run_metrics_value(input.lib_metrics)]),
        record("focused-object-corpus", vec![optional_object_corpus_value(input.focused_object_corpus)]),
        record("critical-surfaces", vec![sequence(input.critical_surfaces.to_vec())]),
        record("source-scope", vec![source_scope_value(input.source_scope_classifications)]),
        record("priority-order", vec![sequence(priority_order_values())]),
        record("no-suppression-policy", vec![no_suppression_policy_value()]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(
            input
                .checks
                .iter()
                .map(|(name, status)| record("check", vec![string(name), string(status)]))
                .collect(),
        )]),
    ])
}

fn run_metrics_value(metrics: &RunMetrics) -> IoValue {
    record("run", vec![
        record("scope", vec![string(&metrics.scope)]),
        record("artifacts", vec![sequence(artifact_ref_values(metrics))]),
        record("status", vec![
            record("state", vec![string(&metrics.status.status)]),
            record("exit-code", vec![string(metrics.status.exit_code.to_string())]),
            record("total", vec![u64_value(metrics.status.total_findings)]),
            record("warnings", vec![u64_value(metrics.status.warning_findings)]),
            record("errors", vec![u64_value(metrics.status.error_findings)]),
            record("autofixable", vec![u64_value(metrics.status.autofixable_findings)]),
        ]),
        record("metadata", vec![
            record("tool", vec![string(&metrics.status.metadata.tool_name)]),
            record("tool-version", vec![string(&metrics.status.metadata.tool_version)]),
            record("rustc", vec![string(&metrics.status.metadata.rustc_version)]),
            record("toolchain", vec![string(&metrics.status.metadata.toolchain)]),
            record("profile-name", vec![string(&metrics.status.metadata.profile_name)]),
            record("profile-hash", vec![string(&metrics.status.metadata.profile_hash)]),
            record("config-hash", vec![string(&metrics.status.metadata.config_hash)]),
        ]),
        record("by-crate", vec![sequence(metric_values(&metrics.by_crate))]),
        record("by-effect", vec![sequence(metric_values(&metrics.by_effect))]),
        record("by-lint", vec![sequence(metric_values(&metrics.by_lint))]),
        record("top-project-files", vec![sequence(metric_values(&metrics.top_project_files))]),
    ])
}

fn optional_run_metrics_value(metrics: Option<&RunMetrics>) -> IoValue {
    match metrics {
        Some(metrics) => record("some", vec![run_metrics_value(metrics)]),
        None => record("none", Vec::new()),
    }
}

fn artifact_ref_values(metrics: &RunMetrics) -> Vec<IoValue> {
    let mut artifacts = vec![
        artifact_ref_value(STATUS_NAME, &metrics.status_ref),
        artifact_ref_value(SUMMARY_NAME, &metrics.summary_ref),
    ];
    if let Some(object_corpus_ref) = metrics.object_corpus_ref.as_ref() {
        artifacts.push(artifact_ref_value(OBJECT_CORPUS_RECEIPT_NAME, object_corpus_ref));
    }
    artifacts
}

fn artifact_ref_value(name: &str, content_ref: &str) -> IoValue {
    record("artifact", vec![
        record("name", vec![string(name)]),
        record("content-ref", vec![string(content_ref)]),
    ])
}

fn optional_object_corpus_value(metrics: Option<&ObjectCorpusMetrics>) -> IoValue {
    match metrics {
        Some(metrics) => record("some", vec![record("object-corpus", vec![
            record("content-ref", vec![string(&metrics.content_ref)]),
            record("object-set-hash", vec![optional_string_value(metrics.object_set_hash.as_deref())]),
            record("object-count", vec![u64_value(metrics.object_count)]),
            record("pure-cache-blocked", vec![u64_value(metrics.pure_cache_blocked_count)]),
            record("source-paths", vec![sequence(metrics.source_paths.iter().map(string).collect())]),
        ])]),
        None => record("none", Vec::new()),
    }
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn source_scope_value(classifications: &[SourceScopeClassification]) -> IoValue {
    record("source-scope-classification-v1", vec![record("classifications", vec![sequence(
        classifications.iter().map(source_scope_classification_value).collect(),
    )])])
}

fn source_scope_classification_value(classification: &SourceScopeClassification) -> IoValue {
    record("classification", vec![
        record("lint", vec![string(&classification.lint)]),
        record("crate", vec![string(&classification.crate_name)]),
        record("location", vec![string(&classification.location)]),
        record("file", vec![string(&classification.file)]),
        record("count", vec![u64_value(classification.count)]),
        record("class", vec![string(classification.classification)]),
        record("decision", vec![string(classification.decision)]),
        record("evidence", vec![string(&classification.evidence)]),
    ])
}

fn priority_order_values() -> Vec<IoValue> {
    vec![
        priority_value(
            1,
            "critical-deny-classes",
            "remove or review panic, unwrap, ambient time, unbounded loops, and source-gate sentinels before other warning classes",
        ),
        priority_value(
            2,
            "resource-bounds",
            "add explicit bounds for collection growth, queues, reports, and execution loops on evidence paths",
        ),
        priority_value(
            3,
            "api-shape",
            "replace high-arity builders and raw refs with input structs and typed validation helpers",
        ),
        priority_value(
            4,
            "module-splits",
            "split main, job_dag, and node_runtime shells without changing canonical refs",
        ),
        priority_value(
            5,
            "style-autofix",
            "drain import/path/bool naming findings after safety and evidence blockers are under control",
        ),
    ]
}

fn priority_value(rank: u64, name: &str, rationale: &str) -> IoValue {
    record("priority", vec![
        record("rank", vec![u64_value(rank)]),
        record("name", vec![string(name)]),
        record("rationale", vec![string(rationale)]),
    ])
}

fn no_suppression_policy_value() -> IoValue {
    record("policy", vec![
        record("hidden-suppressions", vec![string("deny")]),
        record("retained-warning-requires", vec![string("scheduled-remediation-or-reviewed-quarantine-receipt")]),
        record("strict-gate-warning-only", vec![string("deny")]),
        record("quarantine", vec![string("explicit-expiring-reviewed-critical-findings-only")]),
    ])
}

fn plan_checks(
    workspace: &RunMetrics,
    lib_metrics: Option<&RunMetrics>,
    focused_object_corpus: Option<&ObjectCorpusMetrics>,
    source_scope_classifications: &[SourceScopeClassification],
) -> Vec<(&'static str, &'static str)> {
    vec![
        ("workspace-status-bound", pass_fail(!workspace.status_ref.is_empty())),
        ("workspace-summary-bound", pass_fail(!workspace.summary_ref.is_empty())),
        ("workspace-metrics-captured", pass_fail(metrics_present_or_clean(workspace))),
        ("lib-metrics-captured", pass_fail(lib_metrics.is_some_and(metrics_present_or_clean))),
        ("focused-object-corpus-bound", pass_fail(focused_object_corpus.is_some())),
        (
            "source-scope-classified",
            pass_fail(source_scope_is_classified(workspace, focused_object_corpus, source_scope_classifications)),
        ),
        (
            "unknown-source-scope-blocked",
            pass_fail(unknown_source_scope_is_blocked(source_scope_classifications)),
        ),
        ("critical-surface-inventory", "pass"),
        ("priority-order-defined", "pass"),
        ("no-suppression-policy", "pass"),
    ]
}

fn source_scope_is_classified(
    workspace: &RunMetrics,
    focused_object_corpus: Option<&ObjectCorpusMetrics>,
    source_scope_classifications: &[SourceScopeClassification],
) -> bool {
    let source_scope_finding_count =
        workspace.findings.iter().filter(|finding| is_source_scope_lint(&finding.lint)).count();
    if source_scope_finding_count == 0 {
        return true;
    }
    focused_object_corpus.is_some() && source_scope_finding_count == source_scope_classifications.len()
}

fn unknown_source_scope_is_blocked(source_scope_classifications: &[SourceScopeClassification]) -> bool {
    source_scope_classifications.iter().all(|classification| {
        classification.classification != CLASS_UNKNOWN_SOURCE || classification.decision == DECISION_BLOCKED
    })
}

fn pass_fail(pass: bool) -> &'static str {
    if pass { "pass" } else { "fail" }
}

fn metrics_present_or_clean(metrics: &RunMetrics) -> bool {
    metrics.status.total_findings == 0 || !metrics.by_lint.is_empty()
}

fn metric_values(entries: &[MetricEntry]) -> Vec<IoValue> {
    entries
        .iter()
        .map(|entry| record("metric", vec![string(&entry.name), u64_value(entry.count)]))
        .collect()
}

fn sorted_metric_entries(counts: Map<String, u64>, limit: usize) -> Vec<MetricEntry> {
    let mut entries = counts.into_iter().map(|(name, count)| MetricEntry { name, count }).collect::<Vec<_>>();
    entries.sort_by(|left, right| right.count.cmp(&left.count).then(left.name.cmp(&right.name)));
    entries.truncate(limit);
    entries
}

fn insert_count_bounded(
    counts: &mut Map<String, u64>,
    name: &str,
    count: u64,
    limit: usize,
    label: &str,
) -> Result<()> {
    if !counts.contains_key(name) && counts.len() >= limit {
        return Err(MoltenError::invalid_harness(format!("{label} count section exceeds row bound")));
    }
    counts.insert(name.to_string(), count);
    Ok(())
}

fn increment_count(counts: &mut Map<String, u64>, name: &str, count: u64) {
    let current = counts.entry(name.to_string()).or_insert(0);
    *current = current.saturating_add(count);
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, limit: usize, label: &str) -> Result<()> {
    if values.item_count() >= limit {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds item bound")));
    }
    values.push_item(value);
    Ok(())
}

fn critical_count(by_lint: &Map<String, u64>) -> u64 {
    CRITICAL_LINTS.iter().filter_map(|lint| by_lint.get(*lint)).copied().sum()
}

fn push_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: String) {
    if diagnostics.item_count() < MAX_DIAGNOSTICS {
        diagnostics.push_item(diagnostic);
    }
}

fn bytes_ref(bytes: &[u8]) -> String {
    content_ref_from_bytes(bytes)
}
