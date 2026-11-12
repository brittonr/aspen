use crate::error::Failure;
use crate::error::Result;

use super::Client;
use super::ClientConfig;
use super::LocalTier;
use super::TierPut;

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
    pub fn record(&mut self, read: &TimedRead) {
        match read.source {
            ReadSource::Local => self.local.push(read.elapsed),
            ReadSource::Remote => self.remote.push(read.elapsed),
        }
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
        Ok(Self { remote: Client::connect(config)?, tier })
    }

    pub fn connect_with_timeout(config: ClientConfig, tier: LocalTier, timeout: std::time::Duration) -> Result<Self> {
        Ok(Self { remote: Client::connect_with_timeout(config, timeout)?, tier })
    }

    pub async fn put(&mut self, key: &str, bytes: &[u8]) -> Result<CachedObjectObservation> {
        let object_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
        let remote = self.remote.put(key, bytes).await?;
        let cached = matches!(self.tier.put(&object_ref, bytes)?, TierPut::Cached(_));
        Ok(CachedObjectObservation { object_ref, bytes: remote.bytes, cached })
    }

    pub async fn get_timed(&mut self, key: &str, expected_ref: &str) -> Result<TimedRead> {
        let started = std::time::Instant::now();
        if let Some(bytes) = self.tier.get(expected_ref)? {
            return Ok(TimedRead { bytes, source: ReadSource::Local, elapsed: started.elapsed() });
        }
        let bytes = self.remote.get(key).await?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != expected_ref {
            return Err(Failure::invalid_harness("remote object does not match the expected content identity"));
        }
        self.tier.put(expected_ref, &bytes)?;
        Ok(TimedRead { bytes, source: ReadSource::Remote, elapsed: started.elapsed() })
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

    pub async fn delete(&mut self, key: &str, expected_ref: &str) -> Result<()> {
        self.remote.delete(key).await?;
        self.tier.remove(expected_ref)
    }

    pub async fn get_raw(&self, key: &str) -> Result<Vec<u8>> {
        self.remote.get(key).await
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
}

fn percentiles(values: &[std::time::Duration]) -> (u64, u64) {
    if values.is_empty() {
        return (0, 0);
    }
    let mut micros = values.iter().map(|value| value.as_micros() as u64).collect::<Vec<_>>();
    micros.sort_unstable();
    let p50 = micros[(micros.len() - 1) * 50 / 100];
    let p99 = micros[(micros.len() - 1) * 99 / 100];
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
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let endpoint = server(vec![
            b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello",
        ])
        .await;
        let mut client = CachedClient::connect(config(endpoint), tier).expect("client");
        let observation = client.put("objects/hello", b"hello").await.expect("put");
        assert!(observation.cached);
        let expected = crate::preserves_rail::content_ref_from_bytes(b"hello");
        assert_eq!(client.get("objects/hello", &expected).await.expect("remote get"), b"hello");
        assert_eq!(client.get("objects/hello", &expected).await.expect("local get"), b"hello");
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
        assert_eq!(client.get("objects/live-roundtrip", &expected).await.expect("get"), b"live");
    }

    #[tokio::test]
    async fn latency_summary_separates_local_and_remote_reads() {
        let temporary = tempfile::tempdir().expect("tier");
        let tier = LocalTier::open(temporary.path(), 1024, false).expect("tier");
        let endpoint = server(vec![
            b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello",
        ])
        .await;
        let mut client = CachedClient::connect(config(endpoint), tier).expect("client");
        let expected = crate::preserves_rail::content_ref_from_bytes(b"hello");
        let mut recorder = LatencyRecorder::default();
        recorder.record(&client.get_timed("objects/hello", &expected).await.expect("remote"));
        recorder.record(&client.get_timed("objects/hello", &expected).await.expect("local"));
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
        let endpoint = server(vec![
            b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nwrong",
        ])
        .await;
        let mut client = CachedClient::connect(config(endpoint), tier).expect("client");
        let expected = crate::preserves_rail::content_ref_from_bytes(b"hello");
        assert!(client.get_and_cache("objects/hello", &expected).await.is_err());
        assert!(client.tier.get(&expected).expect("tier").is_none());
    }
}
