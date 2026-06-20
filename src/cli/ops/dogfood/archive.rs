#[derive(Debug)]
pub(super) struct Read {
    pub(super) manifest_value: Option<preserves::IOValue>,
    pub(super) member_refs: Vec<(String, String)>,
    pub(super) diagnostics: Vec<String>,
}

pub(super) fn write(
    output_path: &std::path::Path,
    archive_path: &std::path::Path,
    manifest: &molten::operator_dogfood::ReleaseExportManifest,
) -> molten::error::Result<()> {
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    let archive_file = std::fs::File::create(archive_path).map_err(molten::error::MoltenError::from)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 19).map_err(molten::error::MoltenError::from)?;
    let mut builder = tar::Builder::new(encoder);
    append_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        molten::preserves_rail::to_text(&manifest.value)?.as_bytes(),
    )?;
    for (name, expected_ref) in &manifest.member_refs {
        let bytes = std::fs::read(output_path.join(name)).map_err(molten::error::MoltenError::from)?;
        let actual_ref = molten::operator_dogfood::release_export_file_ref(name, &bytes);
        if actual_ref != *expected_ref {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "release export member {name} ref changed before archive write: manifest={expected_ref} observed={actual_ref}"
            )));
        }
        append_bytes(&mut builder, name, &bytes)?;
    }
    let encoder = builder.into_inner().map_err(molten::error::MoltenError::from)?;
    encoder.finish().map_err(molten::error::MoltenError::from)?;
    Ok(())
}

fn append_bytes<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    name: &str,
    bytes: &[u8],
) -> molten::error::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o444);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    builder
        .append_data(&mut header, name, std::io::Cursor::new(bytes))
        .map_err(molten::error::MoltenError::from)
}

pub(super) fn read(path: &std::path::Path) -> molten::error::Result<Read> {
    let archive_file = std::fs::File::open(path).map_err(molten::error::MoltenError::from)?;
    let decoder = zstd::stream::read::Decoder::new(archive_file).map_err(molten::error::MoltenError::from)?;
    let mut archive = tar::Archive::new(decoder);
    let mut manifest_value = None;
    let mut seen_names =
        Vec::with_capacity(molten::operator_dogfood::release_export_member_names().len().saturating_add(16));
    let mut member_refs =
        Vec::with_capacity(molten::operator_dogfood::release_export_member_names().len().saturating_add(16));
    let mut diagnostics = Vec::with_capacity(8);
    let entries = archive.entries().map_err(molten::error::MoltenError::from)?;
    for entry in entries {
        let mut entry = entry.map_err(molten::error::MoltenError::from)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry.path().map_err(molten::error::MoltenError::from)?.to_string_lossy().replace('\\', "/");
        if seen_names.iter().any(|seen| seen == &name) {
            diagnostics.push(format!("duplicate release export archive member: {name}"));
        }
        seen_names.push(name.clone());
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).map_err(molten::error::MoltenError::from)?;
        if name == "release-export-manifest.preserves" {
            if manifest_value.is_some() {
                diagnostics.push("duplicate release export manifest member".to_string());
            }
            let text = String::from_utf8(bytes).map_err(|error| {
                molten::error::MoltenError::invalid_harness(format!("release export manifest is not UTF-8: {error}"))
            })?;
            manifest_value = Some(molten::preserves_rail::parse_text(&text)?);
        } else {
            member_refs.push((name.clone(), molten::operator_dogfood::release_export_file_ref(&name, &bytes)));
        }
    }
    member_refs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(Read {
        manifest_value,
        member_refs,
        diagnostics,
    })
}
