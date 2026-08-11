
fn record_execution(
    input: ReviewInput<'_>,
    entry: &Entry,
    retention_class: &str,
    review: &mut Review,
) -> crate::error::Result<bool> {
    if input.source.dry_run {
        return Ok(false);
    }
    let apply_ref = matching_apply_ref(ApplyRefMatchInput {
        root: input.root,
        apply_refs: input.source.apply_refs,
        subsystem: "ledger-gc",
        action: input.action,
        object_ref: &entry.artifact_ref,
        object_kind: &entry.artifact_kind,
        retention_class,
    });
    let execution_gate = crate::retention::store_gc_execution_gate(crate::retention::GcExecutionGateInput {
        root: input.root,
        subsystem: "ledger-gc",
        action: input.action,
        object_ref: &entry.artifact_ref,
        object_kind: &entry.artifact_kind,
        retention_class,
        apply_ref,
    })?;
    push_bounded(
        &mut review.execution_gate_refs,
        execution_gate.execution_ref.clone(),
        MAX_SCAN_ENTRIES,
        "ledger retention execution gate refs",
    )?;
    if execution_gate.decision == "pass" {
        return Ok(false);
    }
    extend_refs(
        &mut review.execution_diagnostics,
        &execution_gate.diagnostics,
        "ledger retention execution diagnostics",
    )?;
    Ok(true)
}

fn decision_for(denied_refs: &[String]) -> &'static str {
    if denied_refs.is_empty() { "pass" } else { "deny" }
}

fn remove_entries(
    root: &std::path::Path,
    candidates: &[Entry],
    is_dry_run: bool,
    decision: &str,
) -> crate::error::Result<Vec<String>> {
    let mut removed_refs = Vec::new();
    if decision == "pass" {
        for entry in candidates {
            push_bounded(&mut removed_refs, entry.artifact_ref.clone(), MAX_SCAN_ENTRIES, "ledger removed refs")?;
            if !is_dry_run {
                std::fs::remove_file(content_path(root, &entry.artifact_ref)?)
                    .map_err(crate::error::MoltenError::from)?;
            }
        }
    }
    Ok(removed_refs)
}

struct OutcomeInput<'a> {
    is_dry_run: bool,
    decision: &'a str,
    removed_refs: &'a [String],
    evidence_summary: preserves::IOValue,
    review: &'a Review,
}

fn outcome_value(input: OutcomeInput<'_>) -> preserves::IOValue {
    crate::preserves_rail::record("ledger-gc-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::EVIDENCE_LEDGER_GC_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string(mode_for(input.is_dry_run))]),
        crate::preserves_rail::record("removed", vec![crate::preserves_rail::sequence(
            input.removed_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention", vec![crate::preserves_rail::sequence(
            input.review.retention_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention-execution", vec![crate::preserves_rail::sequence(
            input.review.execution_gate_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("denied", vec![crate::preserves_rail::sequence(
            input.review.denied_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention-evidence", vec![input.evidence_summary]),
        crate::preserves_rail::record("retention-admission", vec![crate::preserves_rail::sequence(
            input.review.admission_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention-diagnostics", vec![crate::preserves_rail::sequence(
            input.review.admission_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention-execution-diagnostics", vec![crate::preserves_rail::sequence(
            input.review.execution_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![outcome_checks(input.is_dry_run, input.decision, input.review)]),
    ])
}

fn mode_for(is_dry_run: bool) -> &'static str {
    if is_dry_run { "dry-run" } else { "apply" }
}

fn outcome_checks(is_dry_run: bool, decision: &str, review: &Review) -> preserves::IOValue {
    crate::preserves_rail::sequence(vec![
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("pin-preservation"),
            crate::preserves_rail::string("pass"),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("derived-index-scan"),
            crate::preserves_rail::string("pass"),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("retention-receipt-bound"),
            crate::preserves_rail::string("pass"),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("retention-execution-gate"),
            crate::preserves_rail::string(pass_or_fail(is_dry_run || review.execution_diagnostics.is_empty())),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("retention-authority-evidence"),
            crate::preserves_rail::string(pass_or_fail(review.admission_diagnostics.is_empty())),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("deny-before-removal"),
            crate::preserves_rail::string(if decision == "pass" { "pass" } else { "fail" }),
        ]),
    ])
}

fn pass_or_fail(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

struct ApplyRefMatchInput<'a> {
    root: &'a std::path::Path,
    apply_refs: &'a [String],
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

fn matching_apply_ref<'a>(input: ApplyRefMatchInput<'a>) -> Option<&'a str> {
    let mut fallback_ref = None;
    for apply_ref in input.apply_refs {
        let Ok(apply) = crate::retention::read_gc_apply(input.root, apply_ref) else {
            if fallback_ref.is_none() {
                fallback_ref = Some(apply_ref.as_str());
            }
            continue;
        };
        if apply.decision == "pass"
            && apply.subsystem == input.subsystem
            && apply.action == input.action
            && apply.object_ref == input.object_ref
            && apply.object_kind == input.object_kind
            && apply.retention_class == input.retention_class
        {
            return Some(apply_ref.as_str());
        }
        if fallback_ref.is_none() {
            fallback_ref = Some(apply_ref.as_str());
        }
    }
    fallback_ref
}

fn retention_class(artifact_kind: &str) -> &'static str {
    if artifact_kind.contains("secret") || artifact_kind.contains("encrypted") || artifact_kind.contains("redaction") {
        crate::retention::CLASS_PRIVATE_SECRET_REF
    } else if artifact_kind.contains("cache") {
        crate::retention::CLASS_EPHEMERAL_CACHE
    } else if artifact_kind.contains("artifact") || artifact_kind.contains("manifest") {
        crate::retention::CLASS_PUBLIC_ARTIFACT
    } else {
        crate::retention::CLASS_AUDIT_RECEIPT
    }
}

pub fn import_receipt_value(artifact_ref: &str, artifact_kind: &str) -> preserves::IOValue {
    crate::preserves_rail::record("ledger-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::EVIDENCE_LEDGER_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("artifact-kind", vec![crate::preserves_rail::string(artifact_kind)]),
        crate::preserves_rail::record("artifact", vec![crate::preserves_rail::string(artifact_ref)]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-content-hash"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("immutable-content"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("derived-index-ready"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ])
}

const ARTIFACT_KIND_RECORDS: &[(&str, &str)] = include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/ledger/parts/mod/artifacts/p000/body.rs"
));

pub fn artifact_kind(value: &preserves::IOValue) -> &'static str {
    for &(record_label, kind) in ARTIFACT_KIND_RECORDS {
        if value.collect_simple_record(record_label, None).is_some() {
            return kind;
        }
    }
    "artifact"
}

fn ensure_dirs_with_root(root: &CapabilityLedgerRoot) -> crate::error::Result<()> {
    root.root().create_dir_all(&crate::local_store::LocalStorePath::parse("content")?)?;
    root.root().create_dir_all(&crate::local_store::LocalStorePath::parse("pins")?)
}

fn content_store_path(artifact_ref: &str) -> crate::error::Result<crate::local_store::LocalStorePath> {
    crate::local_store::LocalStorePath::parse("content")?.join(&filename_for_ref(artifact_ref)?)
}

fn ensure_dirs(root: &std::path::Path) -> crate::error::Result<()> {
    std::fs::create_dir_all(root.join("content")).map_err(crate::error::MoltenError::from)?;
    std::fs::create_dir_all(root.join("pins")).map_err(crate::error::MoltenError::from)
}

fn content_path(root: &std::path::Path, artifact_ref: &str) -> crate::error::Result<std::path::PathBuf> {
    Ok(root.join("content").join(filename_for_ref(artifact_ref)?))
}

fn pin_path(root: &std::path::Path, artifact_ref: &str) -> crate::error::Result<std::path::PathBuf> {
    Ok(root.join("pins").join(filename_for_ref(artifact_ref)?))
}

fn push_bounded<T>(
    values: &mut impl crate::bounded::VecSink<T>,
    value: T,
    maximum: usize,
    label: &str,
) -> crate::error::Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} count {total} exceeds bound {maximum}"
        )));
    }
    values.push_item(value);
    Ok(())
}

fn filename_for_ref(artifact_ref: &str) -> crate::error::Result<String> {
    let hex = crate::preserves_rail::content_ref_hex(artifact_ref).map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("unsupported ledger artifact ref {artifact_ref}: {error}"))
    })?;
    Ok(format!("blake3_{hex}.bin"))
}

// r[impl molten.runtime_spine.canonical_content_refs.filename_readback]
fn ref_from_filename(filename: &str) -> Option<String> {
    let hex = filename.strip_prefix("blake3_").and_then(|value| value.strip_suffix(".bin"))?;
    crate::preserves_rail::content_ref_from_hex(hex).ok()
}

fn pinned_refs(root: &std::path::Path) -> crate::error::Result<Vec<String>> {
    let pins = root.join("pins");
    if !pins.exists() {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in std::fs::read_dir(pins).map_err(crate::error::MoltenError::from)? {
        let entry = entry.map_err(crate::error::MoltenError::from)?;
        if entry.file_type().map_err(crate::error::MoltenError::from)?.is_file() {
            let reference = std::fs::read_to_string(entry.path()).map_err(crate::error::MoltenError::from)?;
            crate::preserves_rail::validate_content_ref(&reference).map_err(|error| {
                crate::error::MoltenError::invalid_harness(format!(
                    "ledger pin file contains invalid content ref {reference}: {error}"
                ))
            })?;
            push_bounded(&mut refs, reference, MAX_SCAN_ENTRIES, "ledger pinned refs")?;
        }
    }
    Ok(refs)
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ledger/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ledger/parts/mod/tests/m000/p001/body.rs"));
}
