type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn preserves_file_refs(paths: &[FilePath]) -> Outcome<Vec<String>> {
    let mut refs = Vec::with_capacity(paths.len());
    for path in paths {
        refs.push(preserves_file_ref(path)?);
    }
    Ok(refs)
}

pub(super) fn preserves_file_ref(path: &std::path::Path) -> Outcome<String> {
    let value = read_preserves_file(path)?;
    molten::preserves_rail::canonical_hash(&value)
}

pub(super) fn optional_preserves_ref(path: Option<&FilePath>) -> Outcome<Option<String>> {
    match path {
        Some(value) => preserves_file_ref(value).map(Some),
        None => Ok(None),
    }
}

pub(super) fn raw_file_refs(paths: &[FilePath]) -> Outcome<Vec<String>> {
    let mut refs = Vec::with_capacity(paths.len());
    for path in paths {
        refs.push(raw_file_ref(path)?);
    }
    Ok(refs)
}

pub(super) fn raw_file_ref(path: &std::path::Path) -> Outcome<String> {
    let bytes = std::fs::read(path).map_err(molten::error::MoltenError::from)?;
    Ok(molten::preserves_rail::content_ref_from_bytes(&bytes))
}

pub(super) fn read_preserves_file(path: &std::path::Path) -> Outcome<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

pub(super) fn write_optional_preserves(path: Option<&FilePath>, value: &preserves::IOValue) -> Outcome<bool> {
    let text = molten::preserves_rail::to_text(value)?;
    if let Some(path) = path {
        write_file(path, &text)?;
        Ok(true)
    } else {
        println!("{text}");
        Ok(false)
    }
}

fn write_file(path: &std::path::Path, contents: &str) -> Outcome<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}

pub(super) fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}

pub(super) fn kind(text: &str) -> &'static str {
    const KINDS: &[(&str, &str)] = &[
        ("nixos-vm-topology-v1", "topology"),
        ("nixos-vm-node-evidence-v1", "node-evidence"),
        ("nixos-vm-test-run-v1", "test-run"),
        ("nixos-vm-evidence-validation-v1", "vm-evidence-validation"),
        ("nixos-vm-evidence-manifest-v1", "vm-evidence-manifest"),
        ("nixos-vm-fault-descriptor-v1", "vm-fault-descriptor"),
        ("nixos-vm-fault-receipt-v1", "vm-fault-receipt"),
        ("nixos-vm-fault-validation-v1", "vm-fault-validation"),
        ("prod-soak-run-v1", "prod-soak-run"),
    ];
    // The first marker that appears in the text decides the kind.
    for (marker, kind) in KINDS {
        if text.contains(marker) {
            return kind;
        }
    }
    "artifact"
}
