//! Sync commands sent to the async worker for processing.

use aspen_forge::identity::RepoId;
use iroh::PublicKey;

use crate::gossip::PijulAnnouncement;
use crate::types::ChangeHash;

/// Command sent to the async worker for processing.
pub(super) enum SyncCommand {
    /// Download changes from a peer.
    DownloadChanges {
        repo_id: RepoId,
        hashes: Vec<ChangeHash>,
        provider: PublicKey,
    },
    /// Respond to a WantChanges request.
    RespondWithChanges {
        repo_id: RepoId,
        requested_hashes: Vec<ChangeHash>,
    },
    /// Check if we're behind on a channel and request updates.
    CheckChannelSync {
        repo_id: RepoId,
        channel: String,
        remote_head: ChangeHash,
        announcer: PublicKey,
    },
    /// Request a specific change that was announced.
    RequestChange {
        repo_id: RepoId,
        change_hash: ChangeHash,
        provider: PublicKey,
    },
    /// Process an announcement that was deferred due to lock contention.
    ///
    /// When the sync callback's `on_announcement` cannot acquire locks without
    /// blocking (using `try_write()`), it defers the announcement to the async
    /// worker which can block safely without stalling the gossip event loop.
    ProcessDeferredAnnouncement {
        announcement: PijulAnnouncement,
        signer: PublicKey,
    },
}
