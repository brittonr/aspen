//! Federation resource resolver for Forge repositories.
//!
//! Translates federation sync requests into reads against the Forge KV layout.
//! Maps `FederatedId` → repo refs via `forge:refs:{repo_id}:{ref_name}` keys
//! and checks federation settings via `forge:federation:settings:{fed_id}`.
//!
//! Optionally serves git objects (commits, trees, blobs) by walking the commit
//! DAG via a `GitObjectExporter` — used for federation content transfer.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use aspen_cluster::federation::FederatedId;
use aspen_cluster::federation::FederationMode;
use aspen_cluster::federation::FederationResourceError;
use aspen_cluster::federation::FederationResourceResolver;
use aspen_cluster::federation::FederationResourceState;
use aspen_cluster::federation::FederationSettings;
use aspen_cluster::federation::sync::ResourceMetadata;
use aspen_cluster::federation::sync::SyncObject;
use aspen_core::KeyValueStore;
use aspen_core::ScanRequest;
use async_trait::async_trait;
use tracing::debug;
#[cfg(feature = "git-bridge")]
use tracing::warn;

use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;
use crate::constants::KV_PREFIX_REFS;
use crate::constants::KV_PREFIX_REPOS;
#[cfg(feature = "git-bridge")]
use crate::git::bridge::FederationExportResult;
#[cfg(feature = "git-bridge")]
use crate::identity::RepoId;

/// Maximum refs returned per resource state query.
const MAX_REFS_PER_QUERY: u32 = 1000;

/// Maximum git objects returned per sync request.
const MAX_GIT_OBJECTS_PER_SYNC: u32 = 1000;

/// Trait for exporting git objects from a forge repo for federation.
///
/// Abstraction over `GitExporter` to avoid leaking blob store generics
/// into the resolver. Implementations walk the commit DAG from a ref head
/// and return objects the remote doesn't have.
#[cfg(feature = "git-bridge")]
#[async_trait]
pub trait GitObjectExporter: Send + Sync {
    /// Export git objects reachable from `commit_blake3`, excluding objects
    /// whose BLAKE3 hashes appear in `known_blake3`.
    ///
    /// Returns at most `limit` objects in dependency order (blobs first),
    /// plus a flag indicating whether the DAG was fully traversed.
    async fn export_dag_blake3(
        &self,
        repo_id: &RepoId,
        commit_blake3: blake3::Hash,
        known_blake3: &HashSet<[u8; 32]>,
        limit: usize,
    ) -> Result<FederationExportResult, String>;
}

/// Blanket implementation for `GitExporter<K, B>`.
#[cfg(feature = "git-bridge")]
#[async_trait]
impl<K, B> GitObjectExporter for crate::git::bridge::GitExporter<K, B>
where
    K: KeyValueStore + Send + Sync + ?Sized + 'static,
    B: aspen_blob::prelude::BlobStore + Send + Sync + 'static,
{
    async fn export_dag_blake3(
        &self,
        repo_id: &RepoId,
        commit_blake3: blake3::Hash,
        known_blake3: &HashSet<[u8; 32]>,
        limit: usize,
    ) -> Result<FederationExportResult, String> {
        self.export_commit_dag_blake3(repo_id, commit_blake3, known_blake3, limit)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Forge-specific federation resource resolver.
///
/// Reads ref heads from the Forge KV layout (`forge:refs:{repo_id}:{ref_name}`)
/// and checks federation settings from `forge:federation:settings:{fed_id}`.
///
/// Optionally holds a `GitObjectExporter` for serving git objects (commits,
/// trees, blobs) during federation sync. Without it, only ref entries are served.
///
/// The `FederatedId.local_id` is treated as a 32-byte repo identity hash.
/// This maps directly to `RepoId` which is also a `[u8; 32]`.
pub struct ForgeResourceResolver<K: KeyValueStore + ?Sized> {
    kv: Arc<K>,
    /// Optional git object exporter for serving commit/tree/blob data.
    #[cfg(feature = "git-bridge")]
    git_exporter: Option<Arc<dyn GitObjectExporter>>,
}

impl<K: KeyValueStore + ?Sized> ForgeResourceResolver<K> {
    /// Create a new Forge resource resolver (ref-only, no git object serving).
    pub fn new(kv: Arc<K>) -> Self {
        Self {
            kv,
            #[cfg(feature = "git-bridge")]
            git_exporter: None,
        }
    }

    /// Create a resolver with git object export capability.
    #[cfg(feature = "git-bridge")]
    pub fn with_git_exporter(kv: Arc<K>, exporter: Arc<dyn GitObjectExporter>) -> Self {
        Self {
            kv,
            git_exporter: Some(exporter),
        }
    }

    /// Derive the repo_id hex string from a FederatedId.
    ///
    /// The `local_id` bytes are the repo identity hash (same as `RepoId.0`).
    fn repo_id_hex(fed_id: &FederatedId) -> String {
        hex::encode(fed_id.local_id)
    }

    /// Load federation settings for a resource from KV.
    async fn load_settings(&self, fed_id: &FederatedId) -> Option<FederationSettings> {
        let key = format!("{}{}", KV_PREFIX_FEDERATION_SETTINGS, fed_id);
        let read_req = aspen_core::ReadRequest::new(key);

        match self.kv.read(read_req).await {
            Ok(result) => result.kv.and_then(|kv| serde_json::from_str::<FederationSettings>(&kv.value).ok()),
            Err(_) => None,
        }
    }

    /// Scan all ref heads for a repo from KV.
    ///
    /// Returns `(ref_name, blake3_hash)` pairs. The ref value in KV is a
    /// hex-encoded BLAKE3 hash (stored by the git bridge as the commit's
    /// BLAKE3 hash, not SHA1).
    async fn scan_ref_heads(&self, refs_prefix: &str) -> Result<Vec<(String, [u8; 32])>, FederationResourceError> {
        let scan_req = ScanRequest {
            prefix: refs_prefix.to_string(),
            limit_results: Some(MAX_REFS_PER_QUERY),
            continuation_token: None,
        };

        let full_prefix = format!("{}:", refs_prefix);
        let mut heads = Vec::new();

        match self.kv.scan(scan_req).await {
            Ok(result) => {
                for entry in result.entries {
                    let ref_name = entry.key.strip_prefix(&full_prefix).unwrap_or(&entry.key);
                    if let Ok(hash_bytes) = hex::decode(entry.value.trim())
                        && hash_bytes.len() == 32
                    {
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(&hash_bytes);
                        heads.push((ref_name.to_string(), hash));
                    }
                }
            }
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {}
            Err(e) => {
                return Err(FederationResourceError::Internal {
                    message: format!("scan refs failed: {e}"),
                });
            }
        }

        Ok(heads)
    }

    /// Export git objects reachable from ref heads using the git exporter.
    ///
    /// Walks the commit DAG from each ref head, skipping objects the remote
    /// already has (by BLAKE3 hash). Returns `SyncObject` entries with raw
    /// git content and BLAKE3 hashes.
    #[cfg(feature = "git-bridge")]
    async fn export_git_objects(
        &self,
        exporter: &Arc<dyn GitObjectExporter>,
        repo_id: &RepoId,
        ref_heads: &[(String, [u8; 32])],
        have_set: &HashSet<[u8; 32]>,
        limit: usize,
    ) -> Vec<SyncObject> {
        let mut objects = Vec::new();
        let mut all_known: HashSet<[u8; 32]> = have_set.clone();

        for (_ref_name, head_hash) in ref_heads {
            if objects.len() >= limit {
                break;
            }

            let commit_blake3 = blake3::Hash::from_bytes(*head_hash);
            let remaining = limit.saturating_sub(objects.len());

            match exporter.export_dag_blake3(repo_id, commit_blake3, &all_known, remaining).await {
                Ok(result) => {
                    for obj in result.objects {
                        let object_type = obj.object_type.as_str().to_string();
                        let b3_bytes: [u8; 32] = *obj.blake3.as_bytes();
                        let data = obj.content;

                        // Use the blob-store BLAKE3 hash as the SyncObject hash.
                        // This allows the client to send it back in have_hashes
                        // for incremental sync (the DAG walk uses blob-store hashes).
                        // Content integrity is verified by the import pipeline.
                        let hash = b3_bytes;

                        all_known.insert(b3_bytes);

                        objects.push(SyncObject {
                            object_type,
                            hash,
                            data,
                            signature: None,
                            signer: None,
                        });
                    }
                }
                Err(e) => {
                    warn!(
                        repo_id = %hex::encode(repo_id.0),
                        error = %e,
                        "failed to export git objects for federation"
                    );
                }
            }
        }

        objects
    }

    /// Check if a repo exists by scanning for any keys with its prefix.
    async fn repo_exists(&self, repo_id_hex: &str) -> bool {
        // Check if repo metadata exists
        let prefix = format!("{}{}", KV_PREFIX_REPOS, repo_id_hex);
        let scan_req = ScanRequest {
            prefix,
            limit_results: Some(1),
            continuation_token: None,
        };
        matches!(self.kv.scan(scan_req).await, Ok(r) if !r.entries.is_empty())
    }
}

#[async_trait]
impl<K: ?Sized + KeyValueStore + Send + Sync + 'static> FederationResourceResolver for ForgeResourceResolver<K> {
    async fn get_resource_state(
        &self,
        fed_id: &FederatedId,
    ) -> Result<FederationResourceState, FederationResourceError> {
        let repo_hex = Self::repo_id_hex(fed_id);

        // Check federation settings
        let settings = self.load_settings(fed_id).await;
        match &settings {
            Some(s) if matches!(s.mode, FederationMode::Disabled) => {
                return Err(FederationResourceError::FederationDisabled { fed_id: fed_id.short() });
            }
            None => {
                // No settings means check if repo exists at all
                if !self.repo_exists(&repo_hex).await {
                    return Err(FederationResourceError::NotFound { fed_id: fed_id.short() });
                }
                // Repo exists but no federation settings — treat as disabled
                return Err(FederationResourceError::FederationDisabled { fed_id: fed_id.short() });
            }
            _ => {} // Public or AllowList — proceed
        }

        // Scan refs for this repo
        let refs_prefix = format!("{}{}", KV_PREFIX_REFS, repo_hex);
        let ref_list = self.scan_ref_heads(&refs_prefix).await?;
        let heads: HashMap<String, [u8; 32]> = ref_list.into_iter().collect();

        debug!(
            fed_id = %fed_id.short(),
            repo_id = %repo_hex,
            ref_count = heads.len(),
            "resolved forge resource state"
        );

        Ok(FederationResourceState {
            was_found: true,
            heads,
            metadata: Some(ResourceMetadata {
                resource_type: crate::federation::FORGE_RESOURCE_TYPE.to_string(),
                name: String::new(),
                description: None,
                delegates: Vec::new(),
                threshold_replicas: 0,
                created_at_hlc: {
                    let hlc = aspen_core::hlc::create_hlc("forge-resolver");
                    aspen_core::hlc::SerializableTimestamp::new(aspen_core::hlc::new_timestamp(&hlc))
                },
                updated_at_hlc: {
                    let hlc = aspen_core::hlc::create_hlc("forge-resolver");
                    aspen_core::hlc::SerializableTimestamp::new(aspen_core::hlc::new_timestamp(&hlc))
                },
                policy: None,
                app_metadata: Vec::new(),
            }),
        })
    }

    async fn sync_objects(
        &self,
        fed_id: &FederatedId,
        want_types: &[String],
        have_hashes: &[[u8; 32]],
        limit: u32,
    ) -> Result<Vec<SyncObject>, FederationResourceError> {
        use aspen_cluster::federation::sync::RefEntry;

        // Check federation settings first
        let settings = self.load_settings(fed_id).await;
        match &settings {
            Some(s) if matches!(s.mode, FederationMode::Disabled) => {
                return Err(FederationResourceError::FederationDisabled { fed_id: fed_id.short() });
            }
            None => {
                return Err(FederationResourceError::FederationDisabled { fed_id: fed_id.short() });
            }
            _ => {}
        }

        let wants_refs = want_types.iter().any(|t| t == "refs");
        let wants_git_objects = want_types.iter().any(|t| t == "commit" || t == "tree" || t == "blob");

        if !wants_refs && !wants_git_objects {
            return Ok(Vec::new());
        }

        let repo_hex = Self::repo_id_hex(fed_id);
        let refs_prefix = format!("{}{}", KV_PREFIX_REFS, repo_hex);
        let limit = limit.min(MAX_GIT_OBJECTS_PER_SYNC);
        let have_set: HashSet<[u8; 32]> = have_hashes.iter().copied().collect();
        let mut objects = Vec::new();

        // Phase 1: Collect refs (always needed — git objects are walked from ref heads)
        let ref_heads = self.scan_ref_heads(&refs_prefix).await?;

        // Phase 2: Return ref entries if requested
        if wants_refs {
            for (ref_name, head_hash) in &ref_heads {
                if objects.len() >= limit as usize {
                    break;
                }

                let ref_entry = RefEntry {
                    ref_name: ref_name.clone(),
                    head_hash: *head_hash,
                };
                let data = postcard::to_allocvec(&ref_entry).unwrap_or_default();
                let hash = blake3::hash(&data).into();

                if have_set.contains(&hash) {
                    continue;
                }

                objects.push(SyncObject {
                    object_type: "ref".to_string(),
                    hash,
                    data,
                    signature: None,
                    signer: None,
                });
            }
        }

        // Phase 3: Export git objects if requested and exporter available
        #[cfg(feature = "git-bridge")]
        if wants_git_objects {
            if let Some(ref exporter) = self.git_exporter {
                let repo_id = RepoId(fed_id.local_id);
                let remaining = (limit as usize).saturating_sub(objects.len());

                if remaining > 0 {
                    let git_objects =
                        self.export_git_objects(exporter, &repo_id, &ref_heads, &have_set, remaining).await;
                    objects.extend(git_objects);
                }
            } else {
                debug!(
                    fed_id = %fed_id.short(),
                    "sync_objects: git objects requested but no exporter configured"
                );
            }
        }

        #[cfg(not(feature = "git-bridge"))]
        if wants_git_objects {
            debug!(
                fed_id = %fed_id.short(),
                "sync_objects: git objects requested but git-bridge feature not enabled"
            );
        }

        debug!(
            fed_id = %fed_id.short(),
            total = objects.len(),
            refs = wants_refs,
            git = wants_git_objects,
            "sync_objects: returning objects"
        );

        Ok(objects)
    }

    async fn resource_exists(&self, fed_id: &FederatedId) -> bool {
        let repo_hex = Self::repo_id_hex(fed_id);
        self.repo_exists(&repo_hex).await
    }

    async fn list_resources(&self, limit: u32) -> Vec<(FederatedId, String)> {
        use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;

        let scan_result = self
            .kv
            .scan(aspen_core::ScanRequest {
                prefix: KV_PREFIX_FEDERATION_SETTINGS.to_string(),
                limit_results: Some(limit),
                continuation_token: None,
            })
            .await;

        let entries = match scan_result {
            Ok(resp) => resp.entries,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::with_capacity(entries.len());
        for entry in entries {
            let settings_json = &entry.value;
            if let Ok(settings) = serde_json::from_str::<FederationSettings>(settings_json) {
                if matches!(settings.mode, FederationMode::Disabled) {
                    continue;
                }
                // Parse FederatedId from the key suffix (after the prefix)
                let fed_id_str = entry.key.strip_prefix(KV_PREFIX_FEDERATION_SETTINGS).unwrap_or(&entry.key);
                if let Ok(fed_id) = fed_id_str.parse::<FederatedId>() {
                    let resource_type = settings.resource_type.unwrap_or_else(|| "unknown".to_string());
                    results.push((fed_id, resource_type));
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use aspen_core::WriteCommand;
    use aspen_core::WriteRequest;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    fn test_fed_id() -> FederatedId {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        FederatedId::new(secret.public(), [0xab; 32])
    }

    #[tokio::test]
    async fn test_forge_resolver_not_found() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let resolver = ForgeResourceResolver::new(kv);

        let fed_id = test_fed_id();
        let result = resolver.get_resource_state(&fed_id).await;

        assert!(
            matches!(result, Err(FederationResourceError::NotFound { .. })),
            "expected NotFound, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_forge_resolver_federation_disabled() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let fed_id = test_fed_id();
        let repo_hex = ForgeResourceResolver::<DeterministicKeyValueStore>::repo_id_hex(&fed_id);

        // Create repo metadata so the repo "exists"
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}", KV_PREFIX_REPOS, repo_hex),
                value: "{}".to_string(),
            },
        })
        .await
        .unwrap();

        // No federation settings → disabled
        let resolver = ForgeResourceResolver::new(kv);
        let result = resolver.get_resource_state(&fed_id).await;

        assert!(
            matches!(result, Err(FederationResourceError::FederationDisabled { .. })),
            "expected FederationDisabled, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_forge_resolver_returns_refs() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let fed_id = test_fed_id();
        let repo_hex = ForgeResourceResolver::<DeterministicKeyValueStore>::repo_id_hex(&fed_id);

        // Create repo
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}", KV_PREFIX_REPOS, repo_hex),
                value: "{}".to_string(),
            },
        })
        .await
        .unwrap();

        // Add a ref
        let commit_hash = [0x42u8; 32];
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}:heads/main", KV_PREFIX_REFS, repo_hex),
                value: hex::encode(commit_hash),
            },
        })
        .await
        .unwrap();

        // Enable federation (Public mode)
        let settings = FederationSettings::public();
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}", KV_PREFIX_FEDERATION_SETTINGS, fed_id),
                value: serde_json::to_string(&settings).unwrap(),
            },
        })
        .await
        .unwrap();

        let resolver = ForgeResourceResolver::new(kv);
        let state = resolver.get_resource_state(&fed_id).await.expect("should resolve");

        assert!(state.was_found);
        assert_eq!(state.heads.len(), 1);
        assert_eq!(state.heads.get("heads/main"), Some(&commit_hash));
        assert_eq!(state.metadata.as_ref().map(|m| m.resource_type.as_str()), Some("forge:repo"));
    }

    #[tokio::test]
    async fn test_forge_resolver_resource_exists() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let fed_id = test_fed_id();
        let repo_hex = ForgeResourceResolver::<DeterministicKeyValueStore>::repo_id_hex(&fed_id);

        let resolver = ForgeResourceResolver::new(Arc::clone(&kv));

        // Doesn't exist yet
        assert!(!resolver.resource_exists(&fed_id).await);

        // Create repo
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}", KV_PREFIX_REPOS, repo_hex),
                value: "{}".to_string(),
            },
        })
        .await
        .unwrap();

        // Now exists
        assert!(resolver.resource_exists(&fed_id).await);
    }

    // ====================================================================
    // sync_objects tests
    // ====================================================================

    /// Helper: set up a federated repo with refs and return (kv, fed_id).
    async fn setup_federated_repo_with_refs(
        ref_entries: &[(&str, [u8; 32])],
    ) -> (Arc<DeterministicKeyValueStore>, FederatedId) {
        let kv = DeterministicKeyValueStore::new();
        let fed_id = test_fed_id();
        let repo_hex = ForgeResourceResolver::<DeterministicKeyValueStore>::repo_id_hex(&fed_id);

        // Create repo
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}", KV_PREFIX_REPOS, repo_hex),
                value: "{}".to_string(),
            },
        })
        .await
        .unwrap();

        // Enable federation
        let settings = FederationSettings::public();
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}", KV_PREFIX_FEDERATION_SETTINGS, fed_id),
                value: serde_json::to_string(&settings).unwrap(),
            },
        })
        .await
        .unwrap();

        // Add refs
        for (ref_name, hash) in ref_entries {
            kv.write(WriteRequest {
                command: WriteCommand::Set {
                    key: format!("{}{}:{}", KV_PREFIX_REFS, repo_hex, ref_name),
                    value: hex::encode(hash),
                },
            })
            .await
            .unwrap();
        }

        (kv, fed_id)
    }

    #[tokio::test]
    async fn test_sync_objects_returns_refs() {
        let refs = [("heads/main", [0x11u8; 32]), ("heads/dev", [0x22u8; 32])];
        let (kv, fed_id) = setup_federated_repo_with_refs(&refs).await;

        let resolver = ForgeResourceResolver::new(kv);
        let objects = resolver
            .sync_objects(&fed_id, &["refs".to_string()], &[], 100)
            .await
            .expect("should return objects");

        assert_eq!(objects.len(), 2);
        for obj in &objects {
            assert_eq!(obj.object_type, "ref");
            // Verify BLAKE3 hash matches data
            let expected_hash: [u8; 32] = blake3::hash(&obj.data).into();
            assert_eq!(obj.hash, expected_hash);
            // Verify data deserializes to RefEntry
            let entry: aspen_cluster::federation::sync::RefEntry =
                postcard::from_bytes(&obj.data).expect("should deserialize");
            assert!(entry.ref_name == "heads/main" || entry.ref_name == "heads/dev");
        }
    }

    #[tokio::test]
    async fn test_sync_objects_filters_by_have_hashes() {
        let refs = [("heads/main", [0x11u8; 32]), ("heads/dev", [0x22u8; 32])];
        let (kv, fed_id) = setup_federated_repo_with_refs(&refs).await;

        let resolver = ForgeResourceResolver::new(Arc::clone(&kv));

        // First fetch to get all objects and their hashes
        let all_objects = resolver.sync_objects(&fed_id, &["refs".to_string()], &[], 100).await.unwrap();
        assert_eq!(all_objects.len(), 2);

        // Now fetch again with one hash in have_hashes
        let have = vec![all_objects[0].hash];
        let filtered = resolver.sync_objects(&fed_id, &["refs".to_string()], &have, 100).await.unwrap();

        assert_eq!(filtered.len(), 1);
        assert_ne!(filtered[0].hash, all_objects[0].hash);
    }

    #[tokio::test]
    async fn test_sync_objects_respects_limit() {
        let refs = [
            ("heads/a", [0x01u8; 32]),
            ("heads/b", [0x02u8; 32]),
            ("heads/c", [0x03u8; 32]),
        ];
        let (kv, fed_id) = setup_federated_repo_with_refs(&refs).await;

        let resolver = ForgeResourceResolver::new(kv);
        let objects = resolver.sync_objects(&fed_id, &["refs".to_string()], &[], 2).await.unwrap();

        assert_eq!(objects.len(), 2);
    }

    #[tokio::test]
    async fn test_sync_objects_empty_repo() {
        let (kv, fed_id) = setup_federated_repo_with_refs(&[]).await;

        let resolver = ForgeResourceResolver::new(kv);
        let objects = resolver.sync_objects(&fed_id, &["refs".to_string()], &[], 100).await.unwrap();

        assert!(objects.is_empty());
    }

    #[tokio::test]
    async fn test_sync_objects_ignores_non_ref_types() {
        let refs = [("heads/main", [0x11u8; 32])];
        let (kv, fed_id) = setup_federated_repo_with_refs(&refs).await;

        let resolver = ForgeResourceResolver::new(kv);
        let objects = resolver.sync_objects(&fed_id, &["blobs".to_string()], &[], 100).await.unwrap();

        assert!(objects.is_empty());
    }

    #[tokio::test]
    async fn test_sync_objects_federation_disabled() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let fed_id = test_fed_id();

        // No federation settings
        let resolver = ForgeResourceResolver::new(kv);
        let result = resolver.sync_objects(&fed_id, &["refs".to_string()], &[], 100).await;

        assert!(matches!(result, Err(FederationResourceError::FederationDisabled { .. })));
    }

    #[tokio::test]
    async fn test_forge_resolver_explicit_disabled() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let fed_id = test_fed_id();
        let repo_hex = ForgeResourceResolver::<DeterministicKeyValueStore>::repo_id_hex(&fed_id);

        // Create repo
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}", KV_PREFIX_REPOS, repo_hex),
                value: "{}".to_string(),
            },
        })
        .await
        .unwrap();

        // Explicitly disable federation
        let settings = FederationSettings::disabled();
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("{}{}", KV_PREFIX_FEDERATION_SETTINGS, fed_id),
                value: serde_json::to_string(&settings).unwrap(),
            },
        })
        .await
        .unwrap();

        let resolver = ForgeResourceResolver::new(kv);
        let result = resolver.get_resource_state(&fed_id).await;

        assert!(
            matches!(result, Err(FederationResourceError::FederationDisabled { .. })),
            "expected FederationDisabled, got: {result:?}"
        );
    }
}

/// Integration tests for git object serving via federation sync.
///
/// These tests exercise the full flow: create a repo with git objects,
/// federate it, then sync via ForgeResourceResolver with a GitObjectExporter.
#[cfg(all(test, feature = "git-bridge"))]
mod git_bridge_tests {
    use std::collections::HashSet;

    use aspen_blob::InMemoryBlobStore;
    use aspen_cluster::federation::FederatedId;
    use aspen_cluster::federation::FederationResourceResolver;
    use aspen_cluster::federation::FederationSettings;
    use aspen_core::WriteCommand;
    use aspen_core::WriteRequest;
    use aspen_core::hlc::create_hlc;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;
    use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;
    use crate::constants::KV_PREFIX_REFS;
    use crate::constants::KV_PREFIX_REPOS;
    use crate::git::bridge::GitExporter;
    use crate::git::bridge::GitImporter;
    use crate::git::bridge::HashMappingStore;
    use crate::git::bridge::Sha1Hash;
    use crate::identity::RepoId;
    use crate::refs::RefStore;

    /// Test harness: two forge "clusters" with shared types but separate storage.
    struct FederationTestHarness {
        /// Source cluster KV.
        source_kv: Arc<dyn aspen_core::KeyValueStore>,
        /// Source cluster blobs.
        source_blobs: Arc<InMemoryBlobStore>,
        /// Source importer (for creating objects).
        source_importer: GitImporter<dyn aspen_core::KeyValueStore, InMemoryBlobStore>,
        /// Source refs.
        source_refs: Arc<RefStore<dyn aspen_core::KeyValueStore>>,
        /// Source exporter (used by the resolver).
        source_exporter: Arc<GitExporter<dyn aspen_core::KeyValueStore, InMemoryBlobStore>>,
        /// Repo ID on source.
        repo_id: RepoId,
        /// Federated ID.
        fed_id: FederatedId,
    }

    impl FederationTestHarness {
        fn new() -> Self {
            let source_kv: Arc<dyn aspen_core::KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
            let source_blobs = Arc::new(InMemoryBlobStore::new());
            let mapping = Arc::new(HashMappingStore::new(Arc::clone(&source_kv)));
            let refs = Arc::new(RefStore::new(Arc::clone(&source_kv)));
            let secret_key = iroh::SecretKey::generate(&mut rand::rng());

            let importer = GitImporter::new(
                Arc::clone(&mapping),
                Arc::clone(&source_blobs),
                Arc::clone(&refs),
                secret_key.clone(),
                create_hlc("test-source"),
            );
            let exporter = Arc::new(GitExporter::new(
                Arc::clone(&mapping),
                Arc::clone(&source_blobs),
                Arc::clone(&refs),
                secret_key.clone(),
                create_hlc("test-source-export"),
            ));

            let repo_id = RepoId::from_hash(blake3::hash(b"federation-test-repo"));
            let fed_id = FederatedId::new(secret_key.public(), repo_id.0);

            Self {
                source_kv,
                source_blobs: source_blobs,
                source_importer: importer,
                source_refs: refs,
                source_exporter: exporter,
                repo_id,
                fed_id,
            }
        }

        /// Set up the repo as federated (create repo metadata + federation settings).
        async fn federate_repo(&self) {
            // Create repo metadata
            self.source_kv
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: format!("{}{}", KV_PREFIX_REPOS, hex::encode(self.repo_id.0)),
                        value: "{}".to_string(),
                    },
                })
                .await
                .unwrap();

            // Enable federation
            let settings = FederationSettings::public();
            self.source_kv
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: format!("{}{}", KV_PREFIX_FEDERATION_SETTINGS, self.fed_id),
                        value: serde_json::to_string(&settings).unwrap(),
                    },
                })
                .await
                .unwrap();
        }

        /// Import a git blob and return its BLAKE3 + SHA1 hashes.
        async fn import_blob(&self, content: &[u8]) -> (blake3::Hash, Sha1Hash) {
            let git_bytes = make_git_blob(content);
            let sha1 = compute_sha1(&git_bytes);
            let blake3 = self.source_importer.import_object(&self.repo_id, &git_bytes).await.unwrap();
            (blake3, sha1)
        }

        /// Import a git tree with one blob entry.
        async fn import_tree(&self, blob_sha1: &Sha1Hash, name: &str) -> (blake3::Hash, Sha1Hash) {
            let entry = make_tree_entry("100644", name, blob_sha1);
            let git_bytes = make_git_tree(&[entry]);
            let sha1 = compute_sha1(&git_bytes);
            let blake3 = self.source_importer.import_object(&self.repo_id, &git_bytes).await.unwrap();
            (blake3, sha1)
        }

        /// Import a git commit pointing to a tree.
        async fn import_commit(
            &self,
            tree_sha1: &Sha1Hash,
            parents: &[&Sha1Hash],
            message: &str,
        ) -> (blake3::Hash, Sha1Hash) {
            let git_bytes = make_git_commit(tree_sha1, parents, message);
            let sha1 = compute_sha1(&git_bytes);
            let blake3 = self.source_importer.import_object(&self.repo_id, &git_bytes).await.unwrap();
            (blake3, sha1)
        }

        /// Set a ref on the source repo.
        async fn set_ref(&self, ref_name: &str, blake3: blake3::Hash) {
            self.source_refs.set(&self.repo_id, ref_name, blake3).await.unwrap();

            // Also write the hex-encoded hash to KV (federation reads this)
            let key = format!("{}{}:{}", KV_PREFIX_REFS, hex::encode(self.repo_id.0), ref_name);
            self.source_kv
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key,
                        value: hex::encode(blake3.as_bytes()),
                    },
                })
                .await
                .unwrap();
        }

        /// Build a resolver with the git exporter attached.
        fn resolver(&self) -> ForgeResourceResolver<dyn aspen_core::KeyValueStore> {
            ForgeResourceResolver::with_git_exporter(Arc::clone(&self.source_kv), self.source_exporter.clone())
        }
    }

    // ====================================================================
    // Git object helpers (same as in git/bridge/tests.rs)
    // ====================================================================

    fn make_git_blob(content: &[u8]) -> Vec<u8> {
        let header = format!("blob {}\0", content.len());
        let mut bytes = Vec::with_capacity(header.len() + content.len());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(content);
        bytes
    }

    fn compute_sha1(git_bytes: &[u8]) -> Sha1Hash {
        use sha1::Digest;
        let hash = sha1::Sha1::digest(git_bytes);
        Sha1Hash::from_bytes(hash.into())
    }

    fn make_tree_entry(mode: &str, name: &str, sha1: &Sha1Hash) -> Vec<u8> {
        let mut entry = Vec::new();
        entry.extend_from_slice(mode.as_bytes());
        entry.push(b' ');
        entry.extend_from_slice(name.as_bytes());
        entry.push(0);
        entry.extend_from_slice(sha1.as_slice());
        entry
    }

    fn make_git_tree(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut content = Vec::new();
        for entry in entries {
            content.extend_from_slice(entry);
        }
        let header = format!("tree {}\0", content.len());
        let mut bytes = Vec::with_capacity(header.len() + content.len());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(&content);
        bytes
    }

    fn make_git_commit(tree_sha1: &Sha1Hash, parents: &[&Sha1Hash], message: &str) -> Vec<u8> {
        let mut content = String::new();
        content.push_str(&format!("tree {}\n", tree_sha1));
        for parent in parents {
            content.push_str(&format!("parent {}\n", parent));
        }
        content.push_str("author Test User <test@example.com> 1700000000 +0000\n");
        content.push_str("committer Test User <test@example.com> 1700000000 +0000\n");
        content.push('\n');
        content.push_str(message);
        if !message.ends_with('\n') {
            content.push('\n');
        }
        let header = format!("commit {}\0", content.len());
        let mut bytes = Vec::with_capacity(header.len() + content.len());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(content.as_bytes());
        bytes
    }

    // ====================================================================
    // Test 6.1: Round-trip — create objects, federate, sync via resolver
    // ====================================================================

    #[tokio::test]
    async fn test_federation_sync_git_objects_roundtrip() {
        let h = FederationTestHarness::new();
        h.federate_repo().await;

        // Create: blob → tree → commit → ref
        let (blob_b3, blob_sha1) = h.import_blob(b"hello federation\n").await;
        let (_tree_b3, tree_sha1) = h.import_tree(&blob_sha1, "README.md").await;
        let (commit_b3, _commit_sha1) = h.import_commit(&tree_sha1, &[], "initial commit").await;
        h.set_ref("heads/main", commit_b3).await;

        // Sync via resolver — request git objects
        let resolver = h.resolver();
        let objects = resolver
            .sync_objects(&h.fed_id, &["commit".to_string(), "tree".to_string(), "blob".to_string()], &[], 1000)
            .await
            .expect("sync should succeed");

        // Should get 3 objects: blob, tree, commit (in dependency order)
        assert_eq!(objects.len(), 3, "expected 3 git objects (blob, tree, commit)");

        // Verify types
        let types: Vec<&str> = objects.iter().map(|o| o.object_type.as_str()).collect();
        assert!(types.contains(&"blob"), "should contain a blob");
        assert!(types.contains(&"tree"), "should contain a tree");
        assert!(types.contains(&"commit"), "should contain a commit");

        // Verify hashes are non-zero (blob-store BLAKE3 hashes, not content hashes)
        for obj in &objects {
            assert_ne!(obj.hash, [0u8; 32], "hash should be non-zero for {}", obj.object_type);
            assert!(!obj.data.is_empty(), "data should be non-empty for {}", obj.object_type);
        }
    }

    // ====================================================================
    // Test 6.1b: Refs + git objects in a single sync request
    // ====================================================================

    #[tokio::test]
    async fn test_federation_sync_refs_and_git_objects() {
        let h = FederationTestHarness::new();
        h.federate_repo().await;

        let (blob_b3, blob_sha1) = h.import_blob(b"content\n").await;
        let (_tree_b3, tree_sha1) = h.import_tree(&blob_sha1, "file.txt").await;
        let (commit_b3, _) = h.import_commit(&tree_sha1, &[], "add file").await;
        h.set_ref("heads/main", commit_b3).await;

        let resolver = h.resolver();
        let objects = resolver
            .sync_objects(
                &h.fed_id,
                &[
                    "refs".to_string(),
                    "commit".to_string(),
                    "tree".to_string(),
                    "blob".to_string(),
                ],
                &[],
                1000,
            )
            .await
            .unwrap();

        // 1 ref + 3 git objects = 4
        assert_eq!(objects.len(), 4, "expected 1 ref + 3 git objects");

        let ref_count = objects.iter().filter(|o| o.object_type == "ref").count();
        let git_count = objects.iter().filter(|o| o.object_type != "ref").count();
        assert_eq!(ref_count, 1);
        assert_eq!(git_count, 3);
    }

    // ====================================================================
    // Test 6.2: Incremental sync — have_hashes excludes known objects
    // ====================================================================

    #[tokio::test]
    async fn test_federation_sync_incremental_with_have_hashes() {
        let h = FederationTestHarness::new();
        h.federate_repo().await;

        // First commit: blob1 → tree1 → commit1
        let (blob1_b3, blob1_sha1) = h.import_blob(b"first\n").await;
        let (_tree1_b3, tree1_sha1) = h.import_tree(&blob1_sha1, "first.txt").await;
        let (commit1_b3, commit1_sha1) = h.import_commit(&tree1_sha1, &[], "first commit").await;
        h.set_ref("heads/main", commit1_b3).await;

        // Sync everything first time
        let resolver = h.resolver();
        let first_sync = resolver
            .sync_objects(&h.fed_id, &["commit".to_string(), "tree".to_string(), "blob".to_string()], &[], 1000)
            .await
            .unwrap();
        assert_eq!(first_sync.len(), 3);

        // Collect hashes from first sync as "already have"
        let have_hashes: Vec<[u8; 32]> = first_sync.iter().map(|o| o.hash).collect();

        // Second commit: blob2 → tree2 → commit2 (parent: commit1)
        let (blob2_b3, blob2_sha1) = h.import_blob(b"second\n").await;
        let (_tree2_b3, tree2_sha1) = h.import_tree(&blob2_sha1, "second.txt").await;
        let (commit2_b3, _) = h.import_commit(&tree2_sha1, &[&commit1_sha1], "second commit").await;
        h.set_ref("heads/main", commit2_b3).await;

        // Incremental sync: provide have_hashes from first sync
        let incremental = resolver
            .sync_objects(
                &h.fed_id,
                &["commit".to_string(), "tree".to_string(), "blob".to_string()],
                &have_hashes,
                1000,
            )
            .await
            .unwrap();

        // Should only get the 3 new objects (blob2, tree2, commit2)
        // The first commit's objects should be skipped via have_hashes
        assert_eq!(incremental.len(), 3, "incremental sync should return only new objects, got {}", incremental.len());

        // None of the returned hashes should be in our have set
        for obj in &incremental {
            assert!(!have_hashes.contains(&obj.hash), "incremental sync should not return objects we already have");
        }
    }

    // ====================================================================
    // Test 6.2b: No exporter returns empty for git object types
    // ====================================================================

    #[tokio::test]
    async fn test_federation_sync_no_exporter_returns_refs_only() {
        let h = FederationTestHarness::new();
        h.federate_repo().await;

        let (blob_b3, blob_sha1) = h.import_blob(b"data\n").await;
        let (_tree_b3, tree_sha1) = h.import_tree(&blob_sha1, "f.txt").await;
        let (commit_b3, _) = h.import_commit(&tree_sha1, &[], "commit").await;
        h.set_ref("heads/main", commit_b3).await;

        // Resolver without exporter
        let resolver = ForgeResourceResolver::new(Arc::clone(&h.source_kv));
        let objects = resolver
            .sync_objects(
                &h.fed_id,
                &[
                    "refs".to_string(),
                    "commit".to_string(),
                    "tree".to_string(),
                    "blob".to_string(),
                ],
                &[],
                1000,
            )
            .await
            .unwrap();

        // Should only get ref entries (no git objects without exporter)
        assert_eq!(objects.len(), 1, "without exporter, should only get refs");
        assert_eq!(objects[0].object_type, "ref");
    }
}
