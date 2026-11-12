#![allow(
    tigerstyle::non_trait_imports,
    reason = "the S3 client names its HTTP, signing, and hash imports at the protocol boundary"
)]

use std::time::Duration;

use hmac::Hmac;
use hmac::Mac;
use sha2::Digest;
use sha2::Sha256;
use time::OffsetDateTime;
use time::macros::format_description;

use crate::error::Failure;
use crate::error::Result;

const MAX_KEY_BYTES: usize = 1024;
const MAX_OBJECT_BYTES: u64 = 64 * 1_048_576;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Credentials and destination for one path-style S3-compatible object store.
#[derive(Clone)]
pub struct ClientConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectObservation {
    pub object_ref: String,
    pub bytes: u64,
}

/// Bounded path-style S3 client with SigV4 request signing.
pub struct Client {
    client: reqwest::Client,
    config: ClientConfig,
}

impl Client {
    pub fn connect(config: ClientConfig) -> Result<Self> {
        validate_config(&config)?;
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| Failure::invalid_harness(format!("s3 client: {error}")))?;
        Ok(Self { client, config })
    }

    pub async fn put(&self, key: &str, bytes: &[u8]) -> Result<ObjectObservation> {
        validate_key(key)?;
        if bytes.len() as u64 > MAX_OBJECT_BYTES {
            return Err(Failure::invalid_harness("s3 object exceeds its bound"));
        }
        let request = self.signed(reqwest::Method::PUT, key, bytes)?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|error| Failure::invalid_harness(format!("s3 put: {error}")))?;
        require_success(response.status(), "put").await?;
        Ok(ObjectObservation {
            object_ref: crate::preserves_rail::content_ref_from_bytes(bytes),
            bytes: bytes.len() as u64,
        })
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>> {
        validate_key(key)?;
        let request = self.signed(reqwest::Method::GET, key, b"")?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|error| Failure::invalid_harness(format!("s3 get: {error}")))?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Failure::invalid_harness("s3 object is missing"));
        }
        if !status.is_success() {
            return Err(Failure::invalid_harness(format!("s3 get returned {status}")));
        }
        if response.content_length().is_some_and(|length| length > MAX_OBJECT_BYTES) {
            return Err(Failure::invalid_harness("s3 object exceeds its bound"));
        }
        let bytes =
            response.bytes().await.map_err(|error| Failure::invalid_harness(format!("s3 get body: {error}")))?;
        if bytes.len() as u64 > MAX_OBJECT_BYTES {
            return Err(Failure::invalid_harness("s3 object exceeds its bound"));
        }
        Ok(bytes.to_vec())
    }

    pub async fn head(&self, key: &str) -> Result<Option<u64>> {
        validate_key(key)?;
        let request = self.signed(reqwest::Method::HEAD, key, b"")?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|error| Failure::invalid_harness(format!("s3 head: {error}")))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(Failure::invalid_harness(format!("s3 head returned {}", response.status())));
        }
        Ok(response.content_length())
    }

    fn signed(&self, method: reqwest::Method, key: &str, body: &[u8]) -> Result<reqwest::Request> {
        let url = format!("{}/{}/{}", self.config.endpoint.trim_end_matches('/'), self.config.bucket, key);
        let url = reqwest::Url::parse(&url).map_err(|error| Failure::invalid_harness(format!("s3 url: {error}")))?;
        let host = host_header(&url)?;
        let now = OffsetDateTime::now_utc();
        let amz_date = now
            .format(format_description!("[year][month][day]T[hour][minute][second]Z"))
            .map_err(|error| Failure::invalid_harness(format!("s3 date: {error}")))?;
        let date = &amz_date[..8];
        let payload_hash = sha256_hex(body);
        let path = url.path().to_string();
        let mut canonical_headers =
            format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
        let mut signed_headers = "host;x-amz-content-sha256;x-amz-date".to_string();
        if let Some(token) = &self.config.session_token {
            canonical_headers.push_str(&format!("x-amz-security-token:{token}\n"));
            signed_headers.push_str(";x-amz-security-token");
        }
        let canonical_request =
            format!("{}\n{path}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}", method.as_str());
        let scope = format!("{date}/{}/s3/aws4_request", self.config.region);
        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}", sha256_hex(canonical_request.as_bytes()));
        let signature = self.signature(date, &string_to_sign);
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            self.config.access_key
        );
        let mut builder = self
            .client
            .request(method, url)
            .header("x-amz-content-sha256", payload_hash)
            .header("x-amz-date", amz_date)
            .header("authorization", authorization);
        if let Some(token) = &self.config.session_token {
            builder = builder.header("x-amz-security-token", token);
        }
        if body.is_empty() {
            builder.build().map_err(|error| Failure::invalid_harness(format!("s3 request: {error}")))
        } else {
            builder
                .body(body.to_vec())
                .build()
                .map_err(|error| Failure::invalid_harness(format!("s3 request: {error}")))
        }
    }

    fn signature(&self, date: &str, string_to_sign: &str) -> String {
        let key = signing_key(&self.config.secret_key, date, &self.config.region);
        hex::encode(hmac_sha256(&key, string_to_sign.as_bytes()))
    }
}

fn validate_config(config: &ClientConfig) -> Result<()> {
    if !(config.endpoint.starts_with("http://") || config.endpoint.starts_with("https://"))
        || config.region.is_empty()
        || config.bucket.is_empty()
        || config.access_key.is_empty()
        || config.secret_key.is_empty()
    {
        return Err(Failure::invalid_harness("s3 configuration is incomplete or has a non-HTTP endpoint"));
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<()> {
    if key.is_empty()
        || key.len() > MAX_KEY_BYTES
        || key.starts_with('/')
        || key.contains("..")
        || !key.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/'))
    {
        return Err(Failure::invalid_harness("s3 object key is not bounded and canonical"));
    }
    Ok(())
}

fn host_header(url: &reqwest::Url) -> Result<String> {
    let host = url.host_str().ok_or_else(|| Failure::invalid_harness("s3 endpoint has no host"))?;
    Ok(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    })
}

async fn require_success(status: reqwest::StatusCode, operation: &str) -> Result<()> {
    if status.is_success() {
        return Ok(());
    }
    Err(Failure::invalid_harness(format!("s3 {operation} returned {status}")))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

type HmacSha256 = Hmac<Sha256>;

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let Ok(mut mac) = <HmacSha256 as Mac>::new_from_slice(key) else {
        return Vec::new();
    };
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn signing_key(secret: &str, date: &str, region: &str) -> Vec<u8> {
    let date_key = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let region_key = hmac_sha256(&date_key, region.as_bytes());
    let service_key = hmac_sha256(&region_key, b"s3");
    hmac_sha256(&service_key, b"aws4_request")
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    use super::*;

    #[test]
    fn signing_matches_the_published_sigv4_example_vector() {
        let key = signing_key("wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY", "20120215", "us-east-1");
        assert_eq!(hex::encode(key), "f22dfcb5da3bb8d6c924d26e9bb28e9f96ba691a1b1791d907c06f10ede94251");
    }

    #[tokio::test]
    async fn put_and_get_are_bounded_and_signed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("address");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept put");
            let request = read_request(&mut socket).await;
            assert!(request.contains("PUT /bucket/objects/one"));
            assert!(request.contains("AWS4-HMAC-SHA256 Credential=access/"));
            assert!(request.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                .await
                .expect("put response");
            let (mut socket, _) = listener.accept().await.expect("accept get");
            let request = read_request(&mut socket).await;
            assert!(request.contains("GET /bucket/objects/one"));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello")
                .await
                .expect("get response");
        });
        let store = Client::connect(ClientConfig {
            endpoint: format!("http://{address}"),
            region: "us-east-1".to_string(),
            bucket: "bucket".to_string(),
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
            session_token: None,
        })
        .expect("store");
        let observation = store.put("objects/one", b"hello").await.expect("put");
        assert_eq!(observation.bytes, 5);
        assert_eq!(store.get("objects/one").await.expect("get"), b"hello");
        server.await.expect("server");
    }

    async fn read_request(socket: &mut tokio::net::TcpStream) -> String {
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let read = socket.read(&mut chunk).await.expect("read");
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
            if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&buffer).into_owned()
    }
}
