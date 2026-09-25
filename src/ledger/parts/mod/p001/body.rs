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

const ARTIFACT_KIND_RECORDS: &[(&str, &str)] = include!("../artifacts/p000/body.rs");

pub fn artifact_kind(value: &preserves::IOValue) -> &'static str {
    for &(record_label, kind) in ARTIFACT_KIND_RECORDS {
        if value.collect_simple_record(record_label, None).is_some() {
            return kind;
        }
    }
    "artifact"
}

fn ensure_dirs_with_root(root: &CapabilityLedgerRoot) -> crate::error::Result<()> {
    root.root().create_dir_all(&crate::local_store::RelativeLocator::parse("content")?)?;
    root.root().create_dir_all(&crate::local_store::RelativeLocator::parse("pins")?)
}

fn content_store_path(artifact_ref: &str) -> crate::error::Result<crate::local_store::RelativeLocator> {
    crate::local_store::RelativeLocator::parse("content")?.join(&filename_for_ref(artifact_ref)?)
}

fn ensure_dirs(root: &std::path::Path) -> crate::error::Result<()> {
    std::fs::create_dir_all(root.join("content")).map_err(crate::error::Failure::from)?;
    std::fs::create_dir_all(root.join("pins")).map_err(crate::error::Failure::from)
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
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(crate::error::Failure::invalid_harness(format!(
            "{label} count {total} exceeds bound {maximum}"
        )));
    }
    values.push_item(value);
    Ok(())
}

fn filename_for_ref(artifact_ref: &str) -> crate::error::Result<String> {
    let hex = crate::preserves_rail::content_ref_hex(artifact_ref).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("unsupported ledger artifact ref {artifact_ref}: {error}"))
    })?;
    Ok(format!("blake3_{hex}.bin"))
}

// r[impl molten.runtime_spine.canonical_content_refs.filename_readback]
fn ref_from_filename(filename: &str) -> Option<String> {
    let hex = filename.strip_prefix("blake3_").and_then(|value| value.strip_suffix(".bin"))?;
    crate::preserves_rail::content_ref_from_hex(hex).ok()
}
