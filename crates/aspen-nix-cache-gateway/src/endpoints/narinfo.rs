//! Handler for GET /{hash}.narinfo endpoint.

use std::sync::Arc;

use aspen_cache::CacheEntry;
use aspen_cache::CacheIndex;
use http::Response;
use http::StatusCode;
use tracing::debug;
use tracing::instrument;

use crate::error::NixCacheError;
use crate::error::Result;
use crate::signing::NarinfoSigningProvider;

/// Handle GET /{hash}.narinfo request.
///
/// Returns narinfo metadata in the standard Nix format:
/// ```text
/// StorePath: /nix/store/abc123-hello
/// URL: nar/{blob_hash}.nar
/// Compression: none
/// NarHash: sha256:...
/// NarSize: 12345
/// References: dep1 dep2
/// Sig: cache-name:base64sig
/// ```
#[instrument(skip(cache_index, signer), fields(store_hash = %store_hash))]
pub async fn handle_narinfo<I>(
    store_hash: &str,
    cache_index: &I,
    signer: Option<&Arc<dyn NarinfoSigningProvider>>,
) -> Result<Response<String>>
where
    I: CacheIndex,
{
    // Validate store hash format (32 lowercase alphanumeric characters)
    if !is_valid_store_hash(store_hash) {
        return Err(NixCacheError::InvalidStoreHash {
            hash: store_hash.to_string(),
        });
    }

    // Look up cache entry
    let entry = cache_index
        .get(store_hash)
        .await
        .map_err(|e| NixCacheError::CacheIndex { message: e.to_string() })?;

    let entry = entry.ok_or_else(|| NixCacheError::NotFound {
        store_hash: store_hash.to_string(),
    })?;

    debug!(store_path = %entry.store_path, "cache hit");

    // Build narinfo response
    let body = build_narinfo(&entry, signer).await;

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/x-nix-narinfo")
        .header("Content-Length", body.len())
        .body(body)
        .map_err(|e| NixCacheError::ResponseBuild { message: e.to_string() })
}

/// Build narinfo text from a cache entry.
async fn build_narinfo(entry: &CacheEntry, signer: Option<&Arc<dyn NarinfoSigningProvider>>) -> String {
    let mut narinfo = String::new();

    // Required fields
    narinfo.push_str(&format!("StorePath: {}\n", entry.store_path));
    narinfo.push_str(&format!("URL: nar/{}.nar\n", entry.blob_hash));
    narinfo.push_str("Compression: none\n");
    narinfo.push_str(&format!("NarHash: {}\n", entry.nar_hash));
    narinfo.push_str(&format!("NarSize: {}\n", entry.nar_size));

    // Optional: FileHash and FileSize (same as NAR since no compression)
    if let Some(file_size) = entry.file_size {
        narinfo.push_str(&format!("FileSize: {}\n", file_size));
    }

    // References (space-separated store paths)
    if !entry.references.is_empty() {
        narinfo.push_str(&format!("References: {}\n", entry.references.join(" ")));
    }

    // Deriver (optional)
    if let Some(ref deriver) = entry.deriver {
        narinfo.push_str(&format!("Deriver: {}\n", deriver));
    }

    // Signature (optional)
    if let Some(signer) = signer {
        match signer.sign_narinfo(&entry.store_path, &entry.nar_hash, entry.nar_size, &entry.references).await {
            Ok(sig) => {
                narinfo.push_str(&format!("Sig: {}\n", sig));
            }
            Err(e) => {
                // Log error but don't fail the request - just omit signature
                tracing::warn!(error = %e, "Failed to sign narinfo");
            }
        }
    }

    narinfo
}

/// Validate store hash format.
///
/// Nix store hashes are 32 lowercase alphanumeric characters (base32).
fn is_valid_store_hash(hash: &str) -> bool {
    hash.len() == 32 && hash.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Extract store hash from request path.
///
/// Path format: `/{hash}.narinfo`
pub fn extract_store_hash(path: &str) -> Option<&str> {
    let path = path.trim_start_matches('/');
    path.strip_suffix(".narinfo")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_store_hash() {
        assert!(is_valid_store_hash("abcdefghijklmnopqrstuvwxyz012345"));
        assert!(is_valid_store_hash("00000000000000000000000000000000"));
    }

    #[test]
    fn test_invalid_store_hash() {
        assert!(!is_valid_store_hash("ABC")); // uppercase
        assert!(!is_valid_store_hash("abc")); // too short
        assert!(!is_valid_store_hash("abcdefghijklmnopqrstuvwxyz0123456")); // too long
        assert!(!is_valid_store_hash("abcdefghijklmnopqrstuvwxyz01234-")); // hyphen
    }

    #[test]
    fn test_extract_store_hash() {
        assert_eq!(extract_store_hash("/abc123.narinfo"), Some("abc123"));
        assert_eq!(extract_store_hash("abc123.narinfo"), Some("abc123"));
        assert_eq!(extract_store_hash("/abc123"), None);
        assert_eq!(extract_store_hash("/abc123.nar"), None);
    }
}
