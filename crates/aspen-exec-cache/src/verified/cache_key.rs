//! Pure cache key computation.
//!
//! Formally verified — see `verus/cache_key_spec.rs` for proofs.
//!
//! The cache key is `BLAKE3(command || args || env_hash || sorted_input_hashes)`.
//! Sorting input hashes ensures determinism regardless of file access order.

use crate::types::CacheKey;

/// Compute a cache key from command, arguments, environment hash, and input file hashes.
///
/// Input hashes are sorted before hashing to ensure determinism regardless
/// of file access order. The environment hash covers a curated subset of
/// variables (PATH, CC, CXX, etc.) to avoid cache-busting from irrelevant
/// environment changes.
#[inline]
pub fn compute_cache_key(
    command: &[u8],
    args: &[&[u8]],
    env_hash: &[u8; 32],
    input_hashes: &mut [[u8; 32]],
) -> CacheKey {
    // Sort input hashes for determinism regardless of access order
    input_hashes.sort_unstable();

    let mut hasher = blake3::Hasher::new();

    // Hash command
    hasher.update(&(command.len() as u64).to_le_bytes());
    hasher.update(command);

    // Hash each arg with length prefix to prevent concatenation collisions
    hasher.update(&(args.len() as u64).to_le_bytes());
    for arg in args {
        hasher.update(&(arg.len() as u64).to_le_bytes());
        hasher.update(arg);
    }

    // Hash environment
    hasher.update(env_hash);

    // Hash sorted input file hashes
    hasher.update(&(input_hashes.len() as u64).to_le_bytes());
    for h in input_hashes.iter() {
        hasher.update(h);
    }

    CacheKey(*hasher.finalize().as_bytes())
}

/// Compute a deterministic hash of a curated environment variable set.
///
/// Variables are sorted by name before hashing. Missing variables are
/// included as empty values to distinguish "not set" from "set to empty".
#[inline]
pub fn compute_env_hash(env_vars: &[(&str, Option<&str>)]) -> [u8; 32] {
    let mut sorted: Vec<(&str, Option<&str>)> = env_vars.to_vec();
    sorted.sort_by_key(|(name, _)| *name);

    let mut hasher = blake3::Hasher::new();
    hasher.update(&(sorted.len() as u64).to_le_bytes());

    for (name, value) in &sorted {
        // Hash name with length prefix
        hasher.update(&(name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());

        // Hash value presence + content
        match value {
            Some(v) => {
                hasher.update(&[1u8]); // present
                hasher.update(&(v.len() as u64).to_le_bytes());
                hasher.update(v.as_bytes());
            }
            None => {
                hasher.update(&[0u8]); // absent
            }
        }
    }

    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_different_order_produce_same_key() {
        let command = b"rustc";
        let args: &[&[u8]] = &[b"src/main.rs", b"-o", b"target/main"];
        let env_hash = [0xaa; 32];
        let h1 = [0x11; 32];
        let h2 = [0x22; 32];
        let h3 = [0x33; 32];

        let mut inputs_a = [h1, h2, h3];
        let key_a = compute_cache_key(command, args, &env_hash, &mut inputs_a);

        let mut inputs_b = [h3, h1, h2];
        let key_b = compute_cache_key(command, args, &env_hash, &mut inputs_b);

        assert_eq!(key_a, key_b, "same inputs in different order must produce same key");
    }

    #[test]
    fn different_commands_produce_different_keys() {
        let env_hash = [0; 32];
        let mut inputs = [[0x11; 32]];

        let key_a = compute_cache_key(b"rustc", &[b"main.rs"], &env_hash, &mut inputs.clone());
        let key_b = compute_cache_key(b"gcc", &[b"main.rs"], &env_hash, &mut inputs);

        assert_ne!(key_a, key_b);
    }

    #[test]
    fn different_env_produces_different_keys() {
        let command = b"rustc";
        let args: &[&[u8]] = &[b"main.rs"];
        let mut inputs = [[0x11; 32]];

        let env_a = [0xaa; 32];
        let key_a = compute_cache_key(command, args, &env_a, &mut inputs.clone());

        let env_b = [0xbb; 32];
        let key_b = compute_cache_key(command, args, &env_b, &mut inputs);

        assert_ne!(key_a, key_b);
    }

    #[test]
    fn different_input_hashes_produce_different_keys() {
        let command = b"rustc";
        let args: &[&[u8]] = &[b"main.rs"];
        let env_hash = [0; 32];

        let mut inputs_a = [[0x11; 32]];
        let key_a = compute_cache_key(command, args, &env_hash, &mut inputs_a);

        let mut inputs_b = [[0x22; 32]];
        let key_b = compute_cache_key(command, args, &env_hash, &mut inputs_b);

        assert_ne!(key_a, key_b);
    }

    #[test]
    fn empty_inputs_produce_valid_key() {
        let key = compute_cache_key(b"echo", &[], &[0; 32], &mut []);
        assert_ne!(key.0, [0; 32], "should produce a non-zero hash");
    }

    #[test]
    fn env_hash_deterministic_regardless_of_order() {
        let vars_a = vec![("PATH", Some("/usr/bin")), ("CC", Some("gcc")), ("HOME", Some("/root"))];
        let hash_a = compute_env_hash(&vars_a);

        let vars_b = vec![("HOME", Some("/root")), ("PATH", Some("/usr/bin")), ("CC", Some("gcc"))];
        let hash_b = compute_env_hash(&vars_b);

        assert_eq!(hash_a, hash_b, "env hash must be order-independent");
    }

    #[test]
    fn env_hash_distinguishes_missing_from_empty() {
        let with_empty = vec![("PATH", Some(""))];
        let with_none = vec![("PATH", None)];

        let hash_empty = compute_env_hash(&with_empty);
        let hash_none = compute_env_hash(&with_none);

        assert_ne!(hash_empty, hash_none, "empty value != missing value");
    }

    #[test]
    fn arg_concatenation_collision_prevented() {
        // "ab" + "c" vs "a" + "bc" — length-prefixed hashing prevents collision
        let env = [0; 32];
        let mut inputs = [[0x11; 32]];

        let key_a = compute_cache_key(b"cmd", &[b"ab", b"c"], &env, &mut inputs.clone());
        let key_b = compute_cache_key(b"cmd", &[b"a", b"bc"], &env, &mut inputs);

        assert_ne!(key_a, key_b, "different arg splitting must produce different keys");
    }
}
