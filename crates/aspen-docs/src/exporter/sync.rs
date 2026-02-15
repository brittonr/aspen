//! Full sync operations for the DocsExporter.
//!
//! Provides periodic full-sync from a KV store for drift correction,
//! ensuring eventual consistency between the KV store and iroh-docs.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_core::KeyValueStore;
use aspen_core::ScanRequest;
use aspen_raft::log_subscriber::LogEntryPayload;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;

use super::core::BatchEntry;
use super::core::DocsExporter;
use crate::constants::BACKGROUND_SYNC_INTERVAL;
use crate::constants::EXPORT_BATCH_SIZE;
use crate::constants::MAX_DOC_KEY_SIZE;
use crate::constants::MAX_DOC_VALUE_SIZE;

impl DocsExporter {
    /// Start exporting with periodic background full-sync for drift correction.
    ///
    /// In addition to real-time export, this spawns a background task that
    /// periodically scans all KV entries and exports them to ensure consistency.
    /// This catches any entries that may have been missed due to lag or restarts.
    ///
    /// # Arguments
    /// * `receiver` - Broadcast receiver for real-time log entries
    /// * `kv_store` - Reference to the KV store for scanning all entries
    pub fn spawn_with_full_sync<KV>(
        self: Arc<Self>,
        receiver: broadcast::Receiver<LogEntryPayload>,
        kv_store: Arc<KV>,
    ) -> CancellationToken
    where
        KV: KeyValueStore + 'static,
    {
        // Spawn the real-time exporter with batching
        let cancel = self.clone().spawn(receiver);

        // Spawn background full-sync task
        let exporter = self.clone();
        let cancel_clone = self.cancel.clone();

        tokio::spawn(async move {
            info!(interval_secs = BACKGROUND_SYNC_INTERVAL.as_secs(), "background full-sync started");

            let mut sync_interval = tokio::time::interval(BACKGROUND_SYNC_INTERVAL);
            sync_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Skip the first immediate tick
            sync_interval.tick().await;

            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        info!("background full-sync shutting down");
                        break;
                    }
                    _ = sync_interval.tick() => {
                        info!("starting periodic full-sync");
                        match exporter.full_sync_from_kv(&kv_store).await {
                            Ok(count) => {
                                info!(exported = count, "periodic full-sync completed");
                            }
                            Err(e) => {
                                error!(error = %e, "periodic full-sync failed");
                            }
                        }
                    }
                }
            }
        });

        cancel
    }

    /// Perform a full sync from a KeyValueStore implementation.
    ///
    /// Scans all keys in the KV store and exports them to docs.
    /// Used for initial population and periodic drift correction.
    pub async fn full_sync_from_kv<KV>(&self, kv_store: &KV) -> Result<u64>
    where KV: KeyValueStore {
        info!("starting full sync from KV store");

        let mut exported = 0u64;
        let mut continuation_token: Option<String> = None;

        loop {
            // Scan a batch of keys
            let result = kv_store
                .scan(ScanRequest {
                    prefix: String::new(), // All keys
                    limit: Some(EXPORT_BATCH_SIZE),
                    continuation_token: continuation_token.clone(),
                })
                .await
                .context("failed to scan KV store")?;

            if result.entries.is_empty() {
                break;
            }

            // Collect entries for batch write
            let batch_entries: Vec<BatchEntry> = result
                .entries
                .iter()
                .filter(|entry| entry.key.len() <= MAX_DOC_KEY_SIZE && entry.value.len() <= MAX_DOC_VALUE_SIZE)
                .map(|entry| BatchEntry {
                    key: entry.key.as_bytes().to_vec(),
                    value: entry.value.as_bytes().to_vec(),
                    is_delete: false,
                })
                .collect();

            let batch_count = batch_entries.len() as u64;

            // Write batch
            self.writer.write_batch(batch_entries).await?;

            // Emit event for this batch
            if let Some(broadcaster) = &self.event_broadcaster {
                broadcaster.emit_entry_exported(batch_count as u32, batch_count);
            }

            exported += batch_count;

            // Check for more pages
            if !result.is_truncated {
                break;
            }
            continuation_token = result.continuation_token;
        }

        info!(exported, "full sync complete");
        Ok(exported)
    }
}
