use super::MAX_SNAPSHOT_CANONICAL_BYTES;

const SNAPSHOT_FRAME_VERSION: &[u8] = b"molten-world-snapshot-frame-v1";
const FRAME_SEPARATOR: &[u8] = &[0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotIdentityKind {
    Descriptor,
    Inventory,
    Compatibility,
    RestorePlan,
    ClonePlan,
    Receipt,
}

impl SnapshotIdentityKind {
    const fn context(self) -> &'static str {
        match self {
            Self::Descriptor => "onixresearch.molten.world-snapshot.descriptor.identity.v1",
            Self::Inventory => "onixresearch.molten.world-snapshot.inventory.identity.v1",
            Self::Compatibility => "onixresearch.molten.world-snapshot.compatibility.identity.v1",
            Self::RestorePlan => "onixresearch.molten.world-snapshot.restore-plan.identity.v1",
            Self::ClonePlan => "onixresearch.molten.world-snapshot.clone-plan.identity.v1",
            Self::Receipt => "onixresearch.molten.world-snapshot.receipt.identity.v1",
        }
    }

    const fn tag(self) -> &'static [u8] {
        match self {
            Self::Descriptor => b"descriptor",
            Self::Inventory => b"inventory",
            Self::Compatibility => b"compatibility",
            Self::RestorePlan => b"restore-plan",
            Self::ClonePlan => b"clone-plan",
            Self::Receipt => b"receipt",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotIdentityIssue {
    EmptyCanonicalBytes,
    CanonicalBytesExceeded { actual: usize, maximum: usize },
    ByteLengthOverflow,
}

pub fn identify_snapshot_artifact(
    kind: SnapshotIdentityKind,
    canonical_preserves_bytes: &[u8],
) -> Result<String, SnapshotIdentityIssue> {
    if canonical_preserves_bytes.is_empty() {
        return Err(SnapshotIdentityIssue::EmptyCanonicalBytes);
    }
    if canonical_preserves_bytes.len() > MAX_SNAPSHOT_CANONICAL_BYTES {
        return Err(SnapshotIdentityIssue::CanonicalBytesExceeded {
            actual: canonical_preserves_bytes.len(),
            maximum: MAX_SNAPSHOT_CANONICAL_BYTES,
        });
    }
    let length =
        u64::try_from(canonical_preserves_bytes.len()).map_err(|_| SnapshotIdentityIssue::ByteLengthOverflow)?;
    let mut hasher = blake3::Hasher::new_derive_key(kind.context());
    hasher.update(SNAPSHOT_FRAME_VERSION);
    hasher.update(FRAME_SEPARATOR);
    hasher.update(kind.tag());
    hasher.update(FRAME_SEPARATOR);
    hasher.update(&length.to_be_bytes());
    hasher.update(canonical_preserves_bytes);
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}
