
fn collect_ticket_paths(
    root: &std::path::Path,
    current: &std::path::Path,
    paths: &mut impl crate::bounded::VecSink<std::path::PathBuf>,
) -> crate::error::Result<()> {
    if !current.exists() {
        return Ok(());
    }
    let mut pending = vec![current.to_path_buf()];
    while let Some(next) = pending.pop() {
        if paths.item_count() >= MAX_TICKET_FILES {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "cluster harness ticket file count exceeds bound {MAX_TICKET_FILES} under {}",
                root.display()
            )));
        }
        if next.is_dir() {
            let entries = std::fs::read_dir(&next)
                .map_err(crate::error::MoltenError::from)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(crate::error::MoltenError::from)?;
            // Push in reverse so the stack keeps the depth-first order of the
            // earlier recursion.
            for entry in entries.into_iter().rev() {
                pending.push(entry.path());
            }
        } else if next.is_file()
            && next.file_name().is_some_and(|name| name.to_string_lossy().to_ascii_lowercase().contains("ticket"))
        {
            paths.push_item(next);
        }
    }
    Ok(())
}

fn push_artifact(
    artifacts: &mut impl crate::bounded::VecSink<PreparedArtifact>,
    relative_path: &str,
    kind: &str,
    value: IoValue,
) -> crate::error::Result<()> {
    let expected_ref = crate::preserves_rail::canonical_hash(&value)?;
    artifacts.push_item(PreparedArtifact {
        entry: molten_core::cluster_harness::RunArtifactIndexEntry {
            relative_path: relative_path.to_string(),
            artifact_kind: kind.to_string(),
            expected_ref,
            format: molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES.to_string(),
        },
        value,
    });
    Ok(())
}

fn write_prepared_artifacts(
    output_directory: &std::path::Path,
    artifacts: &[PreparedArtifact],
) -> crate::error::Result<()> {
    for artifact in artifacts {
        write_preserves_path(&output_directory.join(&artifact.entry.relative_path), &artifact.value)?;
    }
    Ok(())
}

fn append_log_entries(
    output_directory: &std::path::Path,
    mut entries: Vec<molten_core::cluster_harness::RunArtifactIndexEntry>,
    plan: &crate::cluster::ClusterPlan,
) -> crate::error::Result<Vec<molten_core::cluster_harness::RunArtifactIndexEntry>> {
    for phase in ["init", "start", "workflow", "status", "stop"] {
        for node in &plan.nodes {
            let relative_path = format!("logs/{phase}-{}.log", node.path_component);
            let path = output_directory.join(&relative_path);
            if !path.exists() {
                continue;
            }
            let text = std::fs::read_to_string(path).map_err(crate::error::MoltenError::from)?;
            entries.push(molten_core::cluster_harness::RunArtifactIndexEntry {
                relative_path,
                artifact_kind: DIAGNOSTIC_LOG_KIND.to_string(),
                expected_ref: content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text),
                format: molten_core::cluster_harness::ARTIFACT_FORMAT_TEXT.to_string(),
            });
        }
    }
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(entries)
}

fn child_log_ref(
    child: &ChildExecution,
    output_directory: &std::path::Path,
    plan: &crate::cluster::ClusterPlan,
) -> crate::error::Result<String> {
    let component = plan
        .nodes
        .iter()
        .find(|node| node.node_id == child.node_id)
        .map(|node| node.path_component.as_str())
        .ok_or_else(|| {
            crate::error::MoltenError::invalid_harness(format!("missing cluster plan node {}", child.node_id))
        })?;
    let text = std::fs::read_to_string(output_directory.join(format!("logs/{}-{component}.log", child.phase)))
        .map_err(crate::error::MoltenError::from)?;
    Ok(content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text))
}

fn render_run_index(entries: &[molten_core::cluster_harness::RunArtifactIndexEntry]) -> String {
    let mut output = String::from(RUN_INDEX_HEADER);
    output.push('\n');
    for entry in entries {
        output.push_str(&entry.relative_path);
        output.push('\t');
        output.push_str(&entry.artifact_kind);
        output.push('\t');
        output.push_str(&entry.expected_ref);
        output.push('\t');
        output.push_str(&entry.format);
        output.push('\n');
    }
    output
}

pub(super) fn parse_run_index(
    source: &str,
) -> crate::error::Result<Vec<molten_core::cluster_harness::RunArtifactIndexEntry>> {
    let mut lines = source.lines();
    let header = lines
        .next()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("cluster run index is empty"))?;
    if header != RUN_INDEX_HEADER {
        return Err(crate::error::MoltenError::invalid_harness("cluster run index has unsupported header"));
    }
    let mut entries = Vec::new();
    for (line_index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != RUN_INDEX_FIELD_COUNT {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "cluster run index line {} must have four tab-separated fields",
                line_index.saturating_add(RUN_INDEX_ENTRY_LINE_OFFSET)
            )));
        }
        crate::bounded::push_bounded(
            &mut entries,
            molten_core::cluster_harness::RunArtifactIndexEntry {
                relative_path: fields[0].to_string(),
                artifact_kind: fields[1].to_string(),
                expected_ref: fields[RUN_INDEX_REF_FIELD].to_string(),
                format: fields[RUN_INDEX_FORMAT_FIELD].to_string(),
            },
            MAX_RUN_INDEX_ENTRIES,
            "cluster run index entry",
        )?;
    }
    Ok(entries)
}

fn assess_indexed_run_directory(
    run_directory: &std::path::Path,
    entries: &[molten_core::cluster_harness::RunArtifactIndexEntry],
) -> molten_core::cluster_harness::RunDirectoryAssessment {
    let mut observations =
        entries.iter().map(|entry| observe_indexed_artifact(run_directory, entry)).collect::<Vec<_>>();
    let indexed_paths =
        entries.iter().map(|entry| entry.relative_path.as_str()).collect::<std::collections::BTreeSet<_>>();
    if let Ok(files) = collect_run_files(run_directory) {
        for relative_path in files {
            if indexed_paths.contains(relative_path.as_str()) || allowed_unindexed_file(&relative_path) {
                continue;
            }
            observations.push(observe_unexpected_artifact(run_directory, &relative_path));
        }
    }
    molten_core::cluster_harness::assess_run_directory(entries, &observations)
}

fn observe_indexed_artifact(
    run_directory: &std::path::Path,
    entry: &molten_core::cluster_harness::RunArtifactIndexEntry,
) -> molten_core::cluster_harness::RunArtifactObservation {
    let path = run_directory.join(&entry.relative_path);
    match entry.format.as_str() {
        molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES => observe_preserves_artifact(&path, entry),
        molten_core::cluster_harness::ARTIFACT_FORMAT_TEXT => observe_text_artifact(&path, entry),
        _ => missing_observation(entry),
    }
}

fn observe_preserves_artifact(
    path: &std::path::Path,
    entry: &molten_core::cluster_harness::RunArtifactIndexEntry,
) -> molten_core::cluster_harness::RunArtifactObservation {
    if !is_regular_file_without_symlink(path) {
        return missing_observation(entry);
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return missing_observation(entry);
    };
    let Ok(value) = crate::preserves_rail::parse_text(&text) else {
        return molten_core::cluster_harness::RunArtifactObservation {
            relative_path: entry.relative_path.clone(),
            artifact_kind: entry.artifact_kind.clone(),
            observed_ref: None,
            format: molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES.to_string(),
            canonical: false,
            pass_eligible: false,
        };
    };
    let observed_ref = crate::preserves_rail::canonical_hash(&value).ok();
    let actual_kind = crate::ledger::artifact_kind(&value).to_string();
    let is_canonical = crate::preserves_rail::to_text(&value).is_ok_and(|rendered| rendered == text);
    let is_pass_eligible = artifact_decision(&value, &actual_kind)
        .ok()
        .flatten()
        .is_none_or(|decision| decision == molten_core::cluster_harness::RUN_DIRECTORY_PASS);
    molten_core::cluster_harness::RunArtifactObservation {
        relative_path: entry.relative_path.clone(),
        artifact_kind: actual_kind,
        observed_ref,
        format: molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES.to_string(),
        canonical: is_canonical,
        pass_eligible: is_pass_eligible,
    }
}

fn observe_text_artifact(
    path: &std::path::Path,
    entry: &molten_core::cluster_harness::RunArtifactIndexEntry,
) -> molten_core::cluster_harness::RunArtifactObservation {
    if !is_regular_file_without_symlink(path) {
        return missing_observation(entry);
    }
    match std::fs::read_to_string(path) {
        Ok(text) => molten_core::cluster_harness::RunArtifactObservation {
            relative_path: entry.relative_path.clone(),
            artifact_kind: entry.artifact_kind.clone(),
            observed_ref: Some(content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text)),
            format: molten_core::cluster_harness::ARTIFACT_FORMAT_TEXT.to_string(),
            canonical: true,
            pass_eligible: true,
        },
        Err(_) => missing_observation(entry),
    }
}

fn missing_observation(
    entry: &molten_core::cluster_harness::RunArtifactIndexEntry,
) -> molten_core::cluster_harness::RunArtifactObservation {
    molten_core::cluster_harness::RunArtifactObservation {
        relative_path: entry.relative_path.clone(),
        artifact_kind: entry.artifact_kind.clone(),
        observed_ref: None,
        format: entry.format.clone(),
        canonical: false,
        pass_eligible: false,
    }
}

fn observe_unexpected_artifact(
    run_directory: &std::path::Path,
    relative_path: &str,
) -> molten_core::cluster_harness::RunArtifactObservation {
    let path = run_directory.join(relative_path);
    if relative_path.ends_with(".preserves")
        && let Ok(value) = read_preserves_path(&path)
    {
        return molten_core::cluster_harness::RunArtifactObservation {
            relative_path: relative_path.to_string(),
            artifact_kind: crate::ledger::artifact_kind(&value).to_string(),
            observed_ref: crate::preserves_rail::canonical_hash(&value).ok(),
            format: molten_core::cluster_harness::ARTIFACT_FORMAT_PRESERVES.to_string(),
            canonical: true,
            pass_eligible: false,
        };
    }
    let text = std::fs::read_to_string(path).unwrap_or_default();
    molten_core::cluster_harness::RunArtifactObservation {
        relative_path: relative_path.to_string(),
        artifact_kind: "unexpected".to_string(),
        observed_ref: Some(content_ref_for_text(TEXT_ARTIFACT_DOMAIN, &text)),
        format: molten_core::cluster_harness::ARTIFACT_FORMAT_TEXT.to_string(),
        canonical: true,
        pass_eligible: false,
    }
}
