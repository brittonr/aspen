use std::collections::HashMap;

use aspen_secrets_core::kv::KvConfig;
use aspen_secrets_core::kv::SecretData;
use aspen_secrets_core::kv::SecretMetadata;
use aspen_secrets_core::kv::VersionMetadata;
use aspen_secrets_core::pki::CertificateAuthority;
use aspen_secrets_core::pki::CrlEntry;
use aspen_secrets_core::pki::CrlState;
use aspen_secrets_core::pki::PendingIntermediateCa;
use aspen_secrets_core::pki::PkiConfig;
use aspen_secrets_core::pki::PkiKeyType;
use aspen_secrets_core::pki::PkiRole;
use aspen_secrets_core::transit::KeyType;
use aspen_secrets_core::transit::KeyVersion;
use aspen_secrets_core::transit::TransitKey;
use serde_json::json;

#[test]
fn kv_state_json_contracts_are_stable() {
    let secret_data = SecretData::new(HashMap::from([("api_key".to_string(), "[REDACTED]".to_string())]));
    assert_eq!(
        serde_json::to_value(&secret_data).expect("serialize secret data"),
        json!({
            "data": { "api_key": "[REDACTED]" }
        })
    );
    let secret_data_roundtrip: SecretData = serde_json::from_value(json!({
        "data": { "api_key": "[REDACTED]" }
    }))
    .expect("deserialize secret data");
    assert_eq!(secret_data_roundtrip.get("api_key"), Some("[REDACTED]"));

    let mut version = VersionMetadata::new(2, 1_700_000_000_000);
    version.deletion_time_unix_ms = Some(1_700_000_001_000);
    version.destroyed = true;
    assert_eq!(
        serde_json::to_value(&version).expect("serialize version metadata"),
        json!({
            "version": 2,
            "created_time_unix_ms": 1700000000000_u64,
            "deletion_time_unix_ms": 1700000001000_u64,
            "destroyed": true
        })
    );

    let mut metadata = SecretMetadata::new(1_700_000_000_000);
    metadata.max_versions = 5;
    metadata.cas_required = true;
    metadata.delete_version_after_secs = 60;
    metadata.custom_metadata.insert("owner".to_string(), "platform".to_string());
    metadata.next_version(1_700_000_000_500);
    assert_eq!(
        serde_json::to_value(&metadata).expect("serialize secret metadata"),
        json!({
            "current_version": 1,
            "created_time_unix_ms": 1700000000000_u64,
            "updated_time_unix_ms": 1700000000500_u64,
            "max_versions": 5,
            "cas_required": true,
            "delete_version_after_secs": 60,
            "custom_metadata": { "owner": "platform" },
            "versions": {
                "1": {
                    "version": 1,
                    "created_time_unix_ms": 1700000000500_u64,
                    "deletion_time_unix_ms": null,
                    "destroyed": false
                }
            }
        })
    );

    assert_eq!(
        serde_json::to_value(KvConfig::default()).expect("serialize kv config"),
        json!({
            "max_versions": aspen_secrets_core::DEFAULT_MAX_VERSIONS,
            "cas_required": false,
            "delete_version_after_secs": 0
        })
    );
}

#[test]
fn transit_state_json_contracts_are_stable() {
    assert_eq!(serde_json::to_value(KeyType::Aes256Gcm).expect("serialize aes"), json!("Aes256Gcm"));
    assert_eq!(
        serde_json::to_value(KeyType::XChaCha20Poly1305).expect("serialize xchacha"),
        json!("XChaCha20Poly1305")
    );
    assert_eq!(serde_json::to_value(KeyType::Ed25519).expect("serialize ed25519"), json!("Ed25519"));

    let version = KeyVersion {
        version: 1,
        created_time_unix_ms: 1_700_000_000_000,
        key_material: vec![1, 2, 3, 4],
        public_key: Some(vec![5, 6, 7, 8]),
    };
    assert_eq!(
        serde_json::to_value(&version).expect("serialize key version"),
        json!({
            "version": 1,
            "created_time_unix_ms": 1700000000000_u64,
            "key_material": [1, 2, 3, 4],
            "public_key": [5, 6, 7, 8]
        })
    );

    let key = TransitKey::new(
        "signing".to_string(),
        KeyType::Ed25519,
        1_700_000_000_000,
        vec![9, 10, 11, 12],
        Some(vec![13, 14, 15, 16]),
    );
    assert_eq!(
        serde_json::to_value(&key).expect("serialize transit key"),
        json!({
            "name": "signing",
            "key_type": "Ed25519",
            "current_version": 1,
            "min_decryption_version": 1,
            "min_encryption_version": 0,
            "deletion_allowed": false,
            "exportable": false,
            "allow_plaintext_backup": false,
            "created_time_unix_ms": 1700000000000_u64,
            "latest_version_time_unix_ms": 1700000000000_u64,
            "supports_convergent_encryption": false,
            "versions": {
                "1": {
                    "version": 1,
                    "created_time_unix_ms": 1700000000000_u64,
                    "key_material": [9, 10, 11, 12],
                    "public_key": [13, 14, 15, 16]
                }
            }
        })
    );
}

#[test]
fn pki_state_json_contracts_are_stable() {
    assert_eq!(serde_json::to_value(PkiKeyType::Rsa2048).expect("serialize rsa2048"), json!("Rsa2048"));
    assert_eq!(serde_json::to_value(PkiKeyType::EcdsaP256).expect("serialize p256"), json!("EcdsaP256"));
    assert_eq!(serde_json::to_value(PkiKeyType::Ed25519).expect("serialize ed25519"), json!("Ed25519"));

    let ca = CertificateAuthority {
        certificate: "-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----".to_string(),
        private_key: vec![1, 2, 3],
        key_type: PkiKeyType::EcdsaP256,
        next_serial: 42,
        created_time_unix_ms: 1_700_000_000_000,
        expiry_time_unix_ms: 1_800_000_000_000,
        is_root: true,
        ca_chain: vec!["root".to_string()],
        common_name: "Aspen Fixture Root".to_string(),
        organization: Some("Aspen".to_string()),
        ou: Some("Platform".to_string()),
        country: Some("US".to_string()),
        province: Some("CA".to_string()),
        locality: Some("San Francisco".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&ca).expect("serialize ca"),
        json!({
            "certificate": "-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----",
            "private_key": [1, 2, 3],
            "key_type": "EcdsaP256",
            "next_serial": 42,
            "created_time_unix_ms": 1700000000000_u64,
            "expiry_time_unix_ms": 1800000000000_u64,
            "is_root": true,
            "ca_chain": ["root"],
            "common_name": "Aspen Fixture Root",
            "organization": "Aspen",
            "ou": "Platform",
            "country": "US",
            "province": "CA",
            "locality": "San Francisco"
        })
    );

    let mut role = PkiRole::default();
    role.name = "web".to_string();
    role.allowed_domains = vec!["example.test".to_string()];
    role.allow_subdomains = true;
    role.created_time_unix_ms = 1_700_000_000_000;
    assert_eq!(
        serde_json::to_value(&role).expect("serialize role"),
        json!({
            "name": "web",
            "allowed_domains": ["example.test"],
            "allow_subdomains": true,
            "allow_bare_domains": false,
            "allow_localhost": false,
            "allow_ip_sans": false,
            "allow_wildcard_certificates": false,
            "allowed_uri_sans": [],
            "key_type": "Rsa2048",
            "max_ttl_secs": aspen_secrets_core::DEFAULT_CERT_TTL_SECS,
            "ttl_secs": aspen_secrets_core::DEFAULT_CERT_TTL_SECS,
            "generate_key": true,
            "key_usages": ["DigitalSignature", "KeyEncipherment"],
            "ext_key_usages": ["ServerAuth", "ClientAuth"],
            "require_cn": true,
            "organization": [],
            "ou": [],
            "country": [],
            "province": [],
            "locality": [],
            "no_store": false,
            "created_time_unix_ms": 1700000000000_u64
        })
    );

    let crl = CrlState {
        entries: vec![CrlEntry {
            serial: "2a".to_string(),
            revocation_time_unix_ms: 1_700_000_010_000,
            reason: Some("cessation_of_operation".to_string()),
        }],
        last_update_unix_ms: 1_700_000_010_000,
        next_update_unix_ms: 1_700_086_410_000,
    };
    assert_eq!(
        serde_json::to_value(&crl).expect("serialize crl"),
        json!({
            "entries": [{
                "serial": "2a",
                "revocation_time_unix_ms": 1700000010000_u64,
                "reason": "cessation_of_operation"
            }],
            "last_update_unix_ms": 1700000010000_u64,
            "next_update_unix_ms": 1700086410000_u64
        })
    );

    assert_eq!(
        serde_json::to_value(PkiConfig::default()).expect("serialize pki config"),
        json!({
            "default_ttl_secs": 0,
            "max_ttl_secs": 0,
            "crl_distribution_points": [],
            "ocsp_servers": [],
            "issuing_certificates": []
        })
    );

    let pending = PendingIntermediateCa {
        private_key: vec![8, 9, 10],
        key_type: PkiKeyType::Rsa3072,
        common_name: "Aspen Fixture Intermediate".to_string(),
        organization: Some("Aspen".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&pending).expect("serialize pending intermediate"),
        json!({
            "private_key": [8, 9, 10],
            "key_type": "Rsa3072",
            "common_name": "Aspen Fixture Intermediate",
            "organization": "Aspen"
        })
    );
}
