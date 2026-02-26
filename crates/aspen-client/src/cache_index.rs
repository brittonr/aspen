//! RPC-backed Nix binary cache index.
//!
//! Implements the `CacheIndex` trait by sending RPCs to the cluster,
//! allowing ephemeral CI workers to query/update the cache without
//! direct KV store access.

#[cfg(feature = "cache-index")]
mod rpc_cache_index {
    use aspen_cache::CacheEntry;
    use aspen_cache::CacheStats;
    use aspen_cache::Result;
    use aspen_cache::error::CacheError;
    use aspen_client_api::ClientRpcRequest;
    use aspen_client_api::ClientRpcResponse;
    use async_trait::async_trait;
    use tracing::debug;
    use tracing::warn;

    use crate::AspenClient;

    /// `CacheIndex` implementation backed by Aspen RPC.
    ///
    /// Sends `CacheQuery` / `CacheStats` RPCs to the cluster, translating
    /// responses into the `aspen_cache` types. Designed for ephemeral CI
    /// workers that need cache access but have no local KV store.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use aspen_client::{AspenClient, RpcCacheIndex};
    /// use aspen_cache::CacheIndex;
    ///
    /// let client = AspenClient::connect("aspen...", timeout, None).await?;
    /// let cache_index = RpcCacheIndex::new(client);
    ///
    /// // Query a store path
    /// if let Some(entry) = cache_index.get("abc123...").await? {
    ///     println!("Found: {} ({} bytes)", entry.store_path, entry.nar_size);
    /// }
    /// ```
    pub struct RpcCacheIndex {
        client: AspenClient,
    }

    impl RpcCacheIndex {
        /// Create a new RPC cache index wrapping an Aspen client.
        pub fn new(client: AspenClient) -> Self {
            Self { client }
        }

        /// Convert a `CacheEntryResponse` from the RPC layer into an
        /// `aspen_cache::CacheEntry`.
        fn response_to_entry(resp: &aspen_client_api::CacheEntryResponse) -> CacheEntry {
            let mut entry = CacheEntry::new(
                resp.store_path.clone(),
                resp.store_hash.clone(),
                resp.blob_hash.clone(),
                resp.nar_size,
                resp.nar_hash.clone(),
                resp.created_at_ms,
                resp.created_by_node,
            );
            entry.file_size = resp.file_size;
            entry.references.clone_from(&resp.references);
            entry.deriver.clone_from(&resp.deriver);
            entry
        }
    }

    #[async_trait]
    impl aspen_cache::CacheIndex for RpcCacheIndex {
        async fn get(&self, store_hash: &str) -> Result<Option<CacheEntry>> {
            debug!(store_hash, "RpcCacheIndex::get");

            let request = ClientRpcRequest::CacheQuery {
                store_hash: store_hash.to_string(),
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::CacheQueryResult(result)) => {
                    if let Some(ref err) = result.error {
                        warn!(error = %err, "cache query returned error");
                        return Err(CacheError::KvStore { message: err.clone() });
                    }
                    if result.was_found {
                        Ok(result.entry.as_ref().map(Self::response_to_entry))
                    } else {
                        Ok(None)
                    }
                }
                Ok(resp) => {
                    warn!(?resp, "unexpected response to CacheQuery");
                    Err(CacheError::KvStore {
                        message: format!("unexpected response type: {:?}", resp),
                    })
                }
                Err(e) => Err(CacheError::KvStore {
                    message: format!("RPC failed: {}", e),
                }),
            }
        }

        async fn put(&self, _entry: CacheEntry) -> Result<()> {
            // Cache entries are created by the node during nix builds,
            // not by workers. Workers only read the index.
            Err(CacheError::KvStore {
                message: "RpcCacheIndex is read-only; cache entries are created by the cluster node".to_string(),
            })
        }

        async fn exists(&self, store_hash: &str) -> Result<bool> {
            debug!(store_hash, "RpcCacheIndex::exists");

            let request = ClientRpcRequest::CacheQuery {
                store_hash: store_hash.to_string(),
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::CacheQueryResult(result)) => {
                    if let Some(ref err) = result.error {
                        warn!(error = %err, "cache exists check returned error");
                        return Err(CacheError::KvStore { message: err.clone() });
                    }
                    Ok(result.was_found)
                }
                Ok(resp) => {
                    warn!(?resp, "unexpected response to CacheQuery (exists)");
                    Err(CacheError::KvStore {
                        message: format!("unexpected response type: {:?}", resp),
                    })
                }
                Err(e) => Err(CacheError::KvStore {
                    message: format!("RPC failed: {}", e),
                }),
            }
        }

        async fn stats(&self) -> Result<CacheStats> {
            debug!("RpcCacheIndex::stats");

            match self.client.send(ClientRpcRequest::CacheStats).await {
                Ok(ClientRpcResponse::CacheStatsResult(result)) => {
                    if let Some(ref err) = result.error {
                        warn!(error = %err, "cache stats returned error");
                        return Err(CacheError::KvStore { message: err.clone() });
                    }
                    Ok(CacheStats {
                        total_entries: result.total_entries,
                        total_nar_bytes: result.total_nar_bytes,
                        query_count: result.query_hits + result.query_misses,
                        hit_count: result.query_hits,
                        miss_count: result.query_misses,
                        last_updated: 0,
                    })
                }
                Ok(resp) => {
                    warn!(?resp, "unexpected response to CacheStats");
                    Err(CacheError::KvStore {
                        message: format!("unexpected response type: {:?}", resp),
                    })
                }
                Err(e) => Err(CacheError::KvStore {
                    message: format!("RPC failed: {}", e),
                }),
            }
        }
    }
}

#[cfg(feature = "cache-index")]
pub use rpc_cache_index::RpcCacheIndex;
