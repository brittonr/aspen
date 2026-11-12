use crate::error::Failure;
use crate::error::Result;

use super::Client;
use super::ClientConfig;
use super::LocalTier;
use super::TierPut;

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

    pub async fn put(&mut self, key: &str, bytes: &[u8]) -> Result<CachedObjectObservation> {
        let object_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
        let remote = self.remote.put(key, bytes).await?;
        let cached = matches!(self.tier.put(&object_ref, bytes)?, TierPut::Cached(_));
        Ok(CachedObjectObservation { object_ref, bytes: remote.bytes, cached })
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

    pub async fn get_raw(&self, key: &str) -> Result<Vec<u8>> {
        self.remote.get(key).await
    }

    pub fn cache_verified(&mut self, bytes: &[u8]) -> Result<TierPut> {
        let object_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
        self.tier.put(&object_ref, bytes)
    }

    pub async fn head(&self, key: &str) -> Result<Option<u64>> {
        self.remote.head(key).await
    }
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
