use aspen_cache::CacheEntry;
use aspen_castore::CASTORE_ALPN;

pub fn portable_cache_entry() -> CacheEntry {
    CacheEntry::new(
        "/nix/store/00000000000000000000000000000000-fixture".to_string(),
        "00000000000000000000000000000000".to_string(),
        "fixture-blob".to_string(),
        64,
        "sha256:0000000000000000000000000000000000000000000000000000".to_string(),
        1,
        2,
    )
}

pub fn castore_alpn_is_stable() -> bool {
    CASTORE_ALPN == b"aspen-castore/0"
}

#[cfg(test)]
mod tests {
    use aspen_cache::CacheSigningKey;
    use aspen_cache::CacheVerifyingKey;
    use aspen_cache::MAX_REFERENCES;
    use aspen_cache::parse_store_path;

    use super::*;

    #[test]
    fn downstream_can_use_cache_metadata_without_kv_adapter() {
        let entry = portable_cache_entry();
        let roundtrip = CacheEntry::from_bytes(&entry.to_bytes().unwrap()).unwrap();
        assert_eq!(roundtrip.kv_key(), "_cache:narinfo:00000000000000000000000000000000");
        assert!(roundtrip.to_narinfo(None).contains("StorePath:"));
        assert!(MAX_REFERENCES >= 1000);
    }

    #[test]
    fn downstream_can_use_signing_and_castore_adapter_constants() {
        let signing = CacheSigningKey::generate("fixture-cache").unwrap();
        let public = signing.to_nix_public_key();
        let verifying = CacheVerifyingKey::from_nix_format(&public).unwrap();
        let signature = signing.sign_fingerprint("fixture-fingerprint");
        assert!(verifying.verify_signature("fixture-fingerprint", &signature).unwrap());
        assert!(castore_alpn_is_stable());
    }

    #[test]
    fn downstream_can_parse_store_paths() {
        let (hash, name) = parse_store_path("/nix/store/00000000000000000000000000000000-fixture").unwrap();
        assert_eq!(hash, "00000000000000000000000000000000");
        assert_eq!(name, "fixture");
    }
}
