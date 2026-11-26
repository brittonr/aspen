/// Snapshot-authority receipts before the readable-assertions check was relabelled. Their canonical
/// identity is unchanged; they are read only through `parse_snapshot_authority_check`.
const SNAPSHOT_AUTHORITY_V1_PREDICATE: &str = "molten.trellis-runtime.snapshot-authority.v1";
/// The v1 label for the readable-assertions check. The check always compared readable assertions with
/// the admitted references, so this label is only an alias of `ReadableAssertionsSubsetAdmitted`.
const LEGACY_READABLE_ASSERTIONS_LABEL: &str = "readable-assertions-subset-claimed";

/// One check recorded by a snapshot-authority receipt. Each label names the predicate the validator
/// evaluates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotAuthorityCheck {
    SnapshotRefCanonical,
    AuthorityRefsCanonical,
    ClaimedAuthoritySubsetAdmitted,
    ReadableAssertionsSubsetAdmitted,
    RequestedAssertionsReadableOrRedacted,
}

impl SnapshotAuthorityCheck {
    /// Checks in receipt order.
    pub const ALL: [Self; 5] = [
        Self::SnapshotRefCanonical,
        Self::AuthorityRefsCanonical,
        Self::ClaimedAuthoritySubsetAdmitted,
        Self::ReadableAssertionsSubsetAdmitted,
        Self::RequestedAssertionsReadableOrRedacted,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::SnapshotRefCanonical => "snapshot-ref-canonical",
            Self::AuthorityRefsCanonical => "snapshot-authority-refs-canonical",
            Self::ClaimedAuthoritySubsetAdmitted => "claimed-authority-subset-admitted",
            Self::ReadableAssertionsSubsetAdmitted => "readable-assertions-subset-admitted",
            Self::RequestedAssertionsReadableOrRedacted => "requested-assertions-readable-or-redacted",
        }
    }
}

/// Parse one check label from a snapshot-authority receipt with the given predicate version. The v1
/// readable-assertions label is accepted only in v1 receipts, as an alias of the admitted-set check.
// r[impl molten.runtime_spine.snapshot_receipt_check_label]
pub fn parse_snapshot_authority_check(predicate: &str, label: &str) -> Result<SnapshotAuthorityCheck> {
    let is_v1 = match predicate {
        SNAPSHOT_AUTHORITY_PREDICATE => false,
        SNAPSHOT_AUTHORITY_V1_PREDICATE => true,
        _ => {
            return Err(crate::error::Failure::invalid_harness(format!(
                "unsupported snapshot authority predicate {predicate}"
            )));
        }
    };
    if is_v1 && label == LEGACY_READABLE_ASSERTIONS_LABEL {
        return Ok(SnapshotAuthorityCheck::ReadableAssertionsSubsetAdmitted);
    }
    SnapshotAuthorityCheck::ALL
        .into_iter()
        .find(|check| check.label() == label)
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("unsupported snapshot authority check {label}")))
}
