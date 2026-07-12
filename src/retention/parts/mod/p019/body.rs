
fn export_bundle_artifact_group(
    input: BundleArtifactGroupInput<'_>,
    artifact_refs: &mut impl VecSink<String>,
    diagnostics: &mut impl VecSink<String>,
) -> Result<()> {
    let group_dir = bundle_path(&format!("artifacts/{}", input.dir_name))?;
    input.bundle_root.root().create_dir_all(&group_dir)?;
    for reference in input.refs {
        match (input.read)(input.root, reference) {
            Ok(value) => {
                write_bundle_value(
                    input.bundle_root,
                    &bundle_artifact_path(input.dir_name, reference)?,
                    &value,
                )?;
                push_bounded(artifact_refs, reference.clone(), MAX_RETENTION_REFS, "retention bundle artifact refs")?;
            }
            Err(_) => push_bounded(
                diagnostics,
                format!("retention-bundle-missing-artifact:{reference}"),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle diagnostics",
            )?,
        }
    }
    Ok(())
}

fn profile_candidate_bundle(
    bundle_root: &CapabilityBundleRoot,
    profile: CandidateBundleExportProfile,
    bundle: &CandidateBundle,
) -> Result<CandidateBundleProfile> {
    let mut marker_refs = Vec::new();
    let mut diagnostics = Vec::new();
    if profile != CandidateBundleExportProfile::Internal {
        collect_bundle_sensitive_markers(&bundle.value, "/bundle", &bundle.bundle_ref, &mut marker_refs)?;
        let explain_value = read_bundle_value(bundle_root, &bundle_path("explain.preserves")?)?;
        collect_bundle_sensitive_markers(&explain_value, "/explain", &bundle.bundle_ref, &mut marker_refs)?;
        collect_bundle_artifact_sensitive_markers(bundle_root, &bundle.bundle_ref, &mut marker_refs)?;
        marker_refs.sort();
        marker_refs.dedup();
    }
    match profile {
        CandidateBundleExportProfile::Internal => {}
        CandidateBundleExportProfile::Public => {
            if !marker_refs.is_empty() {
                push_bounded(
                    &mut diagnostics,
                    format!("retention-bundle-public-sensitive-markers:{}", marker_refs.len()),
                    MAX_RETENTION_DIAGNOSTICS,
                    "retention bundle profile diagnostics",
                )?;
            }
        }
        CandidateBundleExportProfile::Diagnostic => {
            push_bounded(
                &mut diagnostics,
                format!("retention-bundle-diagnostic-redacted-markers:{}", marker_refs.len()),
                MAX_RETENTION_DIAGNOSTICS,
                "retention bundle profile diagnostics",
            )?;
        }
    }
    let decision = if profile == CandidateBundleExportProfile::Public && !marker_refs.is_empty() {
        "deny"
    } else {
        "pass"
    };
    let value = candidate_bundle_profile_value(&CandidateBundleProfileValueInput {
        profile,
        decision,
        bundle_ref: &bundle.bundle_ref,
        marker_refs: &marker_refs,
        diagnostics: &diagnostics,
    })?;
    parse_candidate_bundle_profile(&value)
}

fn collect_bundle_artifact_sensitive_markers(
    bundle_root: &CapabilityBundleRoot,
    bundle_ref: &str,
    marker_refs: &mut impl VecSink<String>,
) -> Result<()> {
    let artifact_dir = bundle_path("artifacts")?;
    if !bundle_root.root().try_exists(&artifact_dir)? {
        return Ok(());
    }
    for dir_name in bundle_artifact_dirs() {
        let group_dir = bundle_path(&format!("artifacts/{dir_name}"))?;
        if !bundle_root.root().try_exists(&group_dir)? {
            continue;
        }
        for entry in bundle_root.root().list_entries(&group_dir)? {
            if entry.kind != crate::local_store::LocalStoreEntryKind::File || !entry.name.ends_with(".preserves") {
                continue;
            }
            let value = read_bundle_value(bundle_root, &entry.path)?;
            collect_bundle_sensitive_markers(
                &value,
                &format!("/artifacts/{dir_name}/{}", entry.name),
                bundle_ref,
                marker_refs,
            )?;
        }
    }
    Ok(())
}

fn collect_bundle_sensitive_markers(
    value: &IoValue,
    path: &str,
    bundle_ref: &str,
    marker_refs: &mut impl VecSink<String>,
) -> Result<()> {
    let mut stack = Vec::new();
    push_bounded(
        &mut stack,
        (value.clone(), path.to_string()),
        MAX_RETENTION_REFS,
        "retention bundle marker scan stack",
    )?;
    while let Some((current, current_path)) = stack.pop() {
        if let Some(label) = record_label_string(&current)
            && is_sensitive_bundle_token(&label)
        {
            push_bounded(
                marker_refs,
                bundle_marker_ref(bundle_ref, &current_path, &label)?,
                MAX_RETENTION_REFS,
                "retention bundle profile markers",
            )?;
        }
        if let Some(text) = current.as_string()
            && is_sensitive_bundle_token(&text)
        {
            push_bounded(
                marker_refs,
                bundle_marker_ref(bundle_ref, &current_path, &text)?,
                MAX_RETENTION_REFS,
                "retention bundle profile markers",
            )?;
        }
        if matches!(
            current.value_class(),
            ValueClass::Compound(CompoundClass::Record) | ValueClass::Compound(CompoundClass::Sequence)
        ) {
            let mut children = Vec::new();
            for (index, child) in current.iter().enumerate() {
                push_bounded(
                    &mut children,
                    (index, crate::preserves_rail::value_to_iovalue(&child)),
                    MAX_RETENTION_REFS,
                    "retention bundle marker scan children",
                )?;
            }
            for (index, child) in children.into_iter().rev() {
                push_bounded(
                    &mut stack,
                    (child, format!("{current_path}/{index}")),
                    MAX_RETENTION_REFS,
                    "retention bundle marker scan stack",
                )?;
            }
        }
    }
    Ok(())
}

fn write_candidate_bundle_redacted_view(
    bundle_root: &CapabilityBundleRoot,
    bundle: &CandidateBundle,
) -> Result<()> {
    let mut ignored_markers = Vec::new();
    let bundle_value = read_bundle_value(bundle_root, &bundle_path("bundle.preserves")?)?;
    let redacted_bundle = redacted_bundle_value(&bundle_value, "/bundle", &bundle.bundle_ref, &mut ignored_markers)?;
    write_bundle_value(
        bundle_root,
        &bundle_path(&format!("{BUNDLE_REDACTED_DIR}/bundle.preserves"))?,
        &redacted_bundle,
    )?;
    let explain_value = read_bundle_value(bundle_root, &bundle_path("explain.preserves")?)?;
    let redacted_explain = redacted_bundle_value(&explain_value, "/explain", &bundle.bundle_ref, &mut ignored_markers)?;
    write_bundle_value(
        bundle_root,
        &bundle_path(&format!("{BUNDLE_REDACTED_DIR}/explain.preserves"))?,
        &redacted_explain,
    )?;
    for dir_name in bundle_artifact_dirs() {
        let group_dir = bundle_path(&format!("artifacts/{dir_name}"))?;
        if !bundle_root.root().try_exists(&group_dir)? {
            continue;
        }
        for entry in bundle_root.root().list_entries(&group_dir)? {
            if entry.kind != crate::local_store::LocalStoreEntryKind::File || !entry.name.ends_with(".preserves") {
                continue;
            }
            let value = read_bundle_value(bundle_root, &entry.path)?;
            let redacted = redacted_bundle_value(
                &value,
                &format!("/artifacts/{dir_name}/{}", entry.name),
                &bundle.bundle_ref,
                &mut ignored_markers,
            )?;
            write_bundle_value(
                bundle_root,
                &bundle_path(&format!("{BUNDLE_REDACTED_DIR}/artifacts/{dir_name}/{}", entry.name))?,
                &redacted,
            )?;
        }
    }
    Ok(())
}

enum RedactionFrame {
    Visit { value: IoValue, path: String },
    BuildRecord { label: IoValue, child_count: usize },
    BuildSequence { child_count: usize },
}

struct RedactionResults {
    values: Vec<IoValue>,
}

impl RedactionResults {
    fn new() -> Self {
        Self { values: Vec::new() }
    }

    fn push(&mut self, value: IoValue) -> Result<()> {
        push_bounded(&mut self.values, value, MAX_RETENTION_REFS, "retention bundle redacted values")
    }

    fn build_record(&mut self, label: IoValue, child_count: usize) -> Result<()> {
        let start = self
            .values
            .len()
            .checked_sub(child_count)
            .ok_or_else(|| MoltenError::invalid_harness("retention bundle redaction record stack underflow"))?;
        let fields = self.values.split_off(start);
        self.push(IoValue::record(label, fields))
    }

    fn build_sequence(&mut self, child_count: usize) -> Result<()> {
        let start = self
            .values
            .len()
            .checked_sub(child_count)
            .ok_or_else(|| MoltenError::invalid_harness("retention bundle redaction sequence stack underflow"))?;
        let values = self.values.split_off(start);
        self.push(crate::preserves_rail::sequence(values))
    }

    fn finish(mut self) -> Result<IoValue> {
        if self.values.len() != 1 {
            return Err(MoltenError::invalid_harness("retention bundle redaction result stack mismatch"));
        }
        self.values
            .pop()
            .ok_or_else(|| MoltenError::invalid_harness("retention bundle redaction produced no result"))
    }
}
