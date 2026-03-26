//! Federation resource resolver for Forge repositories.
//!
//! Translates federation sync requests into reads against the Forge KV layout.
//! Maps `FederatedId` → repo refs via `forge:refs:{repo_id}:{ref_name}` keys
//! and checks federation settings via `forge:federation:settings:{fed_id}`.

use std::collections::HashMap;
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

use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;
use crate::constants::KV_PREFIX_REFS;
use crate::constants::KV_PREFIX_REPOS;

/// Maximum refs returned per resource state query.
const MAX_REFS_PER_QUERY: u32 = 1000;

/// Forge-specific federation resource resolver.
///
/// Reads ref heads from the Forge KV layout (`forge:refs:{repo_id}:{ref_name}`)
/// and checks federation settings from `forge:federation:settings:{fed_id}`.
///
/// The `FederatedId.local_id` is treated as a 32-byte repo identity hash.
/// This maps directly to `RepoId` which is also a `[u8; 32]`.
pub struct ForgeResourceResolver<K: KeyValueStore + ?Sized> {
    kv: Arc<K>,
}

impl<K: KeyValueStore + ?Sized> ForgeResourceResolver<K> {
    /// Create a new Forge resource resolver.
    pub fn new(kv: Arc<K>) -> Self {
        Self { kv }
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

        // Scan refs for this repo: forge:refs:{repo_id}:{ref_name} → hash_hex
        let refs_prefix = format!("{}{}", KV_PREFIX_REFS, repo_hex);
        let scan_req = ScanRequest {
            prefix: refs_prefix.clone(),
            limit_results: Some(MAX_REFS_PER_QUERY),
            continuation_token: None,
        };

        let mut heads = HashMap::new();
        match self.kv.scan(scan_req).await {
            Ok(result) => {
                for entry in result.entries {
                    // Key: forge:refs:{repo_id}:{ref_name}
                    // Strip the prefix including the colon after repo_id
                    let full_prefix = format!("{}:", refs_prefix);
                    let ref_name = entry.key.strip_prefix(&full_prefix).unwrap_or(&entry.key);

                    // Value is hex-encoded hash
                    if let Ok(hash_bytes) = hex::decode(entry.value.trim())
                        && hash_bytes.len() == 32
                    {
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(&hash_bytes);
                        heads.insert(ref_name.to_string(), hash);
                    }
                }
            }
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                // No refs yet
            }
            Err(e) => {
                return Err(FederationResourceError::Internal {
                    message: format!("scan refs failed: {e}"),
                });
            }
        }

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

        // Only handle "refs" object type — git pack data goes via iroh-blobs
        if !want_types.iter().any(|t| t == "refs") {
            return Ok(Vec::new());
        }

        let repo_hex = Self::repo_id_hex(fed_id);
        let refs_prefix = format!("{}{}", KV_PREFIX_REFS, repo_hex);
        let limit = limit.min(MAX_REFS_PER_QUERY);

        let have_set: std::collections::HashSet<[u8; 32]> = have_hashes.iter().copied().collect();

        let scan_req = ScanRequest {
            prefix: refs_prefix.clone(),
            limit_results: Some(limit),
            continuation_token: None,
        };

        let mut objects = Vec::new();
        match self.kv.scan(scan_req).await {
            Ok(result) => {
                let full_prefix = format!("{}:", refs_prefix);
                for entry in result.entries {
                    if objects.len() >= limit as usize {
                        break;
                    }

                    let ref_name = entry.key.strip_prefix(&full_prefix).unwrap_or(&entry.key);

                    // Parse the hex-encoded hash
                    let head_hash = match hex::decode(entry.value.trim()) {
                        Ok(bytes) if bytes.len() == 32 => {
                            let mut h = [0u8; 32];
                            h.copy_from_slice(&bytes);
                            h
                        }
                        _ => continue,
                    };

                    let ref_entry = RefEntry {
                        ref_name: ref_name.to_string(),
                        head_hash,
                    };
                    let data = postcard::to_allocvec(&ref_entry).unwrap_or_default();
                    let hash = blake3::hash(&data).into();

                    // Skip if requester already has this object
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
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {}
            Err(e) => {
                return Err(FederationResourceError::Internal {
                    message: format!("scan refs for sync_objects failed: {e}"),
                });
            }
        }

        debug!(
            fed_id = %fed_id.short(),
            ref_count = objects.len(),
            "sync_objects: returning ref entries"
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
