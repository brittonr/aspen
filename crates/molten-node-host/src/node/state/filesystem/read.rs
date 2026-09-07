//! Consume an already-open file under both observed-size and actual-read bounds.
use std::io::Read;

pub(in crate::node_state) fn consume(
    observed: crate::node_state::NodeStateFile,
    max_bytes: u64,
    label: &str,
) -> crate::error::Result<Vec<u8>> {
    if max_bytes > crate::node_state::MAX_NODE_STATE_FILE_BYTES {
        return Err(crate::node_state::invalid(format!(
            "node state read bound {max_bytes} exceeds hard maximum {}",
            crate::node_state::MAX_NODE_STATE_FILE_BYTES
        )));
    }
    if observed.size > max_bytes {
        return Err(crate::node_state::invalid(format!("{label} size {} exceeds bound {max_bytes}", observed.size)));
    }
    collect(observed.file, max_bytes, label)
}

fn collect(file: cap_std::fs::File, max_bytes: u64, label: &str) -> crate::error::Result<Vec<u8>> {
    let limit_bytes = max_bytes
        .checked_add(1)
        .ok_or_else(|| crate::node_state::invalid("node state read bound overflow"))?;
    let mut bytes = Vec::new();
    file.take(limit_bytes).read_to_end(&mut bytes).map_err(crate::error::MoltenError::from)?;
    if u64::try_from(bytes.len())
        .map_err(|_| crate::node_state::invalid("node state read length conversion overflow"))?
        > max_bytes
    {
        return Err(crate::node_state::invalid(format!("{label} exceeds bound {max_bytes}")));
    }
    Ok(bytes)
}
