//! Trust-aware secrets-at-rest integration.
//!
//! Bridges committed trust epochs to the secrets backend by:
//! - reconstructing the current epoch secret on demand for ordinary secrets access,
//! - deriving epoch-scoped at-rest keys,
//! - carrying prior epoch keys forward for mixed-epoch reads,
//! - starting background re-encryption when the trust epoch advances,
//! - and resuming interrupted re-encryption at startup when a checkpoint is present.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_trust::key_manager::ShareCollector;
use aspen_trust::reencrypt::ReencryptionStore;
use async_trait::async_trait;
use base64::Engine;
use tokio::task::JoinSet;
use tracing::info;
use tracing::warn;

use crate::StateMachineVariant;
use crate::node::RaftNode;
use crate::storage_shared::SharedRedbStorage;

const SHARE_COLLECTION_TIMEOUT: Duration = Duration::from_secs(10);

struct RaftShareCollector {
    node: Arc<RaftNode>,
}

impl RaftShareCollector {
    fn new(node: Arc<RaftNode>) -> Self {
        Self { node }
    }
}

struct TrustAwareEncryptionProvider {
    node: Arc<RaftNode>,
    collector: Arc<RaftShareCollector>,
    key_manager: Arc<aspen_trust::key_manager::KeyManager>,
}

struct RaftReencryptionStore {
    kv: Arc<dyn aspen_core::KeyValueStore>,
    storage: Arc<SharedRedbStorage>,
}

impl RaftReencryptionStore {
    fn new(kv: Arc<dyn aspen_core::KeyValueStore>, storage: Arc<SharedRedbStorage>) -> Self {
        Self { kv, storage }
    }
}

fn trust_storage(node: &RaftNode) -> Result<Arc<SharedRedbStorage>, String> {
    let StateMachineVariant::Redb(storage) = node.state_machine() else {
        return Err("trust requires redb storage".to_string());
    };
    Ok(storage.clone())
}

fn effective_trust_epoch(storage: &SharedRedbStorage, fallback_epoch: u64) -> u64 {
    match storage.load_current_trust_epoch() {
        Ok(Some(epoch)) => epoch,
        Ok(None) => match storage.load_digests(1) {
            Ok(digests) if !digests.is_empty() => 1,
            _ => fallback_epoch,
        },
        Err(_) => fallback_epoch,
    }
}

fn resolve_threshold(member_count: usize, threshold_override: Option<u8>) -> Result<u8, String> {
    let count_u32 = u32::try_from(member_count).map_err(|_| "too many trust members".to_string())?;
    let threshold = match threshold_override {
        Some(value) => {
            aspen_trust::secret::Threshold::new(value).ok_or_else(|| "threshold must be >= 1".to_string())?
        }
        None => aspen_trust::secret::Threshold::default_for_cluster_size(count_u32),
    };
    if usize::from(threshold.value()) > member_count {
        return Err(format!("threshold {} exceeds cluster size {member_count}", threshold.value()));
    }
    Ok(threshold.value())
}

fn collection_failed(reason: impl Into<String>) -> aspen_trust::key_manager::ShareCollectionError {
    aspen_trust::key_manager::ShareCollectionError::CollectionFailed { reason: reason.into() }
}

struct ShareCollectionContext {
    members: BTreeMap<u64, iroh::EndpointAddr>,
    digests: BTreeMap<u64, aspen_trust::shamir::ShareDigest>,
    threshold: u8,
    local_share: Option<aspen_trust::shamir::Share>,
}

type RemoteShareResponse =
    (u64, Option<aspen_trust::shamir::ShareDigest>, anyhow::Result<aspen_trust::protocol::ShareResponse>);

fn load_share_collection_context(
    storage: &SharedRedbStorage,
    epoch: u64,
) -> Result<ShareCollectionContext, aspen_trust::key_manager::ShareCollectionError> {
    let members = storage
        .load_members(epoch)
        .map_err(|error| collection_failed(format!("failed to load trust members: {error}")))?;
    let threshold_override = storage
        .load_trust_threshold_override()
        .map_err(|error| collection_failed(format!("failed to load trust threshold override: {error}")))?;
    let threshold = resolve_threshold(members.len(), threshold_override).map_err(collection_failed)?;
    let digests = storage
        .load_digests(epoch)
        .map_err(|error| collection_failed(format!("failed to load trust digests: {error}")))?;
    let local_share = storage
        .load_share(epoch)
        .map_err(|error| collection_failed(format!("failed to load local trust share: {error}")))?;

    Ok(ShareCollectionContext {
        members,
        digests,
        threshold,
        local_share,
    })
}

fn spawn_remote_share_requests(
    join_set: &mut JoinSet<RemoteShareResponse>,
    client: Arc<dyn crate::trust_share_client::TrustShareClient>,
    members: BTreeMap<u64, iroh::EndpointAddr>,
    digests: &BTreeMap<u64, aspen_trust::shamir::ShareDigest>,
    local_node_id: u64,
    epoch: u64,
) {
    for (node_id, endpoint) in members {
        if node_id == local_node_id {
            continue;
        }
        let client = client.clone();
        let expected_digest = digests.get(&node_id).copied();
        join_set.spawn(async move {
            let response = client.get_share(endpoint, epoch).await;
            (node_id, expected_digest, response)
        });
    }
}

fn try_collect_remote_share(
    collected: &[aspen_trust::shamir::Share],
    expected_digest: Option<aspen_trust::shamir::ShareDigest>,
    response: aspen_trust::protocol::ShareResponse,
    epoch: u64,
) -> Option<aspen_trust::shamir::Share> {
    if response.epoch != epoch {
        return None;
    }
    if let Some(expected_digest) = expected_digest {
        let actual = aspen_trust::shamir::share_digest(&response.share);
        if actual != expected_digest {
            return None;
        }
    }
    let share_digest = aspen_trust::shamir::share_digest(&response.share);
    if collected.iter().any(|share| aspen_trust::shamir::share_digest(share) == share_digest) {
        return None;
    }
    Some(response.share)
}

async fn collect_remote_shares_until_quorum(
    join_set: &mut JoinSet<RemoteShareResponse>,
    collected: &mut Vec<aspen_trust::shamir::Share>,
    threshold: u8,
    epoch: u64,
) -> Result<(), aspen_trust::key_manager::ShareCollectionError> {
    let deadline = tokio::time::sleep(SHARE_COLLECTION_TIMEOUT);
    tokio::pin!(deadline);
    while collected.len() < usize::from(threshold) && !join_set.is_empty() {
        tokio::select! {
            _ = &mut deadline => break,
            joined = join_set.join_next() => {
                let Some(joined) = joined else {
                    continue;
                };
                let (_node_id, expected_digest, response) = joined
                    .map_err(|error| collection_failed(format!("trust share task failed: {error}")))?;
                let response = response.map_err(|error| collection_failed(error.to_string()))?;
                if let Some(share) = try_collect_remote_share(collected, expected_digest, response, epoch) {
                    collected.push(share);
                }
            }
        }
    }
    Ok(())
}

fn ensure_share_quorum(
    collected: &[aspen_trust::shamir::Share],
    threshold: u8,
) -> Result<(), aspen_trust::key_manager::ShareCollectionError> {
    if collected.len() >= usize::from(threshold) {
        return Ok(());
    }

    Err(aspen_trust::key_manager::ShareCollectionError::BelowQuorum {
        collected: u32::try_from(collected.len()).unwrap_or(u32::MAX),
        threshold: u32::from(threshold),
    })
}

async fn sync_key_manager_to_current_epoch(
    node: &Arc<RaftNode>,
    collector: &Arc<RaftShareCollector>,
    key_manager: &Arc<aspen_trust::key_manager::KeyManager>,
) -> Result<Arc<aspen_trust::encryption::SecretsEncryption>, String> {
    let storage = trust_storage(node.as_ref())?;
    let current_epoch = effective_trust_epoch(storage.as_ref(), key_manager.epoch());

    if let Some(enc) = key_manager.get_if_initialized().await
        && enc.epoch() == current_epoch
    {
        return Ok(enc);
    }

    let shares = collector.collect_shares(current_epoch).await.map_err(|e| e.to_string())?;
    let current_secret =
        aspen_trust::shamir::reconstruct_secret(&shares).map_err(|e| format!("failed to reconstruct secret: {e}"))?;

    let prior_secrets = if current_epoch > 1 {
        let chain = storage
            .load_encrypted_chain(current_epoch)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing encrypted chain for epoch {current_epoch}"))?;
        let cluster_id = node.trust_cluster_id().ok_or_else(|| "trust cluster id is not configured".to_string())?;
        aspen_trust::chain::decrypt_chain(&chain, &current_secret, cluster_id)
            .map_err(|e| format!("failed to decrypt historical secret chain: {e}"))?
    } else {
        BTreeMap::new()
    };

    Ok(key_manager.rotate_epoch_with_prior_secrets(&current_secret, current_epoch, &prior_secrets).await)
}

async fn find_oldest_outdated_epoch(
    store: &RaftReencryptionStore,
    prefix: &str,
    current_epoch: u64,
) -> Result<Option<u64>, aspen_trust::reencrypt::ReencryptionError> {
    let mut after_key: Option<String> = None;
    let mut oldest_epoch: Option<u64> = None;

    loop {
        let batch = store.scan_secrets(prefix, after_key.as_deref(), 100).await?;
        if batch.is_empty() {
            return Ok(oldest_epoch);
        }

        for (key, value) in &batch {
            if let Ok(epoch) = aspen_trust::encryption::SecretsEncryption::peek_epoch(value)
                && epoch < current_epoch
            {
                oldest_epoch = Some(oldest_epoch.map_or(epoch, |existing| existing.min(epoch)));
            }
            after_key = Some(key.clone());
        }
    }
}

async fn reconcile_pending_reencryption(
    node: &Arc<RaftNode>,
    kv: &Arc<dyn aspen_core::KeyValueStore>,
    collector: &Arc<RaftShareCollector>,
    key_manager: &Arc<aspen_trust::key_manager::KeyManager>,
) -> Result<(), String> {
    let storage = trust_storage(node.as_ref())?;
    let store = RaftReencryptionStore::new(kv.clone(), storage.clone());

    loop {
        let current_epoch = effective_trust_epoch(storage.as_ref(), key_manager.epoch());
        if current_epoch <= 1 {
            return Ok(());
        }

        let checkpoint = store
            .load_checkpoint(aspen_secrets::SECRETS_SYSTEM_PREFIX)
            .await
            .map_err(|error| error.to_string())?;
        let old_epoch = find_oldest_outdated_epoch(&store, aspen_secrets::SECRETS_SYSTEM_PREFIX, current_epoch)
            .await
            .map_err(|error| error.to_string())?
            .or_else(|| checkpoint.as_ref().map(|_| current_epoch.saturating_sub(1)));

        let Some(old_epoch) = old_epoch else {
            return Ok(());
        };

        let encryption = sync_key_manager_to_current_epoch(node, collector, key_manager).await?;
        let result = aspen_trust::reencrypt::reencrypt_secrets(
            &store,
            encryption.as_ref(),
            old_epoch,
            aspen_secrets::SECRETS_SYSTEM_PREFIX,
        )
        .await
        .map_err(|error| error.to_string())?;

        info!(
            old_epoch,
            current_epoch,
            keys_reencrypted = result.keys_reencrypted,
            keys_skipped = result.keys_skipped,
            checkpoint_resumed = checkpoint.is_some(),
            "completed secrets re-encryption reconciliation"
        );
    }
}

fn spawn_reencryption_watcher(
    node: Arc<RaftNode>,
    kv: Arc<dyn aspen_core::KeyValueStore>,
    collector: Arc<RaftShareCollector>,
    key_manager: Arc<aspen_trust::key_manager::KeyManager>,
) {
    tokio::spawn(async move {
        let Ok(storage) = trust_storage(node.as_ref()) else {
            return;
        };
        let mut last_epoch = effective_trust_epoch(storage.as_ref(), 0);
        if let Err(error) = reconcile_pending_reencryption(&node, &kv, &collector, &key_manager).await {
            warn!(error, "failed to reconcile pending secrets re-encryption at startup");
        }
        last_epoch = effective_trust_epoch(storage.as_ref(), last_epoch);
        let mut interval = tokio::time::interval(Duration::from_millis(50));

        loop {
            interval.tick().await;

            let current_epoch = effective_trust_epoch(storage.as_ref(), 0);
            let has_checkpoint = storage
                .load_reencryption_checkpoint(aspen_secrets::SECRETS_SYSTEM_PREFIX)
                .map(|checkpoint| checkpoint.is_some())
                .unwrap_or(false);

            if last_epoch == 0 {
                last_epoch = current_epoch;
                continue;
            }
            if current_epoch <= last_epoch && !has_checkpoint {
                continue;
            }

            if let Err(error) = reconcile_pending_reencryption(&node, &kv, &collector, &key_manager).await {
                warn!(current_epoch, error, "secrets re-encryption reconciliation failed");
                continue;
            }
            last_epoch = effective_trust_epoch(storage.as_ref(), current_epoch);
        }
    });
}

/// Return whether runtime trust wiring exists for trust-aware secrets-at-rest.
///
/// When the `trust` feature is compiled but the node was started without runtime
/// trust configuration, callers should leave the secrets service in plaintext mode.
pub fn has_runtime_trust_configuration(node: &RaftNode) -> bool {
    node.trust_cluster_id().is_some() && node.trust_share_client().is_some()
}

fn build_key_manager(
    node: &Arc<RaftNode>,
    storage: &Arc<SharedRedbStorage>,
) -> Result<Arc<aspen_trust::key_manager::KeyManager>, String> {
    let cluster_id = node.trust_cluster_id().ok_or_else(|| "trust cluster id is not configured".to_string())?.to_vec();
    let node_id = u32::try_from(node.node_id().0).map_err(|_| format!("node id {} exceeds u32", node.node_id().0))?;
    let initial_nonce_counter = storage.load_nonce_counter(node.node_id().0).map_err(|e| e.to_string())?.unwrap_or(0);
    let epoch = storage.load_current_trust_epoch().map_err(|e| e.to_string())?.unwrap_or(1);

    Ok(Arc::new(aspen_trust::key_manager::KeyManager::new(aspen_trust::key_manager::KeyManagerConfig {
        cluster_id,
        node_id,
        initial_nonce_counter,
        epoch,
    })))
}

#[cfg(test)]
pub(crate) async fn load_current_secrets_encryption_for_tests(
    node: Arc<RaftNode>,
) -> Result<Arc<aspen_trust::encryption::SecretsEncryption>, String> {
    let storage = trust_storage(node.as_ref())?;
    let key_manager = build_key_manager(&node, &storage)?;
    let collector = Arc::new(RaftShareCollector::new(node.clone()));
    sync_key_manager_to_current_epoch(&node, &collector, &key_manager).await
}

/// Build a trust-aware secrets encryption provider and start epoch watching.
pub fn build_trust_aware_secrets_provider(
    node: Arc<RaftNode>,
    kv: Arc<dyn aspen_core::KeyValueStore>,
) -> Result<Arc<dyn aspen_secrets::SecretsEncryptionProvider>, String> {
    let storage = trust_storage(node.as_ref())?;
    let key_manager = build_key_manager(&node, &storage)?;
    let collector = Arc::new(RaftShareCollector::new(node.clone()));
    let provider: Arc<dyn aspen_secrets::SecretsEncryptionProvider> = Arc::new(TrustAwareEncryptionProvider {
        node: node.clone(),
        collector: collector.clone(),
        key_manager: key_manager.clone(),
    });

    spawn_reencryption_watcher(node, kv, collector, key_manager);
    Ok(provider)
}

#[async_trait]
impl aspen_trust::key_manager::ShareCollector for RaftShareCollector {
    async fn collect_shares(
        &self,
        epoch: u64,
    ) -> Result<Vec<aspen_trust::shamir::Share>, aspen_trust::key_manager::ShareCollectionError> {
        let storage = trust_storage(self.node.as_ref()).map_err(collection_failed)?;
        let client = self
            .node
            .trust_share_client()
            .cloned()
            .ok_or_else(|| collection_failed("trust share client is not configured"))?;
        let context = load_share_collection_context(storage.as_ref(), epoch)?;

        let mut collected = Vec::new();
        if let Some(local_share) = context.local_share {
            collected.push(local_share);
        }

        let mut join_set = JoinSet::new();
        spawn_remote_share_requests(
            &mut join_set,
            client,
            context.members,
            &context.digests,
            self.node.node_id().0,
            epoch,
        );
        collect_remote_shares_until_quorum(&mut join_set, &mut collected, context.threshold, epoch).await?;
        ensure_share_quorum(&collected, context.threshold)?;
        Ok(collected)
    }
}

#[async_trait]
impl aspen_secrets::SecretsEncryptionProvider for TrustAwareEncryptionProvider {
    async fn get_encryption(
        &self,
    ) -> Result<Arc<aspen_trust::encryption::SecretsEncryption>, aspen_trust::encryption::SecretsUnavailableError> {
        sync_key_manager_to_current_epoch(&self.node, &self.collector, &self.key_manager)
            .await
            .map_err(|error| {
                warn!(error, "failed to initialize or rotate secrets-at-rest encryption context");
                aspen_trust::encryption::SecretsUnavailableError::BelowQuorum
            })
    }
}

#[async_trait]
impl aspen_trust::reencrypt::ReencryptionStore for RaftReencryptionStore {
    async fn scan_secrets(
        &self,
        prefix: &str,
        after_key: Option<&str>,
        batch_size: u32,
    ) -> Result<Vec<(String, Vec<u8>)>, aspen_trust::reencrypt::ReencryptionError> {
        let result = self
            .kv
            .scan(aspen_kv_types::ScanRequest {
                prefix: prefix.to_string(),
                limit_results: Some(batch_size),
                continuation_token: after_key.map(str::to_string),
            })
            .await
            .map_err(|error| aspen_trust::reencrypt::ReencryptionError::Storage {
                reason: format!("scan failed: {error}"),
            })?;

        result
            .entries
            .into_iter()
            .map(|entry| {
                base64::engine::general_purpose::STANDARD
                    .decode(entry.value)
                    .map(|value| (entry.key, value))
                    .map_err(|error| aspen_trust::reencrypt::ReencryptionError::Storage {
                        reason: format!("base64 decode failed: {error}"),
                    })
            })
            .collect()
    }

    async fn write_reencrypted(
        &self,
        key: &str,
        value: &[u8],
    ) -> Result<(), aspen_trust::reencrypt::ReencryptionError> {
        self.kv
            .write(aspen_kv_types::WriteRequest {
                command: aspen_kv_types::WriteCommand::Set {
                    key: key.to_string(),
                    value: base64::engine::general_purpose::STANDARD.encode(value),
                },
            })
            .await
            .map_err(|error| aspen_trust::reencrypt::ReencryptionError::Storage {
                reason: format!("write failed: {error}"),
            })?;
        Ok(())
    }

    async fn save_checkpoint(
        &self,
        table_name: &str,
        last_key: &str,
    ) -> Result<(), aspen_trust::reencrypt::ReencryptionError> {
        self.storage.save_reencryption_checkpoint(table_name, last_key).map_err(|error| {
            aspen_trust::reencrypt::ReencryptionError::Storage {
                reason: format!("checkpoint save failed: {error}"),
            }
        })
    }

    async fn load_checkpoint(
        &self,
        table_name: &str,
    ) -> Result<Option<String>, aspen_trust::reencrypt::ReencryptionError> {
        self.storage.load_reencryption_checkpoint(table_name).map_err(|error| {
            aspen_trust::reencrypt::ReencryptionError::Storage {
                reason: format!("checkpoint load failed: {error}"),
            }
        })
    }

    async fn clear_checkpoint(&self, table_name: &str) -> Result<(), aspen_trust::reencrypt::ReencryptionError> {
        self.storage.clear_reencryption_checkpoint(table_name).map_err(|error| {
            aspen_trust::reencrypt::ReencryptionError::Storage {
                reason: format!("checkpoint clear failed: {error}"),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_share(x: u8, fill: u8) -> aspen_trust::shamir::Share {
        aspen_trust::shamir::Share {
            x,
            y: [fill; aspen_trust::shamir::SECRET_SIZE],
        }
    }

    #[test]
    fn test_try_collect_remote_share_rejects_wrong_epoch() {
        let share = test_share(1, 0x11);
        let response = aspen_trust::protocol::ShareResponse {
            epoch: 3,
            current_epoch: 3,
            share,
        };
        assert!(try_collect_remote_share(&[], None, response, 4).is_none());
    }

    #[test]
    fn test_try_collect_remote_share_rejects_duplicate_digest() {
        let share = test_share(2, 0x22);
        let response = aspen_trust::protocol::ShareResponse {
            epoch: 7,
            current_epoch: 7,
            share: share.clone(),
        };
        let collected = vec![share];
        assert!(try_collect_remote_share(&collected, None, response, 7).is_none());
    }

    #[test]
    fn test_try_collect_remote_share_accepts_matching_digest() {
        let share = test_share(3, 0x33);
        let digest = aspen_trust::shamir::share_digest(&share);
        let response = aspen_trust::protocol::ShareResponse {
            epoch: 9,
            current_epoch: 9,
            share: share.clone(),
        };
        let collected = Vec::new();
        assert_eq!(try_collect_remote_share(&collected, Some(digest), response, 9), Some(share));
    }

    #[test]
    fn test_ensure_share_quorum_reports_below_quorum() {
        let error = ensure_share_quorum(&[test_share(1, 0x44)], 2).expect_err("quorum check must fail");
        let aspen_trust::key_manager::ShareCollectionError::BelowQuorum { collected, threshold } = error else {
            panic!("expected BelowQuorum error");
        };
        assert_eq!(collected, 1);
        assert_eq!(threshold, 2);
    }
}
