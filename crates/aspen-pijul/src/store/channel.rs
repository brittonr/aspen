//! Channel management operations for PijulStore.

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;
use tracing::info;
use tracing::instrument;

use super::PijulStore;
use crate::constants::MAX_CHANNELS;
use crate::error::PijulError;
use crate::error::PijulResult;
use crate::types::Channel;

impl<B: BlobStore, K: KeyValueStore + ?Sized> PijulStore<B, K> {
    /// Create a new channel in a repository.
    #[instrument(skip(self))]
    pub async fn create_channel(&self, repo_id: &RepoId, channel: &str) -> PijulResult<()> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Check channel count limit
        let count = self.refs.count_channels(repo_id).await?;
        if count >= MAX_CHANNELS {
            return Err(PijulError::TooManyChannels {
                count,
                max: MAX_CHANNELS,
            });
        }

        // Check if channel already exists
        if self.refs.channel_exists(repo_id, channel).await? {
            return Err(PijulError::ChannelAlreadyExists {
                channel: channel.to_string(),
            });
        }

        // Create the empty channel in the ref store
        self.refs.create_empty_channel(repo_id, channel).await?;

        info!(repo_id = %repo_id, channel = channel, "created channel");
        Ok(())
    }

    /// Get a channel's current state.
    ///
    /// Returns `Some(Channel)` if the channel exists (with or without a head),
    /// or `None` if the channel doesn't exist.
    #[instrument(skip(self))]
    pub async fn get_channel(&self, repo_id: &RepoId, channel: &str) -> PijulResult<Option<Channel>> {
        // First check if channel exists
        if !self.refs.channel_exists(repo_id, channel).await? {
            return Ok(None);
        }

        // Channel exists, get its head (may be None for empty channels)
        let head = self.refs.get_channel(repo_id, channel).await?;

        Ok(Some(Channel {
            name: channel.to_string(),
            head,
            updated_at_ms: chrono::Utc::now().timestamp_millis() as u64,
        }))
    }

    /// Get a channel using local (stale) read consistency.
    ///
    /// This reads from the local state without requiring the Raft leader,
    /// making it suitable for use on follower nodes in sync handlers.
    /// The data may be slightly stale but will be eventually consistent.
    #[instrument(skip(self))]
    pub async fn get_channel_local(&self, repo_id: &RepoId, channel: &str) -> PijulResult<Option<Channel>> {
        use aspen_core::ReadConsistency;

        // Use stale read for channel exists check too
        let head = self.refs.get_channel_with_consistency(repo_id, channel, ReadConsistency::Stale).await?;

        // If we got a head, channel exists
        // If None, we can't distinguish between empty channel and non-existent channel with stale reads
        // For sync purposes, treat None as "no local data to compare"
        match head {
            Some(h) => Ok(Some(Channel {
                name: channel.to_string(),
                head: Some(h),
                updated_at_ms: chrono::Utc::now().timestamp_millis() as u64,
            })),
            None => Ok(None),
        }
    }

    /// List all channels in a repository.
    pub async fn list_channels(&self, repo_id: &RepoId) -> PijulResult<Vec<Channel>> {
        let channel_heads = self.refs.list_channels(repo_id).await?;

        Ok(channel_heads
            .into_iter()
            .map(|(name, head)| Channel {
                name,
                head,
                updated_at_ms: chrono::Utc::now().timestamp_millis() as u64,
            })
            .collect())
    }

    /// Delete a channel from a repository.
    ///
    /// This removes the channel head reference from the KV store.
    /// The changes themselves are not deleted (they may be referenced by other channels).
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to delete
    #[instrument(skip(self))]
    pub async fn delete_channel(&self, repo_id: &RepoId, channel: &str) -> PijulResult<()> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Check if channel exists
        if !self.refs.channel_exists(repo_id, channel).await? {
            return Err(PijulError::ChannelNotFound {
                channel: channel.to_string(),
            });
        }

        // Delete the channel
        self.refs.delete_channel(repo_id, channel).await?;

        info!(repo_id = %repo_id, channel = channel, "deleted channel");
        Ok(())
    }

    /// Fork a channel, creating a new channel with the same head.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `source`: Source channel name to fork from
    /// - `target`: Target channel name to create
    #[instrument(skip(self))]
    pub async fn fork_channel(&self, repo_id: &RepoId, source: &str, target: &str) -> PijulResult<Channel> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Check channel count limit
        let count = self.refs.count_channels(repo_id).await?;
        if count >= MAX_CHANNELS {
            return Err(PijulError::TooManyChannels {
                count,
                max: MAX_CHANNELS,
            });
        }

        // Check if target already exists
        if self.refs.channel_exists(repo_id, target).await? {
            return Err(PijulError::ChannelAlreadyExists {
                channel: target.to_string(),
            });
        }

        // Check source channel exists
        if !self.refs.channel_exists(repo_id, source).await? {
            return Err(PijulError::ChannelNotFound {
                channel: source.to_string(),
            });
        }

        // Get source channel's head (may be None for empty channels)
        let source_head = self.refs.get_channel(repo_id, source).await?;

        // Create target channel with the same head (or empty if source is empty)
        match source_head {
            Some(head) => {
                self.refs.set_channel(repo_id, target, head).await?;
                info!(repo_id = %repo_id, source = source, target = target, head = %head, "forked channel");
                Ok(Channel::with_head(target, head))
            }
            None => {
                self.refs.create_empty_channel(repo_id, target).await?;
                info!(repo_id = %repo_id, source = source, target = target, "forked empty channel");
                Ok(Channel::new(target))
            }
        }
    }
}
