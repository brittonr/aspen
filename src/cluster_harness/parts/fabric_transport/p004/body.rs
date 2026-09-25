
fn write_preserves_atomic(path: &std::path::Path, value: &preserves::IOValue) -> crate::error::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        crate::error::MoltenError::invalid_harness("endpoint handoff path requires a parent directory")
    })?;
    std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
    let temporary = path.with_extension("preserves.tmp");
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    let mut file = std::fs::File::create(&temporary).map_err(crate::error::MoltenError::from)?;
    file.write_all(&bytes).map_err(crate::error::MoltenError::from)?;
    file.sync_all().map_err(crate::error::MoltenError::from)?;
    std::fs::rename(&temporary, path).map_err(crate::error::MoltenError::from)
}

fn write_index(run_directory: &std::path::Path) -> crate::error::Result<()> {
    let entries = collect_indexed_artifacts(run_directory)?;
    std::fs::write(run_directory.join(INDEX_FILE), render_index(&entries)).map_err(crate::error::MoltenError::from)
}

fn collect_indexed_artifacts(run_directory: &std::path::Path) -> crate::error::Result<Vec<IndexedArtifact>> {
    let definitions = [
        (CLIENT_START_FILE, "parent-client-start", "preserves"),
        (CLIENT_TERMINAL_FILE, "client-terminal", "preserves"),
        (CLEANUP_FILE, "distinct-process-cleanup", "preserves"),
        (HANDOFF_FILE, "endpoint-handoff", "preserves"),
        (LISTENER_START_FILE, "parent-listener-start", "preserves"),
        (LISTENER_TERMINAL_FILE, "listener-terminal", "preserves"),
        (PARENT_RUN_FILE, "distinct-process-run", "preserves"),
        (PAYLOAD_INPUT_FILE, "transport-payload", "binary"),
        (REQUEST_INPUT_FILE, "transport-request-ref", "text"),
        (CLIENT_LOG_FILE, "diagnostic-log", "text"),
        (LISTENER_LOG_FILE, "diagnostic-log", "text"),
    ];
    let mut entries = Vec::with_capacity(definitions.len());
    for (relative_path, artifact_kind, format) in definitions {
        let path = run_directory.join(relative_path);
        ensure_regular_file(&path)?;
        let bytes = std::fs::read(&path).map_err(crate::error::MoltenError::from)?;
        let expected_ref = if format == "preserves" {
            let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
            crate::preserves_rail::canonical_hash(&value)?
        } else if format == "binary" {
            crate::preserves_rail::content_ref_from_bytes(&bytes)
        } else if relative_path == REQUEST_INPUT_FILE {
            text_ref("molten.fabric.transport.distinct-process-request-input.v1", &String::from_utf8_lossy(&bytes))
        } else {
            text_ref("molten.fabric.transport.distinct-process-log.v1", &String::from_utf8_lossy(&bytes))
        };
        entries.push(IndexedArtifact {
            relative_path: relative_path.to_string(),
            artifact_kind: artifact_kind.to_string(),
            expected_ref,
            format: format.to_string(),
        });
    }
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(entries)
}

fn render_index(entries: &[IndexedArtifact]) -> String {
    let mut output = String::from(INDEX_HEADER);
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

fn validate_run_membership(
    run_directory: &std::path::Path,
    require_companion: bool,
) -> crate::error::Result<Vec<String>> {
    let mut expected = expected_members();
    if !require_companion {
        expected.remove(VERIFICATION_FILE);
    }
    let mut observed = std::collections::BTreeSet::new();
    collect_relative_files(run_directory, run_directory, &mut observed)?;
    let mut diagnostics = expected
        .difference(&observed)
        .map(|missing| format!("missing-run-member:{missing}"))
        .chain(observed.difference(&expected).map(|extra| format!("unexpected-run-member:{extra}")))
        .collect::<Vec<_>>();
    for path in &expected {
        let absolute = run_directory.join(path);
        if absolute.exists() {
            ensure_regular_file(&absolute)?;
        }
    }
    diagnostics.sort();
    Ok(diagnostics)
}

fn expected_members() -> std::collections::BTreeSet<String> {
    let members = [
        HANDOFF_FILE,
        LISTENER_START_FILE,
        CLIENT_START_FILE,
        LISTENER_TERMINAL_FILE,
        CLIENT_TERMINAL_FILE,
        CLEANUP_FILE,
        PARENT_RUN_FILE,
        VERIFICATION_FILE,
        INDEX_FILE,
        PAYLOAD_INPUT_FILE,
        REQUEST_INPUT_FILE,
        LISTENER_LOG_FILE,
        CLIENT_LOG_FILE,
    ];
    debug_assert_eq!(members.len(), EXPECTED_MEMBER_COUNT);
    members.into_iter().map(str::to_string).collect()
}

fn collect_relative_files(
    root: &std::path::Path,
    current: &std::path::Path,
    output: &mut std::collections::BTreeSet<String>,
) -> crate::error::Result<()> {
    let mut pending = vec![current.to_path_buf()];
    while let Some(directory) = pending.pop() {
        if output.len() > MAX_RUN_FILES {
            return Err(crate::error::MoltenError::invalid_harness(
                "distinct-process run directory file count exceeds bound",
            ));
        }
        for entry in std::fs::read_dir(&directory).map_err(crate::error::MoltenError::from)? {
            let entry = entry.map_err(crate::error::MoltenError::from)?;
            let file_type = entry.file_type().map_err(crate::error::MoltenError::from)?;
            if file_type.is_symlink() {
                return Err(crate::error::MoltenError::invalid_harness(
                    "distinct-process run directory must not contain symlinks",
                ));
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|error| {
                        crate::error::MoltenError::invalid_harness(format!("run path strip failed: {error}"))
                    })?
                    .to_string_lossy()
                    .into_owned();
                output.insert(relative);
            } else {
                return Err(crate::error::MoltenError::invalid_harness(
                    "distinct-process run directory contains a non-regular entry",
                ));
            }
        }
    }
    Ok(())
}

fn ensure_regular_file(path: &std::path::Path) -> crate::error::Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(crate::error::MoltenError::from)?;
    if !metadata.file_type().is_file() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "distinct-process artifact is not a regular file: {}",
            path.display()
        )));
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "distinct-process artifact exceeds {MAX_ARTIFACT_BYTES} bytes: {}",
            path.display()
        )));
    }
    Ok(())
}

fn invocation_ref(role: &str) -> String {
    text_ref(INVOCATION_DOMAIN, role)
}

fn command_profile_ref(role: &str) -> String {
    text_ref(COMMAND_PROFILE_DOMAIN, role)
}

fn text_ref(domain: &str, text: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(text.as_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.map(crate::preserves_rail::string).collect())
}

fn checks(names: &[&str]) -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![strings_value(names.iter().copied())])
}

fn simple_record(
    value: &preserves::IOValue,
    label: &str,
    field_count: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let fields = value
        .collect_simple_record(label, Some(field_count))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(fields.iter().collect())
}

fn next<'a>(
    fields: &mut impl Iterator<Item = &'a preserves::Value<preserves::IOValue>>,
    label: &str,
) -> crate::error::Result<&'a preserves::Value<preserves::IOValue>> {
    fields.next().ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("missing {label}")))
}

fn required_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn required_ref(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    let reference = required_string(value, label)?;
    crate::preserves_rail::validate_content_ref(&reference)?;
    Ok(reference)
}

fn required_u64(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn required_bool(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn require_schema(value: &preserves::Value<preserves::IOValue>, expected: &str) -> crate::error::Result<()> {
    let actual = required_string(value, "schema")?;
    if actual == expected {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "schema mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn require_decision(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<()> {
    let decision = required_string(value, "decision")?;
    if decision == PASS_DECISION {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "participant decision must pass, got {decision}"
        )))
    }
}

fn parse_role(value: &str) -> crate::error::Result<EndpointParticipantRole> {
    match value {
        LISTENER_ROLE => Ok(EndpointParticipantRole::Listener),
        CLIENT_ROLE => Ok(EndpointParticipantRole::Client),
        other => Err(crate::error::MoltenError::invalid_harness(format!("unsupported participant role {other}"))),
    }
}

fn parse_delivery(value: &str) -> crate::error::Result<DeliveryOutcome> {
    match value {
        "not-attempted" => Ok(DeliveryOutcome::NotAttempted),
        "pending" => Ok(DeliveryOutcome::Pending),
        "delivered" => Ok(DeliveryOutcome::Delivered),
        "not-delivered" => Ok(DeliveryOutcome::NotDelivered),
        "uncertain" => Ok(DeliveryOutcome::Uncertain),
        other => Err(crate::error::MoltenError::invalid_harness(format!("unsupported delivery outcome {other}"))),
    }
}
