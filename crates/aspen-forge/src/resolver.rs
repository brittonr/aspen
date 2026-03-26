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

                        // BLAKE3 hash the raw content for the SyncObject hash
                        // (this is the content hash the client will verify)
                        let data = obj.content;
                        let hash: [u8; 32] = blake3::hash(&data).into();

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
impl<K: KeyValueStore + Send + Sync + 'static> FederationResourceResolver for ForgeResourceResolver<K> {
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
