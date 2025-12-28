//! Announcement handler for Forge gossip.
//!
//! This module implements the handler that processes incoming announcements
//! and triggers appropriate actions (sync, peer tracking).

use std::sync::Arc;

use iroh::PublicKey;
use tokio::sync::mpsc;
use tracing;

use super::service::AnnouncementCallback;
use super::types::Announcement;
use crate::forge::cob::CobType;
use crate::forge::identity::RepoId;

/// Request to sync objects from peers.
#[derive(Debug, Clone)]
pub enum SyncRequest {
    /// Sync a ref update (fetch commit and its tree).
    RefUpdate {
        repo_id: RepoId,
        ref_name: String,
        commit_hash: blake3::Hash,
        peer: PublicKey,
    },
    /// Sync a COB change.
    CobChange {
        repo_id: RepoId,
        cob_type: CobType,
        cob_id: blake3::Hash,
        change_hash: blake3::Hash,
        peer: PublicKey,
    },
}

/// Handler that processes incoming Forge announcements.
///
/// Implements auto-sync behavior:
/// - RefUpdate → queue commit sync for seeded repos
/// - CobChange → queue COB fetch for seeded repos
/// - Seeding → add peer to seeder list
/// - Unseeding → remove peer from seeder list
/// - RepoCreated → log discovery
pub struct ForgeAnnouncementHandler {
    /// Channel for queuing sync requests.
    sync_tx: mpsc::Sender<SyncRequest>,
    /// Channel for seeding updates.
    seeding_tx: mpsc::Sender<SeedingUpdate>,
}

/// Update to seeding peer tracking.
#[derive(Debug, Clone)]
pub struct SeedingUpdate {
    /// Repository ID.
    pub repo_id: RepoId,
    /// Peer public key.
    pub peer: PublicKey,
    /// Whether adding (true) or removing (false).
    pub is_seeding: bool,
}

impl ForgeAnnouncementHandler {
    /// Create a new announcement handler.
    ///
    /// # Arguments
    ///
    /// - `sync_tx`: Channel for sending sync requests
    /// - `seeding_tx`: Channel for sending seeding updates
    pub fn new(
        sync_tx: mpsc::Sender<SyncRequest>,
        seeding_tx: mpsc::Sender<SeedingUpdate>,
    ) -> Self {
        Self { sync_tx, seeding_tx }
    }

    /// Create a handler with channels for testing or simple integration.
    ///
    /// Returns the handler and receivers for sync and seeding updates.
    pub fn with_channels(
        buffer_size: usize,
    ) -> (
        Self,
        mpsc::Receiver<SyncRequest>,
        mpsc::Receiver<SeedingUpdate>,
    ) {
        let (sync_tx, sync_rx) = mpsc::channel(buffer_size);
        let (seeding_tx, seeding_rx) = mpsc::channel(buffer_size);

        (Self::new(sync_tx, seeding_tx), sync_rx, seeding_rx)
    }
}

impl AnnouncementCallback for ForgeAnnouncementHandler {
    fn on_announcement(&self, announcement: &Announcement, signer: &PublicKey) {
        match announcement {
            Announcement::RefUpdate {
                repo_id,
                ref_name,
                new_hash,
                ..
            } => {
                let request = SyncRequest::RefUpdate {
                    repo_id: *repo_id,
                    ref_name: ref_name.clone(),
                    commit_hash: blake3::Hash::from_bytes(*new_hash),
                    peer: *signer,
                };

                // Non-blocking send - if channel is full, skip
                if let Err(e) = self.sync_tx.try_send(request) {
                    tracing::warn!(
                        repo_id = %repo_id.to_hex(),
                        ref_name = %ref_name,
                        "failed to queue ref sync request: {}",
                        e
                    );
                } else {
                    tracing::debug!(
                        repo_id = %repo_id.to_hex(),
                        ref_name = %ref_name,
                        "queued ref sync request"
                    );
                }
            }

            Announcement::CobChange {
                repo_id,
                cob_type,
                cob_id,
                change_hash,
            } => {
                let request = SyncRequest::CobChange {
                    repo_id: *repo_id,
                    cob_type: *cob_type,
                    cob_id: blake3::Hash::from_bytes(*cob_id),
                    change_hash: blake3::Hash::from_bytes(*change_hash),
                    peer: *signer,
                };

                if let Err(e) = self.sync_tx.try_send(request) {
                    tracing::warn!(
                        repo_id = %repo_id.to_hex(),
                        cob_type = ?cob_type,
                        "failed to queue COB sync request: {}",
                        e
                    );
                } else {
                    tracing::debug!(
                        repo_id = %repo_id.to_hex(),
                        cob_type = ?cob_type,
                        "queued COB sync request"
                    );
                }
            }

            Announcement::Seeding { repo_id, node_id } => {
                let update = SeedingUpdate {
                    repo_id: *repo_id,
                    peer: *node_id,
                    is_seeding: true,
                };

                if let Err(e) = self.seeding_tx.try_send(update) {
                    tracing::warn!(
                        repo_id = %repo_id.to_hex(),
                        node_id = %node_id,
                        "failed to queue seeding update: {}",
                        e
                    );
                } else {
                    tracing::debug!(
                        repo_id = %repo_id.to_hex(),
                        node_id = %node_id,
                        "queued seeding peer add"
                    );
                }
            }

            Announcement::Unseeding { repo_id, node_id } => {
                let update = SeedingUpdate {
                    repo_id: *repo_id,
                    peer: *node_id,
                    is_seeding: false,
                };

                if let Err(e) = self.seeding_tx.try_send(update) {
                    tracing::warn!(
                        repo_id = %repo_id.to_hex(),
                        node_id = %node_id,
                        "failed to queue unseeding update: {}",
                        e
                    );
                } else {
                    tracing::debug!(
                        repo_id = %repo_id.to_hex(),
                        node_id = %node_id,
                        "queued seeding peer remove"
                    );
                }
            }

            Announcement::RepoCreated {
                repo_id,
                name,
                creator,
            } => {
                // Just log - no automatic action for repo creation
                tracing::info!(
                    repo_id = %repo_id.to_hex(),
                    name = %name,
                    creator = %creator,
                    "discovered new repository"
                );
            }
        }
    }
}

/// Sync worker that processes sync requests.
///
/// This runs in a background task and fetches objects based on queued requests.
pub struct SyncWorker<B: crate::blob::BlobStore> {
    /// Sync service for fetching objects.
    sync: Arc<crate::forge::sync::SyncService<B>>,
    /// Receiver for sync requests.
    rx: mpsc::Receiver<SyncRequest>,
}

impl<B: crate::blob::BlobStore> SyncWorker<B> {
    /// Create a new sync worker.
    pub fn new(
        sync: Arc<crate::forge::sync::SyncService<B>>,
        rx: mpsc::Receiver<SyncRequest>,
    ) -> Self {
        Self { sync, rx }
    }

    /// Run the sync worker until the channel is closed.
    pub async fn run(mut self) {
        while let Some(request) = self.rx.recv().await {
            match request {
                SyncRequest::RefUpdate {
                    repo_id,
                    ref_name,
                    commit_hash,
                    peer,
                } => {
                    tracing::debug!(
                        repo_id = %repo_id.to_hex(),
                        ref_name = %ref_name,
                        commit = %hex::encode(commit_hash.as_bytes()),
                        peer = %peer,
                        "processing ref sync request"
                    );

                    // Fetch the commit and its objects
                    match self.sync.fetch_commits(vec![commit_hash], &[peer]).await {
                        Ok(result) => {
                            if result.is_complete() {
                                tracing::info!(
                                    repo_id = %repo_id.to_hex(),
                                    ref_name = %ref_name,
                                    fetched = result.fetched,
                                    already_present = result.already_present,
                                    "ref sync completed"
                                );
                            } else {
                                tracing::warn!(
                                    repo_id = %repo_id.to_hex(),
                                    ref_name = %ref_name,
                                    missing = result.missing.len(),
                                    errors = result.errors.len(),
                                    "ref sync incomplete"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                repo_id = %repo_id.to_hex(),
                                ref_name = %ref_name,
                                "ref sync failed: {}",
                                e
                            );
                        }
                    }
                }

                SyncRequest::CobChange {
                    repo_id,
                    cob_type,
                    change_hash,
                    peer,
                    ..
                } => {
                    tracing::debug!(
                        repo_id = %repo_id.to_hex(),
                        cob_type = ?cob_type,
                        change = %hex::encode(change_hash.as_bytes()),
                        peer = %peer,
                        "processing COB sync request"
                    );

                    // Fetch the COB change
                    match self.sync.fetch_object(change_hash, &[peer]).await {
                        Ok(true) => {
                            tracing::info!(
                                repo_id = %repo_id.to_hex(),
                                cob_type = ?cob_type,
                                "COB change fetched"
                            );
                        }
                        Ok(false) => {
                            tracing::warn!(
                                repo_id = %repo_id.to_hex(),
                                cob_type = ?cob_type,
                                "COB change not found on peer"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                repo_id = %repo_id.to_hex(),
                                cob_type = ?cob_type,
                                "COB sync failed: {}",
                                e
                            );
                        }
                    }
                }
            }
        }

        tracing::debug!("sync worker shutting down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_creation() {
        let (handler, _sync_rx, _seeding_rx) = ForgeAnnouncementHandler::with_channels(10);

        // Just verify it can be created
        drop(handler);
    }

    #[tokio::test]
    async fn test_seeding_announcement() {
        let (handler, _sync_rx, mut seeding_rx) = ForgeAnnouncementHandler::with_channels(10);

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
        let signer = iroh::SecretKey::generate(&mut rand::rng()).public();

        let announcement = Announcement::Seeding {
            repo_id,
            node_id: signer,
        };

        handler.on_announcement(&announcement, &signer);

        // Check that seeding update was queued
        let update = seeding_rx.try_recv().expect("should receive update");
        assert_eq!(update.repo_id, repo_id);
        assert_eq!(update.peer, signer);
        assert!(update.is_seeding);
    }

    #[tokio::test]
    async fn test_unseeding_announcement() {
        let (handler, _sync_rx, mut seeding_rx) = ForgeAnnouncementHandler::with_channels(10);

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
        let signer = iroh::SecretKey::generate(&mut rand::rng()).public();

        let announcement = Announcement::Unseeding {
            repo_id,
            node_id: signer,
        };

        handler.on_announcement(&announcement, &signer);

        let update = seeding_rx.try_recv().expect("should receive update");
        assert!(!update.is_seeding);
    }

    #[tokio::test]
    async fn test_ref_update_announcement() {
        let (handler, mut sync_rx, _seeding_rx) = ForgeAnnouncementHandler::with_channels(10);

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
        let signer = iroh::SecretKey::generate(&mut rand::rng()).public();

        let announcement = Announcement::RefUpdate {
            repo_id,
            ref_name: "heads/main".to_string(),
            new_hash: [1u8; 32],
            old_hash: None,
        };

        handler.on_announcement(&announcement, &signer);

        let request = sync_rx.try_recv().expect("should receive request");
        match request {
            SyncRequest::RefUpdate {
                repo_id: r,
                ref_name,
                peer,
                ..
            } => {
                assert_eq!(r, repo_id);
                assert_eq!(ref_name, "heads/main");
                assert_eq!(peer, signer);
            }
            _ => panic!("expected RefUpdate request"),
        }
    }

    #[tokio::test]
    async fn test_cob_change_announcement() {
        let (handler, mut sync_rx, _seeding_rx) = ForgeAnnouncementHandler::with_channels(10);

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
        let signer = iroh::SecretKey::generate(&mut rand::rng()).public();

        let announcement = Announcement::CobChange {
            repo_id,
            cob_type: CobType::Issue,
            cob_id: [2u8; 32],
            change_hash: [3u8; 32],
        };

        handler.on_announcement(&announcement, &signer);

        let request = sync_rx.try_recv().expect("should receive request");
        match request {
            SyncRequest::CobChange {
                repo_id: r,
                cob_type,
                peer,
                ..
            } => {
                assert_eq!(r, repo_id);
                assert_eq!(cob_type, CobType::Issue);
                assert_eq!(peer, signer);
            }
            _ => panic!("expected CobChange request"),
        }
    }
}
