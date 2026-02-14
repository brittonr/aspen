//! PijulSyncCallback implementation for PijulSyncHandler.
//!
//! Handles incoming Pijul announcements without blocking the gossip event loop.

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use tracing::debug;
use tracing::info;
use tracing::trace;

use super::PijulSyncHandler;
use super::commands::SyncCommand;
use crate::gossip::PijulAnnouncement;
use crate::sync::PijulSyncCallback;

impl<B: BlobStore + 'static, K: KeyValueStore + ?Sized + 'static> PijulSyncCallback for PijulSyncHandler<B, K> {
    /// Handle an incoming Pijul announcement without blocking.
    ///
    /// This callback is invoked synchronously from the gossip receiver loop. To avoid
    /// blocking the gossip event loop on lock contention, we use `try_write()` to
    /// acquire locks non-blocking. If a lock is contended, we defer the announcement
    /// to the async worker via `SyncCommand::ProcessDeferredAnnouncement`.
    ///
    /// This pattern ensures that:
    /// 1. The gossip receiver never blocks on parking_lot locks
    /// 2. Announcements are never lost (they're queued for later processing)
    /// 3. Lock contention doesn't cause cascading delays in gossip processing
    fn on_announcement(&self, announcement: &PijulAnnouncement, signer: &iroh::PublicKey) {
        let repo_id = *announcement.repo_id();

        // Try to check if we're watching this repo without blocking.
        // If the lock is contended, defer the entire announcement.
        match self.try_is_watching(&repo_id) {
            Some(false) => {
                trace!(repo_id = %repo_id.to_hex(), "ignoring announcement for unwatched repo");
                return;
            }
            None => {
                // Lock contended - defer to async worker
                trace!(repo_id = %repo_id.to_hex(), "deferring announcement due to watched_repos lock contention");
                let _ = self.command_tx.send(SyncCommand::ProcessDeferredAnnouncement {
                    announcement: announcement.clone(),
                    signer: *signer,
                });
                return;
            }
            Some(true) => {
                // We're watching, continue processing
            }
        }

        match announcement {
            PijulAnnouncement::HaveChanges { hashes, offerer, .. } => {
                // Try to acquire the pending lock without blocking
                match self.pending.try_write() {
                    Some(mut pending) => {
                        let mut to_download = Vec::new();

                        for hash in hashes {
                            if pending.should_request(hash) {
                                pending.mark_requested(*hash);
                                to_download.push(*hash);
                            }
                        }

                        drop(pending);

                        if !to_download.is_empty() {
                            debug!(
                                repo_id = %repo_id.to_hex(),
                                count = to_download.len(),
                                offerer = %offerer.fmt_short(),
                                "queueing download of offered changes"
                            );

                            let _ = self.command_tx.send(SyncCommand::DownloadChanges {
                                repo_id,
                                hashes: to_download,
                                provider: *offerer,
                            });
                        }
                    }
                    None => {
                        // Lock contended - defer to async worker
                        trace!(repo_id = %repo_id.to_hex(), "deferring HaveChanges due to pending lock contention");
                        let _ = self.command_tx.send(SyncCommand::ProcessDeferredAnnouncement {
                            announcement: announcement.clone(),
                            signer: *signer,
                        });
                    }
                }
            }

            PijulAnnouncement::WantChanges { hashes, requester, .. } => {
                // WantChanges doesn't need locks, just forward to worker
                debug!(
                    repo_id = %repo_id.to_hex(),
                    count = hashes.len(),
                    requester = %requester.fmt_short(),
                    "received WantChanges, checking if we can help"
                );

                let _ = self.command_tx.send(SyncCommand::RespondWithChanges {
                    repo_id,
                    requested_hashes: hashes.clone(),
                });
            }

            PijulAnnouncement::ChannelUpdate { channel, new_head, .. } => {
                // Try to acquire the pending lock without blocking
                match self.pending.try_write() {
                    Some(mut pending) => {
                        if pending.should_check_channel(&repo_id, channel, new_head) {
                            pending.mark_channel_checked(&repo_id, channel, new_head);
                            drop(pending);

                            info!(
                                repo_id = %repo_id.to_hex(),
                                channel = %channel,
                                new_head = %new_head,
                                "handler: received ChannelUpdate, queueing sync check"
                            );

                            let _ = self.command_tx.send(SyncCommand::CheckChannelSync {
                                repo_id,
                                channel: channel.clone(),
                                remote_head: *new_head,
                                announcer: *signer,
                            });
                        } else {
                            trace!(
                                repo_id = %repo_id.to_hex(),
                                channel = %channel,
                                "handler: skipping ChannelUpdate, already checked recently"
                            );
                        }
                    }
                    None => {
                        // Lock contended - defer to async worker
                        trace!(repo_id = %repo_id.to_hex(), "deferring ChannelUpdate due to pending lock contention");
                        let _ = self.command_tx.send(SyncCommand::ProcessDeferredAnnouncement {
                            announcement: announcement.clone(),
                            signer: *signer,
                        });
                    }
                }
            }

            PijulAnnouncement::ChangeAvailable { change_hash, .. } => {
                // Try to acquire the pending lock without blocking
                match self.pending.try_write() {
                    Some(mut pending) => {
                        if pending.should_request(change_hash) {
                            pending.mark_requested(*change_hash);
                            drop(pending);

                            debug!(
                                repo_id = %repo_id.to_hex(),
                                hash = %change_hash,
                                "received ChangeAvailable, requesting change"
                            );

                            let _ = self.command_tx.send(SyncCommand::RequestChange {
                                repo_id,
                                change_hash: *change_hash,
                                provider: *signer,
                            });
                        }
                    }
                    None => {
                        // Lock contended - defer to async worker
                        trace!(repo_id = %repo_id.to_hex(), "deferring ChangeAvailable due to pending lock contention");
                        let _ = self.command_tx.send(SyncCommand::ProcessDeferredAnnouncement {
                            announcement: announcement.clone(),
                            signer: *signer,
                        });
                    }
                }
            }

            // These announcements don't require sync actions
            PijulAnnouncement::Seeding { node_id, channels, .. } => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    node_id = %node_id.fmt_short(),
                    channels = ?channels,
                    "peer announced seeding"
                );
            }

            PijulAnnouncement::Unseeding { node_id, .. } => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    node_id = %node_id.fmt_short(),
                    "peer announced unseeding"
                );
            }

            PijulAnnouncement::RepoCreated {
                name,
                creator,
                default_channel,
                ..
            } => {
                info!(
                    repo_id = %repo_id.to_hex(),
                    name = %name,
                    creator = %creator.fmt_short(),
                    default_channel = %default_channel,
                    "new repository created"
                );
            }
        }
    }
}
