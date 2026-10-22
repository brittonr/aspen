use super::super::*;
use super::*;

/// How a trace member was located in storage. A locator never identifies bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayMemberLocator {
    None,
    Path(String),
    SharedInode { device: u64, inode: u64 },
}

/// What a store presents for one trace member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayTraceMemberEvidence {
    pub content_ref: Option<String>,
    pub byte_length: Option<u64>,
    pub locator: ReplayMemberLocator,
}

/// The canonical identity of a trace member: content and length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayTraceMemberIdentity {
    pub content_ref: String,
    pub byte_length: u64,
}

/// Accept a trace member only by content identity and length. A path, a
/// shared inode, a hard link, or a reflink is a locator, not an identity.
// r[impl molten.world_replay_boundary.trace_identity]
pub fn identify_replay_trace_member(
    evidence: &ReplayTraceMemberEvidence,
) -> Result<ReplayTraceMemberIdentity, ReplayBoundaryBlocker> {
    match (&evidence.content_ref, evidence.byte_length) {
        (Some(content_ref), Some(byte_length)) if valid_reference(content_ref) => Ok(ReplayTraceMemberIdentity {
            content_ref: content_ref.clone(),
            byte_length,
        }),
        _ => Err(ReplayBoundaryBlocker::InvalidRecord("unidentified-trace-member")),
    }
}

/// The facts a shell gathers before keeping a shared copy of a trace member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayMemberSharingInput {
    pub declared: ReplayTraceMemberIdentity,
    /// The identity of the source bytes, as the shell hashed them.
    pub source: ReplayTraceMemberIdentity,
    pub is_source_immutable: bool,
    pub is_sharing_admitted: bool,
    /// The identity of the shared destination, read back after it was made.
    pub destination_readback: Option<ReplayTraceMemberIdentity>,
}

/// Whether the shell may keep a shared copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMemberSharing {
    /// Keep the reflink or deduplicated copy: every precondition held.
    RetainShared,
    /// Store an independent copy instead.
    CopyRequired,
}

/// Decide whether a storage-sharing optimization may stand. Sharing runs only
/// after the source identity, immutability, policy, and destination readback
/// all pass; a mismatched source or readback blocks the member entirely.
// r[impl molten.world_replay_boundary.trace_identity]
pub fn plan_replay_member_sharing(
    input: &ReplayMemberSharingInput,
) -> Result<ReplayMemberSharing, ReplayBoundaryBlocker> {
    if input.source != input.declared {
        return Err(ReplayBoundaryBlocker::InvalidRecord("trace-member-source-identity"));
    }
    if !input.is_source_immutable || !input.is_sharing_admitted {
        return Ok(ReplayMemberSharing::CopyRequired);
    }
    match &input.destination_readback {
        None => Ok(ReplayMemberSharing::CopyRequired),
        Some(readback) if *readback == input.declared => Ok(ReplayMemberSharing::RetainShared),
        Some(_) => Err(ReplayBoundaryBlocker::InvalidRecord("trace-member-readback-identity")),
    }
}
