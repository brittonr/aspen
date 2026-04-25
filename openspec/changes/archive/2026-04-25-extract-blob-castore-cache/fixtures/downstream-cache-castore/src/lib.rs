#[cfg(test)]
mod tests {
    use aspen_cache::CacheSigningKey;
    use aspen_cache::CacheVerifyingKey;
    use aspen_cache::parse_store_path;
    use aspen_castore::CASTORE_ALPN;

    const CACHE_NAME: &str = "fixture-cache";
    const STORE_PATH: &str = "/nix/store/w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1";
    const STORE_HASH: &str = "w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb";
    const STORE_NAME: &str = "hello-2.12.1";
    const FINGERPRINT: &str = "1;/nix/store/w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1;sha256:deadbeef;1024;";
    const EXPECTED_ALPN: &[u8] = b"aspen-castore/0";

    #[test]
    fn cache_signing_roundtrip_is_reusable() {
        let key = CacheSigningKey::generate(CACHE_NAME).expect("generate fixture cache key");
        let signature = key.sign_fingerprint(FINGERPRINT);
        let verifier = CacheVerifyingKey::from_nix_format(&key.to_nix_public_key()).expect("parse fixture public key");

        assert!(verifier.verify_signature(FINGERPRINT, &signature).expect("verify fixture signature"));
    }

    #[test]
    fn malformed_cache_name_is_rejected() {
        let bad_cache_name = "bad:name";

        assert!(CacheSigningKey::generate(bad_cache_name).is_err());
    }

    #[test]
    fn narinfo_store_path_parsing_is_reusable() {
        let (hash, name) = parse_store_path(STORE_PATH).expect("parse fixture store path");

        assert_eq!(hash, STORE_HASH);
        assert_eq!(name, STORE_NAME);
    }

    #[test]
    fn castore_adapter_alpn_is_available_without_app_shells() {
        assert_eq!(CASTORE_ALPN, EXPECTED_ALPN);
    }
}
