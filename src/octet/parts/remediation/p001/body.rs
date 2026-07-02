
fn run_metrics(artifacts: &RunArtifacts, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<RunMetrics> {
    let status: StatusArtifact = serde_json::from_str(&artifacts.status.text)
        .map_err(|error| MoltenError::invalid_harness(format!("malformed {}: {error}", artifacts.status.name)))?;
    let summary = parse_summary_metrics(&artifacts.summary.text)?;
    if summary.parsed_findings != status.total_findings {
        push_diagnostic(
            diagnostics,
            format!(
                "{} summary parsed {} indexed findings but status reports {}",
                artifacts.scope, summary.parsed_findings, status.total_findings
            ),
        );
    }
    Ok(RunMetrics {
        scope: artifacts.scope.clone(),
        status_ref: artifacts.status.content_ref.clone(),
        summary_ref: artifacts.summary.content_ref.clone(),
        object_corpus_ref: artifacts.object_corpus.as_ref().map(|artifact| artifact.content_ref.clone()),
        status,
        by_crate: sorted_metric_entries(summary.by_crate, MAX_METRIC_ROWS),
        by_effect: sorted_metric_entries(summary.by_effect, MAX_METRIC_ROWS),
        by_lint: sorted_metric_entries(summary.by_lint, MAX_METRIC_ROWS),
        top_project_files: sorted_metric_entries(summary.by_file, MAX_TOP_FILES),
        findings: summary.findings,
    })
}

fn read_focused_object_corpus(
    input: &PlanInput,
    workspace: &RunArtifacts,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<Option<ObjectCorpusMetrics>> {
    let artifact = match input.focused_object_corpus.as_ref() {
        Some(path) => Some(read_artifact_text(path.clone(), OBJECT_CORPUS_RECEIPT_NAME)?),
        None => workspace.object_corpus.clone(),
    };
    let Some(artifact) = artifact else {
        push_diagnostic(diagnostics, "octet remediation has no focused object corpus artifact".to_string());
        return Ok(None);
    };
    let parsed: ObjectCorpusReceipt = serde_json::from_str(&artifact.text)
        .map_err(|error| MoltenError::invalid_harness(format!("malformed object corpus receipt: {error}")))?;
    let object_count = parsed
        .object_count
        .ok_or_else(|| MoltenError::invalid_harness("object corpus receipt missing object_count"))?;
    let pure_cache_blocked_count = parsed
        .pure_cache_blocked_count
        .ok_or_else(|| MoltenError::invalid_harness("object corpus receipt missing pure_cache_blocked_count"))?;
    let source_paths = parsed
        .source_paths
        .ok_or_else(|| MoltenError::invalid_harness("object corpus receipt missing source_paths"))?;
    Ok(Some(ObjectCorpusMetrics {
        content_ref: artifact.content_ref,
        object_set_hash: parsed.object_set_hash,
        object_count,
        pure_cache_blocked_count,
        source_paths,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryMetrics {
    by_crate: Map<String, u64>,
    by_effect: Map<String, u64>,
    by_lint: Map<String, u64>,
    by_file: Map<String, u64>,
    findings: Vec<FindingIndexEntry>,
    parsed_findings: u64,
}

fn parse_summary_metrics(text: &str) -> Result<SummaryMetrics> {
    let by_crate = parse_count_section(text, "By crate:")?;
    let by_effect = parse_count_section(text, "By effect:")?;
    let by_lint = parse_count_section(text, "By lint:")?;
    let findings = parse_index_findings(text)?;
    let parsed_findings = findings.iter().map(|finding| finding.count).sum::<u64>();
    let mut by_file = Map::new();
    for finding in &findings {
        if finding.file.starts_with("src/") {
            increment_count(&mut by_file, &finding.file, finding.count);
        }
    }
    Ok(SummaryMetrics {
        by_crate,
        by_effect,
        by_lint,
        by_file,
        findings,
        parsed_findings,
    })
}

fn parse_count_section(text: &str, header: &str) -> Result<Map<String, u64>> {
    let mut values = Map::new();
    let mut is_parsing_section = false;
    for (line_index, line) in text.lines().enumerate() {
        if line_index >= MAX_SUMMARY_LINES {
            return Err(MoltenError::invalid_harness("octet summary exceeds line bound"));
        }
        let trimmed = line.trim();
        if trimmed == header {
            is_parsing_section = true;
            continue;
        }
        if is_parsing_section && (trimmed.is_empty() || trimmed.ends_with(':')) {
            break;
        }
        if !is_parsing_section {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(count_text) = parts.next() else { continue };
        if let Ok(count) = count_text.parse::<u64>() {
            insert_count_bounded(&mut values, name, count, MAX_METRIC_ROWS, header)?;
        }
    }
    Ok(values)
}

fn parse_index_findings(text: &str) -> Result<Vec<FindingIndexEntry>> {
    let mut by_key: Map<(String, String, String, String), u64> = Map::new();
    let mut is_parsing_index = false;
    let mut parsed_rows = 0usize;
    for (line_index, line) in text.lines().enumerate() {
        if line_index >= MAX_SUMMARY_LINES {
            return Err(MoltenError::invalid_harness("octet summary exceeds line bound"));
        }
        let trimmed = line.trim();
        if trimmed == "Index:" {
            is_parsing_index = true;
            continue;
        }
        if !is_parsing_index || trimmed.is_empty() {
            continue;
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 4 || !parts[0].starts_with('F') {
            continue;
        }
        parsed_rows = parsed_rows.saturating_add(1);
        if parsed_rows > MAX_INDEX_FINDINGS {
            return Err(MoltenError::invalid_harness("octet finding index exceeds row bound"));
        }
        let lint = parts[1].to_string();
        let crate_name = parts[2].to_string();
        let location = parts[3].to_string();
        let file = location_file(&location);
        let key = (lint, crate_name, location, file);
        *by_key.entry(key).or_insert(0) += 1;
    }
    let mut findings = by_key
        .into_iter()
        .map(|((lint, crate_name, location, file), count)| FindingIndexEntry {
            lint,
            crate_name,
            location,
            file,
            count,
        })
        .collect::<Vec<_>>();
    findings.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then(left.file.cmp(&right.file))
            .then(left.lint.cmp(&right.lint))
            .then(left.location.cmp(&right.location))
    });
    Ok(findings)
}

fn location_file(location: &str) -> String {
    let Some((file, line_text)) = location.rsplit_once(':') else {
        return location.to_string();
    };
    if line_text.chars().all(|ch| ch.is_ascii_digit()) {
        return file.to_string();
    }
    location.to_string()
}

fn classify_source_scope_findings(
    metrics: &RunMetrics,
    focused_object_corpus: Option<&ObjectCorpusMetrics>,
) -> Result<Vec<SourceScopeClassification>> {
    let source_inventory = source_path_inventory(focused_object_corpus)?;
    let mut classifications = Vec::new();
    for finding in &metrics.findings {
        if !is_source_scope_lint(&finding.lint) {
            continue;
        }
        push_bounded(
            &mut classifications,
            classify_source_scope_finding(finding, &source_inventory),
            MAX_SOURCE_SCOPE_CLASSIFICATIONS,
            "source-scope classifications",
        )?;
    }
    Ok(classifications)
}

fn source_path_inventory(
    focused_object_corpus: Option<&ObjectCorpusMetrics>,
) -> Result<std::collections::BTreeSet<String>> {
    let mut inventory = std::collections::BTreeSet::new();
    if let Some(corpus) = focused_object_corpus {
        for path in &corpus.source_paths {
            insert_source_path_bounded(&mut inventory, path.clone())?;
        }
    }
    Ok(inventory)
}

fn classify_source_scope_finding(
    finding: &FindingIndexEntry,
    source_inventory: &std::collections::BTreeSet<String>,
) -> SourceScopeClassification {
    let normalized_file = workspace_relative_path(&finding.file);
    let (classification, decision, evidence) = source_scope_decision(&finding.file, normalized_file, source_inventory);
    SourceScopeClassification {
        lint: finding.lint.clone(),
        crate_name: finding.crate_name.clone(),
        location: finding.location.clone(),
        file: finding.file.clone(),
        count: finding.count,
        classification,
        decision,
        evidence,
    }
}

fn is_source_scope_lint(lint: &str) -> bool {
    lint == MODULE_FILE_COUNT_LINT || lint == UNDERSCORE_MODULE_LINT
}

fn workspace_relative_path(file: &str) -> Option<&str> {
    file.strip_prefix(WORKSPACE_PATH_PREFIX)
}

fn source_scope_decision(
    file: &str,
    normalized_file: Option<&str>,
    source_inventory: &std::collections::BTreeSet<String>,
) -> (&'static str, &'static str, String) {
    if inventory_contains(source_inventory, file, normalized_file) {
        return (
            CLASS_MOLTEN_OWNED_SOURCE,
            DECISION_ACTIONABLE,
            "reported path is present in the focused Molten source inventory".to_string(),
        );
    }
    if is_integration_test_path(file, normalized_file) {
        return (
            CLASS_INTEGRATION_TEST_SOURCE,
            DECISION_ACTIONABLE,
            "reported path is an integration-test source path".to_string(),
        );
    }
    if is_registry_or_rustlib_path(file) {
        return (
            CLASS_REGISTRY_RUSTLIB_SOURCE,
            DECISION_IGNORE_EXTERNAL,
            "reported path is under a registry or rustlib source root".to_string(),
        );
    }
    if normalized_file.is_some_and(|path| path.starts_with(SOURCE_PATH_PREFIX)) {
        return (
            CLASS_GENERATED_REMAP_SOURCE,
            DECISION_IGNORE_EXTERNAL,
            "reported <WORKSPACE>/src path is absent from the focused Molten source inventory".to_string(),
        );
    }
    (
        CLASS_UNKNOWN_SOURCE,
        DECISION_BLOCKED,
        "reported path is not in the focused inventory and does not match an external source pattern".to_string(),
    )
}

fn inventory_contains(
    source_inventory: &std::collections::BTreeSet<String>,
    file: &str,
    normalized_file: Option<&str>,
) -> bool {
    source_inventory.contains(file) || normalized_file.is_some_and(|path| source_inventory.contains(path))
}

fn is_integration_test_path(file: &str, normalized_file: Option<&str>) -> bool {
    file.starts_with(TEST_PATH_PREFIX) || normalized_file.is_some_and(|path| path.starts_with(TEST_PATH_PREFIX))
}

fn is_registry_or_rustlib_path(file: &str) -> bool {
    file.contains(CARGO_REGISTRY_MARKER) || file.starts_with(RUSTC_SOURCE_PREFIX)
}
