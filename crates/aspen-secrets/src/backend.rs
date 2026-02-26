//! Storage backend trait for secrets engines.
//!
//! Provides an abstraction layer between secrets engines and the underlying
//! storage (Aspen's distributed KV store).

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::Result;
use crate::error::SecretsError;

/// Storage backend for secrets engines.
///
/// This trait abstracts the underlying storage mechanism, allowing secrets
/// engines to work with both in-memory (testing) and distributed (production)
/// storage backends.
#[async_trait]
pub trait SecretsBackend: Send + Sync {
    /// Store a value at the given path.
    ///
    /// The path is relative to the engine's mount point.
    /// Values are automatically encrypted at rest.
    async fn put(&self, path: &str, value: &[u8]) -> Result<()>;

    /// Get a value at the given path.
    ///
    /// Returns `None` if the path doesn't exist.
    async fn get(&self, path: &str) -> Result<Option<Vec<u8>>>;

    /// Delete a value at the given path.
    ///
    /// Returns `true` if the value existed and was deleted.
    async fn delete(&self, path: &str) -> Result<bool>;

    /// List all keys under a prefix.
    ///
    /// Returns relative paths from the prefix.
    /// Keys ending in `/` represent "directories" (prefixes with children).
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;

    /// Check if a path exists.
    async fn exists(&self, path: &str) -> Result<bool> {
        Ok(self.get(path).await?.is_some())
    }

    /// Atomically put a value only if it doesn't exist (or matches expected).
    ///
    /// Used for CAS (check-and-set) operations.
    /// Returns `true` if the operation succeeded, `false` if CAS failed.
    async fn put_cas(&self, path: &str, value: &[u8], expected_version: Option<u64>) -> Result<bool>;

    /// Get a value with its version number.
    ///
    /// Returns `None` if the path doesn't exist.
    async fn get_with_version(&self, path: &str) -> Result<Option<(Vec<u8>, u64)>>;
}

/// Wrapper around Aspen's KeyValueStore for secrets storage.
///
/// All paths are prefixed with the secrets system prefix to keep
/// secrets isolated from regular KV data.
pub struct AspenSecretsBackend {
    /// Underlying KV store.
    kv: Arc<dyn aspen_core::KeyValueStore>,
    /// Prefix for all secrets storage paths.
    prefix: String,
}

impl AspenSecretsBackend {
    /// Create a new secrets backend wrapping the given KV store.
    pub fn new(kv: Arc<dyn aspen_core::KeyValueStore>, mount: &str) -> Self {
        Self {
            kv,
            prefix: format!("{}{}/", crate::constants::SECRETS_SYSTEM_PREFIX, mount),
        }
    }

    /// Get the full path for a relative secrets path.
    fn full_path(&self, path: &str) -> String {
        format!("{}{}", self.prefix, path)
    }
}

#[async_trait]
impl SecretsBackend for AspenSecretsBackend {
    async fn put(&self, path: &str, value: &[u8]) -> Result<()> {
        use aspen_core::WriteCommand;
        use aspen_core::WriteRequest;
        use base64::Engine;

        let full_path = self.full_path(path);
        let encoded = base64::engine::general_purpose::STANDARD.encode(value);

        let request = WriteRequest {
            command: WriteCommand::Set {
                key: full_path,
                value: encoded,
            },
        };

        self.kv.write(request).await.map_err(|e| SecretsError::KvStore { reason: e.to_string() })?;

        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Option<Vec<u8>>> {
        use aspen_core::KeyValueStoreError;
        use aspen_core::ReadRequest;
        use base64::Engine;

        let full_path = self.full_path(path);
        let request = ReadRequest::new(&full_path);

        match self.kv.read(request).await {
            Ok(result) => {
                if let Some(entry) = result.kv {
                    let decoded = base64::engine::general_purpose::STANDARD.decode(&entry.value).map_err(|e| {
                        SecretsError::Internal {
                            reason: format!("corrupted secrets storage: {e}"),
                        }
                    })?;
                    Ok(Some(decoded))
                } else {
                    Ok(None)
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(SecretsError::KvStore { reason: e.to_string() }),
        }
    }

    async fn delete(&self, path: &str) -> Result<bool> {
        use aspen_core::DeleteRequest;

        let full_path = self.full_path(path);
        let request = DeleteRequest { key: full_path };

        let result = self.kv.delete(request).await.map_err(|e| SecretsError::KvStore { reason: e.to_string() })?;

        Ok(result.is_deleted)
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        use std::collections::HashSet;

        use aspen_core::ScanRequest;

        let full_prefix = self.full_path(prefix);
        let request = ScanRequest {
            prefix: full_prefix.clone(),
            limit_results: None,
            continuation_token: None,
        };

        let result = self.kv.scan(request).await.map_err(|e| SecretsError::KvStore { reason: e.to_string() })?;

        // Extract unique keys/directories from results
        let prefix_len = full_prefix.len();
        let mut keys = HashSet::new();

        for entry in result.entries {
            if entry.key.len() > prefix_len {
                let relative = &entry.key[prefix_len..];
                // If there's a slash, it's a "directory", return just the first component
                if let Some(slash_pos) = relative.find('/') {
                    keys.insert(format!("{}/", &relative[..slash_pos]));
                } else {
                    keys.insert(relative.to_string());
                }
            }
        }

        let mut result: Vec<String> = keys.into_iter().collect();
        result.sort();
        Ok(result)
    }

    async fn put_cas(&self, path: &str, value: &[u8], expected_version: Option<u64>) -> Result<bool> {
        use aspen_core::WriteCommand;
        use aspen_core::WriteRequest;
        use base64::Engine;

        let full_path = self.full_path(path);
        let encoded = base64::engine::general_purpose::STANDARD.encode(value);

        let request = WriteRequest {
            command: WriteCommand::CompareAndSwap {
                key: full_path,
                expected: expected_version.map(|v| v.to_string()),
                new_value: encoded,
            },
        };

        let result = self.kv.write(request).await.map_err(|e| SecretsError::KvStore { reason: e.to_string() })?;

        Ok(result.succeeded.unwrap_or(false))
    }

    async fn get_with_version(&self, path: &str) -> Result<Option<(Vec<u8>, u64)>> {
        use aspen_core::KeyValueStoreError;
        use aspen_core::ReadRequest;
        use base64::Engine;

        let full_path = self.full_path(path);
        let request = ReadRequest::new(&full_path);

        match self.kv.read(request).await {
            Ok(result) => {
                if let Some(entry) = result.kv {
                    let decoded = base64::engine::general_purpose::STANDARD.decode(&entry.value).map_err(|e| {
                        SecretsError::Internal {
                            reason: format!("corrupted secrets storage: {e}"),
                        }
                    })?;
                    Ok(Some((decoded, entry.mod_revision)))
                } else {
                    Ok(None)
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(SecretsError::KvStore { reason: e.to_string() }),
        }
    }
}

/// In-memory secrets backend for testing.
///
/// Thread-safe and deterministic for simulation testing.
#[derive(Default)]
pub struct InMemorySecretsBackend {
    data: tokio::sync::RwLock<std::collections::HashMap<String, (Vec<u8>, u64)>>,
    version_counter: std::sync::atomic::AtomicU64,
}

impl InMemorySecretsBackend {
    /// Create a new in-memory backend.
    pub fn new() -> Self {
        Self::default()
    }

    fn next_version(&self) -> u64 {
        self.version_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl SecretsBackend for InMemorySecretsBackend {
    async fn put(&self, path: &str, value: &[u8]) -> Result<()> {
        let version = self.next_version();
        let mut data = self.data.write().await;
        data.insert(path.to_string(), (value.to_vec(), version));
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let data = self.data.read().await;
        Ok(data.get(path).map(|(v, _)| v.clone()))
    }

    async fn delete(&self, path: &str) -> Result<bool> {
        let mut data = self.data.write().await;
        Ok(data.remove(path).is_some())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        use std::collections::HashSet;

        let data = self.data.read().await;
        let mut keys = HashSet::new();

        for key in data.keys() {
            if let Some(relative) = key.strip_prefix(prefix) {
                if let Some(slash_pos) = relative.find('/') {
                    keys.insert(format!("{}/", &relative[..slash_pos]));
                } else if !relative.is_empty() {
                    keys.insert(relative.to_string());
                }
            }
        }

        let mut result: Vec<String> = keys.into_iter().collect();
        result.sort();
        Ok(result)
    }

    async fn put_cas(&self, path: &str, value: &[u8], expected_version: Option<u64>) -> Result<bool> {
        let mut data = self.data.write().await;

        match (data.get(path), expected_version) {
            (None, None) => {
                // Key doesn't exist and we expect it not to exist
                let version = self.next_version();
                data.insert(path.to_string(), (value.to_vec(), version));
                Ok(true)
            }
            (Some((_, current_version)), Some(expected)) if *current_version == expected => {
                // Key exists with expected version
                let version = self.next_version();
                data.insert(path.to_string(), (value.to_vec(), version));
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn get_with_version(&self, path: &str) -> Result<Option<(Vec<u8>, u64)>> {
        let data = self.data.read().await;
        Ok(data.get(path).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inmemory_put_get() {
        let backend = InMemorySecretsBackend::new();

        backend.put("test/key", b"value").await.unwrap();
        let result = backend.get("test/key").await.unwrap();
        assert_eq!(result, Some(b"value".to_vec()));
    }

    #[tokio::test]
    async fn test_inmemory_delete() {
        let backend = InMemorySecretsBackend::new();

        backend.put("test/key", b"value").await.unwrap();
        assert!(backend.exists("test/key").await.unwrap());

        let deleted = backend.delete("test/key").await.unwrap();
        assert!(deleted);

        assert!(!backend.exists("test/key").await.unwrap());
    }

    #[tokio::test]
    async fn test_inmemory_list() {
        let backend = InMemorySecretsBackend::new();

        backend.put("a/b/c", b"1").await.unwrap();
        backend.put("a/b/d", b"2").await.unwrap();
        backend.put("a/e", b"3").await.unwrap();
        backend.put("f", b"4").await.unwrap();

        let root = backend.list("").await.unwrap();
        assert!(root.contains(&"a/".to_string()));
        assert!(root.contains(&"f".to_string()));

        let a_list = backend.list("a/").await.unwrap();
        assert!(a_list.contains(&"b/".to_string()));
        assert!(a_list.contains(&"e".to_string()));

        let ab_list = backend.list("a/b/").await.unwrap();
        assert!(ab_list.contains(&"c".to_string()));
        assert!(ab_list.contains(&"d".to_string()));
    }

    #[tokio::test]
    async fn test_inmemory_cas() {
        let backend = InMemorySecretsBackend::new();

        // CAS on non-existent key with expected=None should succeed
        assert!(backend.put_cas("test", b"v1", None).await.unwrap());

        // Get version
        let (_, version) = backend.get_with_version("test").await.unwrap().unwrap();

        // CAS with wrong version should fail
        assert!(!backend.put_cas("test", b"v2", Some(version + 1)).await.unwrap());

        // CAS with correct version should succeed
        assert!(backend.put_cas("test", b"v2", Some(version)).await.unwrap());

        // Verify new value
        let result = backend.get("test").await.unwrap();
        assert_eq!(result, Some(b"v2".to_vec()));
    }
}
