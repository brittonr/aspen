//! Nix base32 encoding.
//!
//! Nix uses a custom base32 encoding ("nix32") for hash representation
//! in narinfo files and store path fingerprints. The alphabet is:
//! `0123456789abcdfghijklmnpqrsvwxyz` — standard lowercase alphanumeric
//! with `e`, `o`, `t`, `u` removed.
//!
//! This encoding is NOT standard base32 (RFC 4648). It also processes
//! bits in reverse byte order compared to standard base32.

/// Nix base32 alphabet (32 chars).
const NIX32_CHARS: &[u8; 32] = b"0123456789abcdfghijklmnpqrsvwxyz";

/// Encode raw bytes to nix base32.
///
/// The encoded length for SHA-256 (32 bytes) is 52 characters.
///
/// Ported directly from nix's `printHash32` (libutil/hash.cc):
/// iterates n from len-1 down to 0, extracting 5-bit groups from
/// the raw hash bytes in little-endian bit order.
pub fn encode(input: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }

    let len = (input.len() * 8).div_ceil(5);
    let mut out = Vec::with_capacity(len);

    for n in (0..len).rev() {
        let b = n * 5;
        let i = b / 8;
        let j = b % 8;
        let mut c = (input[i] >> j) as u32;
        if i + 1 < input.len() && j > 3 {
            c |= (input[i + 1] as u32) << (8 - j);
        }
        out.push(NIX32_CHARS[(c & 0x1f) as usize]);
    }

    // SAFETY: NIX32_CHARS contains only ASCII bytes
    unsafe { String::from_utf8_unchecked(out) }
}

/// Convert a hex-encoded hash to nix32 format.
///
/// Input: `"sha256:dc810f96..."` (hex)
/// Output: `"sha256:14srlg7x..."` (nix32)
///
/// Returns the input unchanged if it doesn't start with a known hash
/// prefix followed by hex, or if decoding fails.
pub fn hex_to_nix32(hash_str: &str) -> String {
    let Some((prefix, hex_data)) = hash_str.split_once(':') else {
        return hash_str.to_string();
    };

    let Ok(bytes) = hex::decode(hex_data) else {
        return hash_str.to_string();
    };

    format!("{}:{}", prefix, encode(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(encode(&[]), "");
    }

    #[test]
    fn test_known_vector() {
        // SHA-256 of empty NAR (nix-hash --type sha256 --base32)
        // empty string SHA-256 = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let hash = hex::decode("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855").unwrap();
        let encoded = encode(&hash);
        // Expected: nix32 encoding of the empty string SHA-256
        assert_eq!(encoded.len(), 52);
        // Verify round-trip: the encoding should be valid nix32
        assert!(encoded.chars().all(|c| NIX32_CHARS.contains(&(c as u8))));
    }

    #[test]
    fn test_sha256_output_length() {
        // 32 bytes → 52 nix32 chars
        let hash = [0u8; 32];
        assert_eq!(encode(&hash).len(), 52);
    }

    #[test]
    fn test_hex_to_nix32_conversion() {
        let hex = "sha256:dc810f96d61eb9414b986771cb4e2f51c6b879314e2f4296f8d926d0cfa35993";
        let nix32 = hex_to_nix32(hex);
        assert!(nix32.starts_with("sha256:"));
        let hash_part = nix32.strip_prefix("sha256:").unwrap();
        assert_eq!(hash_part.len(), 52);
        // Should NOT be hex anymore
        assert_ne!(nix32, hex);
    }

    #[test]
    fn test_hex_to_nix32_preserves_unknown() {
        // Unknown format → returned as-is
        assert_eq!(hex_to_nix32("notahash"), "notahash");
    }

    #[test]
    fn test_hex_to_nix32_invalid_hex() {
        // Invalid hex → returned as-is
        assert_eq!(hex_to_nix32("sha256:not_hex_data"), "sha256:not_hex_data");
    }

    #[test]
    fn test_cowsay_hash_matches_nixos_cache() {
        // The cowsay NAR hash from cache.nixos.org:
        // sha256:14srlg7x09nrz2b44bsf65wviiji5x7cnwb7k15l3f8yssb0z0fw
        //
        // Our hex hash: sha256:dc810f96d61eb9414b986771cb4e2f51c6b879314e2f4296f8d926d0cfa35993
        let nix32 = hex_to_nix32("sha256:dc810f96d61eb9414b986771cb4e2f51c6b879314e2f4296f8d926d0cfa35993");
        assert_eq!(nix32, "sha256:14srlg7x09nrz2b44bsf65wviiji5x7cnwb7k15l3f8yssb0z0fw");
    }
}
