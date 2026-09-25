
fn require_blake3_ref(label: &str, value: &str) -> crate::error::Result<()> {
    let Some(digest) = value.strip_prefix("blake3:") else {
        return Err(crate::error::MoltenError::invalid_harness(format!("artifact-auth {label} ref must use blake3")));
    };
    if digest.len() != BLAKE3_HEX_CHARS
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(crate::error::MoltenError::invalid_harness(format!("artifact-auth {label} ref is malformed")));
    }
    Ok(())
}
