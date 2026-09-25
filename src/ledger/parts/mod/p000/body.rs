const MAX_SCAN_ENTRIES: usize = 100_000;
const _: () = assert!(MAX_SCAN_ENTRIES > 0);

pub type CapabilityLedgerRoot = crate::local_store::LedgerStoreRoot;

pub fn open_capability_ledger_root(root: &std::path::Path) -> crate::error::Result<CapabilityLedgerRoot> {
    CapabilityLedgerRoot::open(root)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub artifact_ref: String,
    pub artifact_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub artifact_ref: String,
    pub artifact_kind: String,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub artifact_ref: String,
    pub artifact_kind: String,
    pub receipt_value: preserves::IOValue,
}

pub fn import_artifact(root: &std::path::Path, artifact: &preserves::IOValue) -> crate::error::Result<Import> {
    let root = open_capability_ledger_root(root)?;
    import_artifact_with_root(&root, artifact)
}

pub fn import_artifact_with_root(
    root: &CapabilityLedgerRoot,
    artifact: &preserves::IOValue,
) -> crate::error::Result<Import> {
    ensure_dirs_with_root(root)?;
    let artifact_ref = crate::preserves_rail::canonical_hash(artifact)?;
    let artifact_kind = artifact_kind(artifact).to_string();
    let bytes = crate::preserves_rail::canonical_bytes(artifact)?;
    let path = content_store_path(&artifact_ref)?;
    match root.root().entry_kind_optional(&path)? {
        Some(crate::local_store::ObjectKind::File) => {
            let existing = root.root().read(&path)?;
            let existing_value = crate::preserves_rail::parse_canonical_bytes(&existing)?;
            let existing_ref = crate::preserves_rail::canonical_hash(&existing_value)?;
            if existing_ref != artifact_ref {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "ledger content path for {artifact_ref} contains corrupted bytes hashing to {existing_ref}"
                )));
            }
        }
        None => root.root().write(&path, &bytes)?,
        Some(kind) => {
            return Err(crate::error::Failure::invalid_harness(format!(
                "ledger content path for {artifact_ref} must be a regular file, got {kind:?}"
            )));
        }
    }
    let receipt_value = import_receipt_value(&artifact_ref, &artifact_kind);
    Ok(Import {
        artifact_ref,
        artifact_kind,
        receipt_value,
    })
}

pub fn export_artifact(
    root: &std::path::Path,
    artifact_ref: &str,
    out: &std::path::Path,
) -> crate::error::Result<Export> {
    let artifact = read_artifact(root, artifact_ref)?;
    let artifact_kind = artifact_kind(&artifact).to_string();
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(crate::error::Failure::from)?;
    }
    std::fs::write(out, crate::preserves_rail::to_text(&artifact)?).map_err(crate::error::Failure::from)?;
    let receipt_value = crate::preserves_rail::record("ledger-export-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::EVIDENCE_LEDGER_EXPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("artifact-kind", vec![crate::preserves_rail::string(&artifact_kind)]),
        crate::preserves_rail::record("artifact", vec![crate::preserves_rail::string(artifact_ref)]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("content-ref-found"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-export"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]);
    Ok(Export {
        artifact_ref: artifact_ref.to_string(),
        artifact_kind,
        receipt_value,
    })
}

pub fn read_artifact(root: &std::path::Path, artifact_ref: &str) -> crate::error::Result<preserves::IOValue> {
    let root = CapabilityLedgerRoot::open_existing(root)?;
    read_artifact_with_root(&root, artifact_ref)
}

// r[impl molten.runtime_spine.canonical_content_refs.materialized_readback]
pub fn read_artifact_with_root(
    root: &CapabilityLedgerRoot,
    artifact_ref: &str,
) -> crate::error::Result<preserves::IOValue> {
    let path = content_store_path(artifact_ref)?;
    let bytes = root.root().read(&path)?;
    let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
    let actual_ref = crate::preserves_rail::canonical_hash(&value)?;
    if actual_ref != artifact_ref {
        return Err(crate::error::Failure::invalid_harness(format!(
            "ledger content hash mismatch: got {actual_ref}, expected {artifact_ref}"
        )));
    }
    Ok(value)
}

pub fn list_artifacts(root: &std::path::Path) -> crate::error::Result<Vec<Entry>> {
    let content = root.join("content");
    if !content.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(content).map_err(crate::error::Failure::from)? {
        let entry = entry.map_err(crate::error::Failure::from)?;
        if !entry.file_type().map_err(crate::error::Failure::from)?.is_file() {
            continue;
        }
        let Some(artifact_ref) = ref_from_filename(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        let value = read_artifact(root, &artifact_ref)?;
        push_bounded(
            &mut entries,
            Entry {
                artifact_ref,
                artifact_kind: artifact_kind(&value).to_string(),
            },
            MAX_SCAN_ENTRIES,
            "ledger artifact entries",
        )?;
    }
    entries.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(entries)
}

pub fn pin_artifact(root: &std::path::Path, artifact_ref: &str) -> crate::error::Result<()> {
    ensure_dirs(root)?;
    read_artifact(root, artifact_ref)?;
    std::fs::write(pin_path(root, artifact_ref)?, artifact_ref).map_err(crate::error::Failure::from)
}
