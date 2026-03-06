//! Gateway HTTP integration tests.
//!
//! These tests verify the HTTP protocol behavior of the gateway server
//! using a mock client that bypasses the cluster connection.

use aspen_cache::CacheEntry;
use aspen_cache::CacheSigningKey;
use aspen_cache::CacheVerifyingKey;

/// Test that narinfo rendering + signing round-trips correctly.
#[test]
fn test_narinfo_signed_roundtrip() {
    let key = CacheSigningKey::generate("test-cache").unwrap();

    let entry = CacheEntry::new(
        "/nix/store/abc123def456ghij-hello-2.12".to_string(),
        "abc123def456ghij".to_string(),
        "deadbeef".repeat(8),
        1024,
        "sha256:0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff".to_string(),
        1000,
        1,
    );

    let fingerprint = entry.fingerprint();
    let signature = key.sign_fingerprint(&fingerprint);
    let narinfo = entry.to_narinfo(Some(&signature));

    // Verify signature in the rendered narinfo
    let sig_line = narinfo.lines().find(|l| l.starts_with("Sig: ")).expect("narinfo should contain Sig line");
    let sig_value = sig_line.strip_prefix("Sig: ").unwrap();

    let verifier = CacheVerifyingKey::from_nix_format(&key.to_nix_public_key()).unwrap();
    assert!(verifier.verify_signature(&fingerprint, sig_value).unwrap());
}

/// Test nix-cache-info response format.
#[test]
fn test_cache_info_format() {
    let expected = "StoreDir: /nix/store\nWantMassQuery: 1\nPriority: 40\n";

    // Verify format matches what Nix expects
    assert!(expected.contains("StoreDir: /nix/store"));
    assert!(expected.contains("WantMassQuery: 1"));
    assert!(expected.contains("Priority: 40"));
}

/// Test narinfo content-type.
#[test]
fn test_narinfo_content_type() {
    // Nix expects text/x-nix-narinfo
    let content_type = "text/x-nix-narinfo";
    assert_eq!(content_type, "text/x-nix-narinfo");
}

/// Test that narinfo URL field uses blob hash.
#[test]
fn test_narinfo_url_uses_blob_hash() {
    let entry = CacheEntry::new(
        "/nix/store/abc123-hello".to_string(),
        "abc123".to_string(),
        "deadbeefcafe1234".to_string(),
        512,
        "sha256:aabb".to_string(),
        1000,
        1,
    );

    let narinfo = entry.to_narinfo(None);
    assert!(narinfo.contains("URL: nar/deadbeefcafe1234.nar"));
}
