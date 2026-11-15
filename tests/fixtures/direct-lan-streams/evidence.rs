fn reference(label: &str) -> molten::fabric::FabricPortResult<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("direct-stream-test-input-v1", vec![
        molten::preserves_rail::string(label),
    ])).map_err(molten::fabric::FabricPortError::from)
}

fn canonical_output(path: &std::path::Path, evidence: &molten::fabric_transport::direct::Evidence) -> TestResult {
    let bytes = molten::preserves_rail::canonical_bytes(&evidence.value)?;
    let parsed = molten::preserves_rail::parse_canonical_bytes(&bytes)?;
    assert_eq!(parsed, evidence.value);
    assert_eq!(molten::preserves_rail::canonical_hash(&parsed)?, evidence.evidence_ref);
    std::fs::write(path, bytes)?;
    Ok(())
}

fn retain_evidence(root: &std::path::Path, case: &str, output: Option<&std::path::Path>) -> TestResult {
    let Some(output) = output else {
        return Ok(());
    };
    if output.exists() {
        let metadata = std::fs::symlink_metadata(output)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("direct evidence root must be an explicit real directory".into());
        }
    } else {
        std::fs::create_dir_all(output)?;
    }
    let target = output.join(case);
    // Each run/case is create-only. Never overwrite or merge stale successful evidence.
    std::fs::create_dir(&target)?;
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700))?;
    let names = ["listener-evidence.preserves", "client-evidence.preserves"];
    let mut artifacts = Vec::with_capacity(names.len());
    for name in names {
        let source = root.join(name);
        if !source.exists() {
            continue;
        }
        let bytes = std::fs::read(&source)?;
        if bytes.len() as u64 > MAX_ARTIFACT_BYTES {
            return Err("native stream evidence exceeded retention bound".into());
        }
        let value = molten::preserves_rail::parse_canonical_bytes(&bytes)?;
        std::fs::write(target.join(name), &bytes)?;
        std::fs::set_permissions(target.join(name), std::fs::Permissions::from_mode(0o600))?;
        artifacts.push(molten::preserves_rail::record("artifact", vec![
            molten::preserves_rail::string(name),
            molten::preserves_rail::string(molten::preserves_rail::canonical_hash(&value)?),
        ]));
    }
    let index = molten::preserves_rail::record("fabric-direct-stream-retained-index-v1", vec![
        molten::preserves_rail::string(case),
        molten::preserves_rail::sequence(artifacts),
        molten::preserves_rail::string("native-child-output-not-two-host-or-kiln-qualification"),
    ]);
    std::fs::write(target.join("artifact-index.preserves"), molten::preserves_rail::canonical_bytes(&index)?)?;
    Ok(())
}
