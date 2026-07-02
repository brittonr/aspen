
fn parse_review_manifest(value: &IoValue) -> Result<ParsedReviewManifest> {
    let fields = value
        .collect_simple_record("octet-review-manifest-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <octet-review-manifest-v1 ...>"))?;
    require_schema(&fields[0], OCTET_REVIEW_MANIFEST_SCHEMA, "octet review manifest")?;
    Ok(ParsedReviewManifest {
        review_ref: canonical_hash(value)?,
        profile: record_string(&fields[1], "profile")?,
        expires_at: record_string(&fields[2], "expires-at")?,
        finding_keys: record_string_sequence(&fields[3], "finding-keys")?,
    })
}

fn finding_is_reviewed(finding: &FindingEntry, reviews: &[ParsedReviewManifest], profile: &str, as_of: &str) -> bool {
    reviews.iter().any(|review| {
        review.profile == profile
            && review.expires_at.as_str() >= as_of
            && review.finding_keys.iter().any(|key| key == &finding.key)
    })
}

fn parse_warning_baseline(value: &IoValue) -> Result<ParsedWarningBaseline> {
    let fields = value
        .collect_simple_record("octet-warning-baseline-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <octet-warning-baseline-v1 ...>"))?;
    require_schema(&fields[0], OCTET_WARNING_BASELINE_SCHEMA, "octet warning baseline")?;
    let findings = record_finding_entries(&fields[8], "finding-keys")?;
    let _critical_keys = record_string_sequence(&fields[9], "critical-finding-keys")?;
    let allowed_profiles = record_string_sequence(&fields[10], "allowed-profiles")?;
    let burn_down = value_to_iovalue(&fields[11]);
    let burn_down_fields = burn_down
        .collect_simple_record("burn-down", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected baseline burn-down record"))?;
    Ok(ParsedWarningBaseline {
        baseline_ref: canonical_hash(value)?,
        expires_at: record_string(&fields[3], "expires-at")?,
        config_hash: record_string(&fields[4], "octet-config-hash")?,
        profile_hash: record_string(&fields[5], "octet-profile-hash")?,
        findings,
        allowed_profiles,
        target_next: record_u64(&burn_down_fields[1], "target-next")?,
        review_refs: record_string_sequence(&fields[12], "review-refs")?,
    })
}

fn octet_structured_findings_value(
    status_file: &GateFile,
    summary: &GateFile,
    status: &StatusArtifact,
) -> (IoValue, u64) {
    let (findings, parsed_count) = parse_summary_findings(summary, status);
    let unkeyed_findings = status.total_findings.saturating_sub(parsed_count);
    let critical_count = findings.values().filter(|finding| is_critical_lint(&finding.lint)).count() as u64;
    let keyed_status = if unkeyed_findings == 0 { "pass" } else { "fail" };
    let value = record("octet-structured-findings-v1", vec![
        string(crate::preserves_rail::OCTET_STRUCTURED_FINDINGS_SCHEMA),
        record("status", vec![string(&status_file.artifact_ref)]),
        record("summary", vec![string(&summary.artifact_ref)]),
        record("metadata", vec![
            record("config-hash", vec![string(&status.metadata.config_hash)]),
            record("profile-hash", vec![string(&status.metadata.profile_hash)]),
            record("tool", vec![string(format!(
                "{}@{}",
                status.metadata.tool_name, status.metadata.tool_version
            ))]),
        ]),
        record("counts", vec![
            record("total", vec![u64_value(status.total_findings)]),
            record("parsed", vec![u64_value(parsed_count)]),
            record("unkeyed", vec![u64_value(unkeyed_findings)]),
            record("critical", vec![u64_value(critical_count)]),
        ]),
        record("finding-keys", vec![sequence(findings.values().map(finding_entry_value).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("summary-index-stable-keys"), string(keyed_status)]),
            record("check", vec![string("artifact-ref-binding"), string("pass")]),
        ])]),
    ]);
    (value, unkeyed_findings)
}

fn parse_summary_findings(summary: &GateFile, status: &StatusArtifact) -> (OrderedMap<String, FindingEntry>, u64) {
    let mut findings = OrderedMap::new();
    let mut parsed_count = 0u64;
    let mut is_parsing_index = false;
    for line in summary.text.lines() {
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
        parsed_count = parsed_count.saturating_add(1);
        let lint = parts[1].to_string();
        let crate_name = parts[2].to_string();
        let location = parts[3].to_string();
        let key =
            finding_key(&lint, &crate_name, &location, &status.metadata.config_hash, &status.metadata.tool_version);
        findings
            .entry(key.clone())
            .and_modify(|entry: &mut FindingEntry| entry.count = entry.count.saturating_add(1))
            .or_insert(FindingEntry {
                key,
                lint,
                crate_name,
                location,
                count: 1,
            });
    }
    (findings, parsed_count)
}

fn finding_key(lint: &str, crate_name: &str, location: &str, config_hash: &str, tool_version: &str) -> String {
    b3_full_hash(&format!(
        "lint={lint}\ncrate={crate_name}\nlocation={location}\nconfig={config_hash}\ntool=cargo-octet@{tool_version}\n"
    ))
}

fn finding_entry_value(finding: &FindingEntry) -> IoValue {
    record("finding-key", vec![
        string(&finding.key),
        string(&finding.lint),
        string(&finding.crate_name),
        string(&finding.location),
        u64_value(finding.count),
    ])
}

fn finding_count_delta(
    current: &OrderedMap<String, FindingEntry>,
    baseline: &OrderedMap<String, FindingEntry>,
    kind: DeltaKind,
) -> Vec<FindingEntry> {
    let mut delta = Vec::new();
    match kind {
        DeltaKind::NewOrIncreased => {
            for (key, current_entry) in current {
                let baseline_count = baseline.get(key).map(|entry| entry.count).unwrap_or(0);
                if current_entry.count > baseline_count {
                    let mut entry = current_entry.clone();
                    entry.count -= baseline_count;
                    delta.push(entry);
                }
            }
        }
        DeltaKind::Removed => {
            for (key, baseline_entry) in baseline {
                let current_count = current.get(key).map(|entry| entry.count).unwrap_or(0);
                if baseline_entry.count > current_count {
                    let mut entry = baseline_entry.clone();
                    entry.count -= current_count;
                    delta.push(entry);
                }
            }
        }
    }
    delta
}

fn finding_intersection(
    current: &OrderedMap<String, FindingEntry>,
    baseline: &OrderedMap<String, FindingEntry>,
) -> Vec<FindingEntry> {
    let mut intersection = Vec::new();
    for (key, current_entry) in current {
        if let Some(baseline_entry) = baseline.get(key) {
            let count = current_entry.count.min(baseline_entry.count);
            if count > 0 {
                let mut entry = current_entry.clone();
                entry.count = count;
                if !push_finding_bounded(&mut intersection, entry) {
                    break;
                }
            }
        }
    }
    intersection
}

fn critical_keys(findings: &OrderedMap<String, FindingEntry>) -> Vec<String> {
    findings
        .values()
        .filter(|finding| is_critical_lint(&finding.lint))
        .map(|finding| finding.key.clone())
        .collect()
}

fn is_critical_lint(lint: &str) -> bool {
    CRITICAL_LINTS.iter().any(|critical| critical == &lint)
}

fn source_snapshot_ref(run: &CurrentOctetRun) -> Result<String> {
    canonical_hash(&record("octet-source-snapshot-v1", vec![
        record("status", vec![string(&run.status_ref)]),
        record("summary", vec![string(&run.summary_ref)]),
        record("object-corpus", vec![string(&run.object_corpus_ref)]),
    ]))
}

fn record_finding_entries(value: &Value<IoValue>, label: &str) -> Result<OrderedMap<String, FindingEntry>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    let items = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} sequence")))?;
    ensure_count_at_most(items.len(), MAX_OCTET_FINDING_ENTRIES, label)?;
    let mut findings = OrderedMap::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let fields = item
            .collect_simple_record("finding-key", Some(5))
            .ok_or_else(|| MoltenError::invalid_harness("expected finding-key record"))?;
        let key = required_string(&fields[0], "finding key")?;
        insert_bounded(
            &mut findings,
            key.clone(),
            FindingEntry {
                key,
                lint: required_string(&fields[1], "finding lint")?,
                crate_name: required_string(&fields[2], "finding crate")?,
                location: required_string(&fields[3], "finding location")?,
                count: required_u64(&fields[4], "finding count")?,
            },
            MAX_OCTET_FINDING_ENTRIES,
            label,
        )?;
    }
    Ok(findings)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    required_string(&record[0], label)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    required_u64(&record[0], label)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    let items = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} sequence")))?;
    ensure_count_at_most(items.len(), MAX_OCTET_STRING_SEQUENCE, label)?;
    let mut strings = Vec::with_capacity(items.len());
    for item in items.iter() {
        strings.push(required_string(item, label)?);
    }
    Ok(strings)
}

fn require_schema(value: &Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} schema mismatch: got {actual}, expected {expected}")))
    }
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}
