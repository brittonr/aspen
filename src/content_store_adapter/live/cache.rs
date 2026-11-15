use super::Client;
use super::ClientConfig;
use super::LocalTier;
use super::TierPut;
use crate::error::Failure;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadSource {
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub struct TimedRead {
    pub bytes: Vec<u8>,
    pub source: ReadSource,
    pub elapsed: std::time::Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LatencySummary {
    pub local_count: usize,
    pub remote_count: usize,
    pub local_p50_micros: u64,
    pub local_p99_micros: u64,
    pub remote_p50_micros: u64,
    pub remote_p99_micros: u64,
}

#[derive(Debug, Default)]
pub struct LatencyRecorder {
    local: Vec<std::time::Duration>,
    remote: Vec<std::time::Duration>,
}

impl LatencyRecorder {
    pub fn record(&mut self, read: &TimedRead) -> Result<()> {
        if self.local.len() + self.remote.len() >= 4096 {
            return Err(Failure::invalid_harness("latency observation bound exceeded"));
        }
        match read.source {
            ReadSource::Local => self.local.push(read.elapsed),
            ReadSource::Remote => self.remote.push(read.elapsed),
        }
        Ok(())
    }

    pub fn summary(&self) -> LatencySummary {
        let (local_p50, local_p99) = percentiles(&self.local);
        let (remote_p50, remote_p99) = percentiles(&self.remote);
        LatencySummary {
            local_count: self.local.len(),
            remote_count: self.remote.len(),
            local_p50_micros: local_p50,
            local_p99_micros: local_p99,
            remote_p50_micros: remote_p50,
            remote_p99_micros: remote_p99,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedObjectObservation {
    pub object_ref: String,
    pub bytes: u64,
    pub cached: bool,
}

/// Verifies remote bytes before caching and serves verified local hits without
/// touching the remote endpoint.
pub struct CachedClient {
    remote: Client,
    tier: LocalTier,
}

impl CachedClient {
    pub fn connect(config: ClientConfig, tier: LocalTier) -> Result<Self> {
        Ok(Self {
            remote: Client::connect(config)?,
            tier,
        })
    }

    pub fn connect_with_timeout(config: ClientConfig, tier: LocalTier, timeout: std::time::Duration) -> Result<Self> {
        Ok(Self {
            remote: Client::connect_with_timeout(config, timeout)?,
            tier,
        })
    }

    pub async fn put(&mut self, key: &str, bytes: &[u8]) -> Result<CachedObjectObservation> {
        let object_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
        let remote = self.remote.put(key, bytes).await?;
        let cached = matches!(self.tier.put(&object_ref, bytes)?, TierPut::Cached(_));
        Ok(CachedObjectObservation {
            object_ref,
            bytes: remote.bytes,
            cached,
        })
    }

    pub async fn get_timed(&mut self, key: &str, expected_ref: &str) -> Result<TimedRead> {
        let started = std::time::Instant::now();
        if let Some(bytes) = self.tier.get(expected_ref)? {
            return Ok(TimedRead {
                bytes,
                source: ReadSource::Local,
                elapsed: started.elapsed(),
            });
        }
        let bytes = self.remote.get(key).await?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != expected_ref {
            return Err(Failure::invalid_harness("remote object does not match the expected content identity"));
        }
        self.tier.put(expected_ref, &bytes)?;
        Ok(TimedRead {
            bytes,
            source: ReadSource::Remote,
            elapsed: started.elapsed(),
        })
    }

    pub async fn get(&self, key: &str, expected_ref: &str) -> Result<Vec<u8>> {
        if let Some(bytes) = self.tier.get(expected_ref)? {
            return Ok(bytes);
        }
        let bytes = self.remote.get(key).await?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != expected_ref {
            return Err(Failure::invalid_harness("remote object does not match the expected content identity"));
        }
        Ok(bytes)
    }

    pub async fn get_and_cache(&mut self, key: &str, expected_ref: &str) -> Result<Vec<u8>> {
        if let Some(bytes) = self.tier.get(expected_ref)? {
            return Ok(bytes);
        }
        let bytes = self.remote.get(key).await?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != expected_ref {
            return Err(Failure::invalid_harness("remote object does not match the expected content identity"));
        }
        self.tier.put(expected_ref, &bytes)?;
        Ok(bytes)
    }

    pub(super) async fn delete(&mut self, key: &str, expected_ref: &str) -> Result<()> {
        self.remote.delete(key).await?;
        self.tier.remove(expected_ref)
    }

    pub fn contains(&self, object_ref: &str) -> Result<bool> {
        Ok(self.tier.get(object_ref)?.is_some())
    }

    pub fn cache_verified(&mut self, bytes: &[u8]) -> Result<TierPut> {
        let object_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
        self.tier.put(&object_ref, bytes)
    }

    pub async fn head(&self, key: &str) -> Result<Option<u64>> {
        self.remote.head(key).await
    }

    pub(super) async fn get_chunk(
        &self,
        key: &str,
        chunk_ref: &str,
        chunk_size: usize,
        expected_length: u64,
    ) -> std::result::Result<Vec<u8>, molten_core::content_store_adapter::ContentFailure> {
        use molten_core::content_store_adapter::ContentFailure;
        let raw_ref =
            key.rsplit('/').next().map(|hex| format!("blake3:{hex}")).ok_or(ContentFailure::AdapterFailure)?;
        if let Some(bytes) =
            self.tier.get_bounded(&raw_ref, expected_length).map_err(|_| ContentFailure::AdapterFailure)?
        {
            if crate::chunk_store::hash_chunk(&bytes, chunk_size) == chunk_ref {
                return Ok(bytes);
            }
            return Err(ContentFailure::CorruptChunk);
        }
        self.remote.get_bounded(key, expected_length).await
    }

    pub(super) async fn put_classified(
        &mut self,
        key: &str,
        bytes: &[u8],
    ) -> std::result::Result<CachedObjectObservation, molten_core::content_store_adapter::ContentFailure> {
        use molten_core::content_store_adapter::ContentFailure;
        let remote = self.remote.put_classified(key, bytes).await?;
        let object_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
        let cached = matches!(
            self.tier.put(&object_ref, bytes).map_err(|_| ContentFailure::TransportDisconnected)?,
            TierPut::Cached(_)
        );
        Ok(CachedObjectObservation {
            object_ref,
            bytes: remote.bytes,
            cached,
        })
    }

    pub fn local_tier_bytes(&self) -> u64 {
        self.tier.used_bytes()
    }

    pub fn local_tier_capacity(&self) -> u64 {
        self.tier.max_bytes()
    }

    pub fn remote_policy(&self) -> super::RemotePolicyObservation {
        self.remote.policy()
    }

    /// Eviction affects residency only; it never deletes the remote copy.
    pub fn evict_local(&mut self, object_ref: &str) -> Result<()> {
        self.tier.remove(object_ref)
    }
}

fn percentiles(values: &[std::time::Duration]) -> (u64, u64) {
    if values.is_empty() {
        return (0, 0);
    }
    let mut micros = values.iter().map(|value| value.as_micros() as u64).collect::<Vec<_>>();
    micros.sort_unstable();
    let p50 = micros[(micros.len() * 50).div_ceil(100) - 1];
    let p99 = micros[(micros.len() * 99).div_ceil(100) - 1];
    (p50, p99)
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    use super::*;

    async fn server(responses: Vec<&'static [u8]>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            for response in responses {
                let (mut socket, _) = listener.accept().await.expect("accept");
                let mut buffer = [0_u8; 2048];
                let _ = socket.read(&mut buffer).await.expect("read");
                socket.write_all(response).await.expect("write");
            }
        });
        format!("http://{address}")
    }

    fn config(endpoint: String) -> ClientConfig {
        ClientConfig {
            endpoint,
            region: "us-east-1".to_string(),
            bucket: "bucket".to_string(),
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
            session_token: None,
        }
    }

    #[tokio::test]
    async fn local_hit_makes_no_remote_request() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let endpoint = format!("http://{}", listener.local_addr().expect("address"));
        let temporary = tempfile::tempdir().expect("tier");
        let mut tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let expected = crate::preserves_rail::content_ref_from_bytes(b"hello");
        tier.put(&expected, b"hello").expect("seed local cache");
        let client = CachedClient::connect(config(endpoint), tier).expect("client");
        assert_eq!(client.get("objects/hello", &expected).await.expect("local get"), b"hello");
        assert!(tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept()).await.is_err());
    }

    #[tokio::test]
    #[ignore = "requires MOLTEN_S3_ENDPOINT and MOLTEN_S3_BUCKET"]
    async fn live_endpoint_roundtrip() {
        let endpoint = std::env::var("MOLTEN_S3_ENDPOINT").expect("endpoint");
        let bucket = std::env::var("MOLTEN_S3_BUCKET").expect("bucket");
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 4096, false).expect("tier");
        let mut client = CachedClient::connect(
            ClientConfig {
                endpoint,
                region: "us-east-1".to_string(),
                bucket,
                access_key: "molten".to_string(),
                secret_key: "molten".to_string(),
                session_token: None,
            },
            tier,
        )
        .expect("client");
        let observation = client.put("objects/live-roundtrip", b"live").await.expect("put");
        assert_eq!(observation.bytes, 4);
        let expected = crate::preserves_rail::content_ref_from_bytes(b"live");
        client.evict_local(&expected).expect("force remote roundtrip");
        assert_eq!(client.get("objects/live-roundtrip", &expected).await.expect("get"), b"live");
    }

    #[tokio::test]
    async fn latency_summary_separates_local_and_remote_reads() {
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let endpoint = server(vec![b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello"]).await;
        let mut client = CachedClient::connect(config(endpoint), tier).expect("client");
        let expected = crate::preserves_rail::content_ref_from_bytes(b"hello");
        let mut recorder = LatencyRecorder::default();
        recorder
            .record(&client.get_timed("objects/hello", &expected).await.expect("remote"))
            .expect("record remote");
        recorder
            .record(&client.get_timed("objects/hello", &expected).await.expect("local"))
            .expect("record local");
        let summary = recorder.summary();
        assert_eq!(summary.remote_count, 1);
        assert_eq!(summary.local_count, 1);
        assert!(summary.remote_p50_micros <= summary.remote_p99_micros);
        assert!(summary.local_p50_micros <= summary.local_p99_micros);
    }

    #[tokio::test]
    async fn corrupt_remote_bytes_are_not_cached() {
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let endpoint = server(vec![b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nwrong"]).await;
        let mut client = CachedClient::connect(config(endpoint), tier).expect("client");
        let expected = crate::preserves_rail::content_ref_from_bytes(b"hello");
        assert!(client.get_and_cache("objects/hello", &expected).await.is_err());
        assert!(client.tier.get(&expected).expect("tier").is_none());
    }

    #[tokio::test]
    async fn cold_get_at_capacity_is_verified_remote_only() {
        let temporary = tempfile::tempdir().expect("tier");
        let mut tier = LocalTier::open(temporary.path(), 5, false).expect("tier");
        let first = crate::preserves_rail::content_ref_from_bytes(b"first");
        tier.put(&first, b"first").expect("fill tier");
        let endpoint = server(vec![b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello"]).await;
        let mut client = CachedClient::connect(config(endpoint), tier).expect("client");
        let second = crate::preserves_rail::content_ref_from_bytes(b"hello");
        assert_eq!(client.get_and_cache("objects/second", &second).await.expect("cold get"), b"hello");
        assert_eq!(client.local_tier_bytes(), 5);
        assert!(client.contains(&first).expect("first resident"));
        assert!(!client.contains(&second).expect("second remote only"));
    }
}
