const RELEASE_ARCHIVE_PROFILE: &str = "release-archive-v1";
const RELEASE_ARCHIVE_MANIFEST: &str = "release-export-manifest.preserves";
const RELEASE_ARCHIVE_COMPRESSION_LEVEL: i32 = 19;

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
    // r[impl molten.filesystem_materialization.archive_members]
    let policy = archive_policy()?;
    let source = molten::materialization::SourceDirectoryRoot::open_existing(output_path)?;
    let mut payloads = vec![molten::materialization::MaterializationPayload::new(
        RELEASE_ARCHIVE_MANIFEST,
        molten::preserves_rail::to_text(&manifest.value)?.into_bytes(),
    )];
    for (name, expected_ref) in &manifest.member_refs {
        let logical_path = molten::materialization::MaterializationPath::parse(name, policy.max_path_bytes)?;
        let bytes = source.read_path(&logical_path, policy.max_member_bytes)?;
        let actual_ref = molten::operator_dogfood::release_export_file_ref(name, &bytes);
        if actual_ref != *expected_ref {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "release export member {name} ref changed before archive write: manifest={expected_ref} observed={actual_ref}"
            )));
        }
        payloads.push(molten::materialization::MaterializationPayload::new(name, bytes));
    }
    let archive_file = molten::materialization::create_explicit_output_file(archive_path)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, RELEASE_ARCHIVE_COMPRESSION_LEVEL)
        .map_err(molten::error::MoltenError::from)?;
    let encoder = molten::materialization::write_archive(encoder, &policy, &payloads)?;
    encoder.finish().map_err(molten::error::MoltenError::from)?;
    Ok(())
}

pub(super) fn read(path: &std::path::Path) -> molten::error::Result<Read> {
    // r[impl molten.filesystem_materialization.archive_members]
    let policy = archive_policy()?;
    let archive_file = molten::materialization::open_explicit_input_file(path)?;
    let decoder = match zstd::stream::read::Decoder::new(archive_file) {
        Ok(decoder) => decoder,
        Err(_) => return Ok(denied_read("release-export-archive-compression-invalid")),
    };
    let verified = match molten::materialization::verify_archive(decoder, &policy) {
        Ok(verified) => verified,
        Err(_) => return Ok(denied_read("release-export-archive-member-policy-invalid")),
    };
    let mut manifest_value = None;
    let mut member_refs = Vec::with_capacity(verified.payloads.len());
    let mut diagnostics = Vec::new();
    for payload in verified.payloads {
        if payload.logical_path == RELEASE_ARCHIVE_MANIFEST {
            manifest_value = String::from_utf8(payload.bytes)
                .ok()
                .and_then(|text| molten::preserves_rail::parse_text(&text).ok());
            if manifest_value.is_none() {
                diagnostics.push("release-export-manifest-invalid".to_string());
            }
        } else {
            let reference = molten::operator_dogfood::release_export_file_ref(&payload.logical_path, &payload.bytes);
            member_refs.push((payload.logical_path, reference));
        }
    }
    member_refs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(Read {
        manifest_value,
        member_refs,
        diagnostics,
    })
}

fn denied_read(diagnostic: &str) -> Read {
    Read {
        manifest_value: None,
        member_refs: Vec::new(),
        diagnostics: vec![diagnostic.to_string()],
    }
}

fn archive_policy() -> molten::error::Result<molten::materialization::MaterializationPolicy> {
    molten::materialization::MaterializationPolicy::bounded(
        RELEASE_ARCHIVE_PROFILE,
        molten::materialization::ReplacementPolicy::NoReplace,
    )
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    const MEMBER_NAME: &str = "release-member.txt";
    const MEMBER_BYTES: &[u8] = b"release evidence";

    #[test]
    fn release_archive_round_trip_uses_verified_logical_members() {
        let root = crate::tests::temp_dir("release-archive-round-trip");
        write_member(&root.join(MEMBER_NAME), MEMBER_BYTES);
        let manifest = manifest(molten::operator_dogfood::release_export_file_ref(MEMBER_NAME, MEMBER_BYTES));
        let archive = root.join("release.tar.zst");
        super::write(&root, &archive, &manifest).expect("write release archive");
        let read = super::read(&archive).expect("read release archive");
        assert_eq!(read.manifest_value, Some(manifest.value));
        assert_eq!(read.member_refs, manifest.member_refs);
        assert!(read.diagnostics.is_empty());
    }

    #[test]
    fn release_archive_rejects_source_bytes_that_do_not_match_manifest() {
        let root = crate::tests::temp_dir("release-archive-ref-mismatch");
        write_member(&root.join(MEMBER_NAME), MEMBER_BYTES);
        let wrong_ref = molten::operator_dogfood::release_export_file_ref(MEMBER_NAME, b"different bytes");
        let manifest = manifest(wrong_ref);
        assert!(super::write(&root, &root.join("release.tar.zst"), &manifest).is_err());
    }

    fn manifest(member_ref: String) -> molten::operator_dogfood::ReleaseExportManifest {
        let value = molten::preserves_rail::record("release-export-manifest-test", Vec::new());
        let fixture_ref = molten::preserves_rail::canonical_hash(&value).expect("fixture ref");
        molten::operator_dogfood::ReleaseExportManifest {
            manifest_ref: fixture_ref.clone(),
            output_path_ref: fixture_ref.clone(),
            promotion_summary_ref: fixture_ref,
            member_refs: vec![(MEMBER_NAME.to_string(), member_ref)],
            checks: Vec::new(),
            value,
        }
    }

    fn write_member(path: &std::path::Path, bytes: &[u8]) {
        let mut file = molten::materialization::create_explicit_output_file(path).expect("member file");
        file.write_all(bytes).expect("write member");
        file.flush().expect("flush member");
    }
}
