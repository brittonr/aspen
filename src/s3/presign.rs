/// Presigned URL generation for S3-compatible requests.
///
/// This module provides utilities for generating presigned URLs that grant
/// temporary access to S3 objects without exposing credentials.
///
/// # AWS Signature Version 4
///
/// Presigned URLs use AWS Signature Version 4 (SigV4) for authentication.
/// The signature is embedded in the URL query parameters, allowing clients
/// without credentials to access protected resources for a limited time.
///
/// # Example
///
/// ```no_run
/// use aspen::s3::presign::{PresignedUrlBuilder, HttpMethod};
///
/// let url = PresignedUrlBuilder::new(
///     "http://localhost:9000",
///     "my-bucket",
///     "path/to/object.txt",
/// )
/// .method(HttpMethod::Get)
/// .expires_in_seconds(3600) // 1 hour
/// .access_key("my-access-key")
/// .secret_key("my-secret-key")
/// .build()
/// .unwrap();
///
/// println!("Presigned URL: {}", url);
/// ```
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// HTTP methods supported for presigned URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    /// GET - retrieve objects
    Get,
    /// PUT - upload objects
    Put,
    /// DELETE - remove objects
    Delete,
    /// HEAD - retrieve object metadata
    Head,
}

impl HttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
        }
    }
}

/// Maximum expiration time for presigned URLs (7 days in seconds).
const MAX_EXPIRES_SECONDS: u64 = 604800;

/// Default expiration time (1 hour in seconds).
const DEFAULT_EXPIRES_SECONDS: u64 = 3600;

/// Default AWS region for S3.
const DEFAULT_REGION: &str = "us-east-1";

/// Default service name.
const DEFAULT_SERVICE: &str = "s3";

/// Builder for creating presigned URLs.
pub struct PresignedUrlBuilder {
    endpoint: String,
    bucket: String,
    key: String,
    method: HttpMethod,
    expires_seconds: u64,
    access_key: String,
    secret_key: String,
    region: String,
    timestamp: Option<DateTime<Utc>>,
}

impl PresignedUrlBuilder {
    /// Create a new presigned URL builder.
    pub fn new(endpoint: &str, bucket: &str, key: &str) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            bucket: bucket.to_string(),
            key: key.trim_start_matches('/').to_string(),
            method: HttpMethod::Get,
            expires_seconds: DEFAULT_EXPIRES_SECONDS,
            access_key: String::new(),
            secret_key: String::new(),
            region: DEFAULT_REGION.to_string(),
            timestamp: None,
        }
    }

    /// Set the HTTP method.
    pub fn method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    /// Set the expiration time in seconds.
    ///
    /// Maximum is 7 days (604800 seconds).
    pub fn expires_in_seconds(mut self, seconds: u64) -> Self {
        self.expires_seconds = seconds.min(MAX_EXPIRES_SECONDS);
        self
    }

    /// Set the access key.
    pub fn access_key(mut self, key: &str) -> Self {
        self.access_key = key.to_string();
        self
    }

    /// Set the secret key.
    pub fn secret_key(mut self, key: &str) -> Self {
        self.secret_key = key.to_string();
        self
    }

    /// Set the AWS region (default: us-east-1).
    pub fn region(mut self, region: &str) -> Self {
        self.region = region.to_string();
        self
    }

    /// Set a specific timestamp (for testing).
    pub fn timestamp(mut self, ts: DateTime<Utc>) -> Self {
        self.timestamp = Some(ts);
        self
    }

    /// Build the presigned URL.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Access key or secret key is not set
    /// - Bucket or key is empty
    pub fn build(self) -> Result<String, PresignError> {
        // Validate inputs
        if self.access_key.is_empty() {
            return Err(PresignError::MissingAccessKey);
        }
        if self.secret_key.is_empty() {
            return Err(PresignError::MissingSecretKey);
        }
        if self.bucket.is_empty() {
            return Err(PresignError::InvalidBucket);
        }
        if self.key.is_empty() {
            return Err(PresignError::InvalidKey);
        }

        // Generate timestamp
        let now = self.timestamp.unwrap_or_else(Utc::now);
        let date_str = now.format("%Y%m%d").to_string();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

        // Build credential scope
        let credential_scope = format!(
            "{}/{}/{}/aws4_request",
            date_str, self.region, DEFAULT_SERVICE
        );

        // Build canonical URI (path-style)
        let canonical_uri = format!("/{}/{}", self.bucket, self.key);
        let encoded_uri = uri_encode_path(&canonical_uri);

        // Build canonical query string
        let mut query_params = BTreeMap::new();
        query_params.insert(
            "X-Amz-Algorithm".to_string(),
            "AWS4-HMAC-SHA256".to_string(),
        );
        query_params.insert(
            "X-Amz-Credential".to_string(),
            format!("{}/{}", self.access_key, credential_scope),
        );
        query_params.insert("X-Amz-Date".to_string(), amz_date.clone());
        query_params.insert(
            "X-Amz-Expires".to_string(),
            self.expires_seconds.to_string(),
        );
        query_params.insert("X-Amz-SignedHeaders".to_string(), "host".to_string());

        let canonical_query_string = build_canonical_query_string(&query_params);

        // Extract host from endpoint
        let host = extract_host(&self.endpoint)?;

        // Build canonical headers
        let canonical_headers = format!("host:{}\n", host);
        let signed_headers = "host";

        // For presigned URLs, we use UNSIGNED-PAYLOAD
        let hashed_payload = "UNSIGNED-PAYLOAD";

        // Build canonical request
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            self.method.as_str(),
            encoded_uri,
            canonical_query_string,
            canonical_headers,
            signed_headers,
            hashed_payload
        );

        // Hash the canonical request
        let canonical_request_hash = hex_sha256(canonical_request.as_bytes());

        // Build string to sign
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, credential_scope, canonical_request_hash
        );

        // Calculate signature
        let signature = calculate_signature(
            &self.secret_key,
            &date_str,
            &self.region,
            DEFAULT_SERVICE,
            &string_to_sign,
        );

        // Build final URL
        let presigned_url = format!(
            "{}{}?{}&X-Amz-Signature={}",
            self.endpoint, canonical_uri, canonical_query_string, signature
        );

        Ok(presigned_url)
    }
}

/// Errors that can occur during presigned URL generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresignError {
    /// Access key was not provided.
    MissingAccessKey,
    /// Secret key was not provided.
    MissingSecretKey,
    /// Bucket name is invalid.
    InvalidBucket,
    /// Object key is invalid.
    InvalidKey,
    /// Endpoint URL is invalid.
    InvalidEndpoint,
}

impl std::fmt::Display for PresignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresignError::MissingAccessKey => write!(f, "Access key is required"),
            PresignError::MissingSecretKey => write!(f, "Secret key is required"),
            PresignError::InvalidBucket => write!(f, "Bucket name is invalid"),
            PresignError::InvalidKey => write!(f, "Object key is invalid"),
            PresignError::InvalidEndpoint => write!(f, "Endpoint URL is invalid"),
        }
    }
}

impl std::error::Error for PresignError {}

/// URI-encode a path component (preserving slashes).
///
/// Tiger Style: Uses write! macro to avoid string allocations in loops.
fn uri_encode_path(path: &str) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(path.len() * 3);
    for c in path.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' => {
                result.push(c);
            }
            _ => {
                // Encode multi-byte characters correctly
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                for b in encoded.as_bytes() {
                    let _ = write!(result, "%{:02X}", b);
                }
            }
        }
    }
    result
}

/// URI-encode a query parameter value.
///
/// Tiger Style: Uses write! macro to avoid string allocations in loops.
fn uri_encode_value(value: &str) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(value.len() * 3);
    for c in value.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                result.push(c);
            }
            _ => {
                // Encode multi-byte characters correctly
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                for b in encoded.as_bytes() {
                    let _ = write!(result, "%{:02X}", b);
                }
            }
        }
    }
    result
}

/// Build canonical query string from parameters.
fn build_canonical_query_string(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", uri_encode_value(k), uri_encode_value(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Calculate SHA-256 hash and return as hex string.
fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Calculate HMAC-SHA256.
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC key should be valid");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Calculate the AWS SigV4 signature.
fn calculate_signature(
    secret_key: &str,
    date: &str,
    region: &str,
    service: &str,
    string_to_sign: &str,
) -> String {
    let k_date = hmac_sha256(format!("AWS4{}", secret_key).as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hmac_sha256(&k_signing, string_to_sign.as_bytes());
    hex::encode(signature)
}

/// Extract host from endpoint URL.
fn extract_host(endpoint: &str) -> Result<String, PresignError> {
    // Simple extraction: remove protocol prefix
    let host = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("https://"))
        .unwrap_or(endpoint);

    // Remove trailing path if any
    let host = host.split('/').next().unwrap_or(host);

    if host.is_empty() {
        return Err(PresignError::InvalidEndpoint);
    }

    Ok(host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_presigned_url_basic() {
        let url = PresignedUrlBuilder::new("http://localhost:9000", "test-bucket", "test-key.txt")
            .method(HttpMethod::Get)
            .expires_in_seconds(3600)
            .access_key("test-access-key")
            .secret_key("test-secret-key")
            .timestamp(Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap())
            .build()
            .unwrap();

        assert!(url.contains("test-bucket"));
        assert!(url.contains("test-key.txt"));
        assert!(url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
        assert!(url.contains("X-Amz-Credential=test-access-key"));
        assert!(url.contains("X-Amz-Expires=3600"));
        assert!(url.contains("X-Amz-Signature="));
    }

    #[test]
    fn test_presigned_url_put() {
        let url = PresignedUrlBuilder::new("http://localhost:9000", "bucket", "key")
            .method(HttpMethod::Put)
            .access_key("access")
            .secret_key("secret")
            .build()
            .unwrap();

        assert!(url.starts_with("http://localhost:9000/bucket/key?"));
    }

    #[test]
    fn test_missing_access_key() {
        let result = PresignedUrlBuilder::new("http://localhost:9000", "bucket", "key")
            .secret_key("secret")
            .build();

        assert_eq!(result, Err(PresignError::MissingAccessKey));
    }

    #[test]
    fn test_missing_secret_key() {
        let result = PresignedUrlBuilder::new("http://localhost:9000", "bucket", "key")
            .access_key("access")
            .build();

        assert_eq!(result, Err(PresignError::MissingSecretKey));
    }

    #[test]
    fn test_uri_encoding() {
        // Test path encoding preserves slashes
        assert_eq!(uri_encode_path("/bucket/key"), "/bucket/key");
        assert_eq!(
            uri_encode_path("/bucket/path/to/key"),
            "/bucket/path/to/key"
        );

        // Test value encoding escapes slashes
        assert_eq!(uri_encode_value("path/to/key"), "path%2Fto%2Fkey");

        // Test special characters
        assert_eq!(uri_encode_value("hello world"), "hello%20world");
        assert_eq!(uri_encode_value("a+b"), "a%2Bb");
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(
            extract_host("http://localhost:9000").unwrap(),
            "localhost:9000"
        );
        assert_eq!(
            extract_host("https://s3.amazonaws.com").unwrap(),
            "s3.amazonaws.com"
        );
        assert_eq!(
            extract_host("http://localhost:9000/path").unwrap(),
            "localhost:9000"
        );
    }

    #[test]
    fn test_max_expires() {
        let builder = PresignedUrlBuilder::new("http://localhost:9000", "bucket", "key")
            .expires_in_seconds(1_000_000); // > MAX_EXPIRES_SECONDS

        // Should be capped at MAX_EXPIRES_SECONDS
        assert_eq!(builder.expires_seconds, MAX_EXPIRES_SECONDS);
    }
}
