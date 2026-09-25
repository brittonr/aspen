
fn advance(state_root: &Path, claim_path: &Path, signature_path: &Path) -> Result<()> {
    let claim = parse_canonical_world_head_claim(&std::fs::read(claim_path)?)?;
    let signature: SignatureDocument = serde_json::from_slice(&std::fs::read(signature_path)?)
        .map_err(|error| MoltenError::invalid_harness(format!("parse world-head signature: {error}")))?;
    let _ = bytes_from_hex(&signature.public_key_hex)?;
    let _ = bytes_from_hex(&signature.signature_hex)?;
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldHeadStore::open(&storage)?;
    let observed = store
        .read_head(&claim.claim.branch_id)
        .map_err(|error| MoltenError::invalid_harness(format!("read world head: {error}")))?;
    println!("claim_ref={}", claim.claim_ref);
    println!("observed_state={}", if observed.is_some() { "present" } else { "absent" });
    println!("decision=denied");
    println!("issue=current-authority-adapter-unavailable");
    Err(MoltenError::invalid_harness(
        "standalone world-head advance is disabled until a current authority adapter is composed",
    ))
}

fn conflicts(state_root: &Path, branch: &str, out_dir: Option<&Path>) -> Result<()> {
    let branch = WorldBranchId::new(branch).map_err(head_reference_error)?;
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldHeadStore::open(&storage)?;
    let records = store
        .read_conflicts(&branch)
        .map_err(|error| MoltenError::invalid_harness(format!("read world-head conflicts: {error}")))?;
    println!("branch={branch}");
    println!("conflict_count={}", records.len());
    if let Some(out_dir) = out_dir {
        std::fs::create_dir_all(out_dir)?;
        for bytes in &records {
            let reference = molten::preserves_rail::content_ref_from_bytes(bytes);
            let digest = molten::preserves_rail::content_ref_hex(&reference)?;
            std::fs::write(out_dir.join(format!("{digest}.preserves")), bytes)?;
        }
    }
    Ok(())
}

fn reconcile(state_root: &Path, branch: &str) -> Result<()> {
    inspect(state_root, branch)?;
    println!("reconciliation=manual-review-required");
    println!("automatic-head-selection=disabled");
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(HEX_CHARACTERS_PER_BYTE));
    for byte in bytes {
        use std::fmt::Write;
        let write_result = write!(&mut output, "{byte:02x}");
        assert!(write_result.is_ok(), "writing to String must succeed");
    }
    output
}

fn bytes_from_hex(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(HEX_CHARACTERS_PER_BYTE) {
        return Err(MoltenError::invalid_harness("hex value has an odd length"));
    }
    value
        .as_bytes()
        .chunks_exact(HEX_CHARACTERS_PER_BYTE)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| MoltenError::invalid_harness("hex value is not UTF-8"))?;
            u8::from_str_radix(text, 16).map_err(|_| MoltenError::invalid_harness("hex value contains an invalid byte"))
        })
        .collect()
}

fn head_reference_error(error: molten_core::world_head::WorldHeadReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world-head reference: {error}"))
}

fn commit_reference_error(error: molten_core::world_commit::WorldCommitReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world commit reference: {error:?}"))
}
