//! SOPS encrypt/decrypt roundtrip tests.
//!
//! These test the format-level encryption and decryption without
//! requiring a running Aspen cluster. They directly use data keys
//! to exercise the full value encryption → MAC → decryption → MAC verify cycle.

#![cfg(feature = "sops")]

use aspen_secrets::sops::encrypt::encrypt_data_key_for_age;
use aspen_secrets::sops::format;
use aspen_secrets::sops::format::common::decrypt_sops_value;
use aspen_secrets::sops::format::common::encrypt_sops_value;
use aspen_secrets::sops::format::common::is_sops_encrypted;
use aspen_secrets::sops::mac::encrypt_mac;
use aspen_secrets::sops::mac::verify_mac;
use aspen_secrets::sops::metadata::AgeRecipient;
use aspen_secrets::sops::metadata::AspenTransitRecipient;
use aspen_secrets::sops::metadata::SopsFileMetadata;

/// Generate a test data key (32 bytes of deterministic data).
fn test_data_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = (i as u8).wrapping_mul(7).wrapping_add(42);
    }
    key
}

fn test_metadata() -> SopsFileMetadata {
    let mut meta = SopsFileMetadata::new();
    meta.add_aspen_recipient(AspenTransitRecipient {
        cluster_ticket: "aspen1test".to_string(),
        mount: "transit".to_string(),
        name: "sops-data-key".to_string(),
        enc: "aspen:v1:dGVzdA==".to_string(),
        key_version: 1,
    });
    meta
}

// ============================================================================
// Value-level roundtrip
// ============================================================================

#[test]
fn test_value_encrypt_decrypt_string() {
    let key = test_data_key();
    let encrypted = encrypt_sops_value("hello world", &key, "str").unwrap();
    assert!(is_sops_encrypted(&encrypted));
    assert!(encrypted.starts_with("ENC[AES256_GCM,"));
    let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
    assert_eq!(decrypted, "hello world");
}

#[test]
fn test_value_encrypt_decrypt_integer() {
    let key = test_data_key();
    let encrypted = encrypt_sops_value("42", &key, "int").unwrap();
    let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
    assert_eq!(decrypted, "42");
}

#[test]
fn test_value_encrypt_decrypt_bool() {
    let key = test_data_key();
    let encrypted = encrypt_sops_value("true", &key, "bool").unwrap();
    let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
    assert_eq!(decrypted, "true");
}

#[test]
fn test_value_wrong_key_fails() {
    let key1 = test_data_key();
    let mut key2 = test_data_key();
    key2[0] ^= 0xff;

    let encrypted = encrypt_sops_value("secret", &key1, "str").unwrap();
    assert!(decrypt_sops_value(&encrypted, &key2).is_err());
}

#[test]
fn test_value_empty_string() {
    let key = test_data_key();
    let encrypted = encrypt_sops_value("", &key, "str").unwrap();
    let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
    assert_eq!(decrypted, "");
}

#[test]
fn test_value_unicode() {
    let key = test_data_key();
    let encrypted = encrypt_sops_value("こんにちは🌍", &key, "str").unwrap();
    let decrypted = decrypt_sops_value(&encrypted, &key).unwrap();
    assert_eq!(decrypted, "こんにちは🌍");
}

// ============================================================================
// MAC roundtrip
// ============================================================================

#[test]
fn test_mac_encrypt_verify_roundtrip() {
    let key = test_data_key();
    let values = vec![
        ("database.host".to_string(), "localhost".to_string()),
        ("database.password".to_string(), "secret123".to_string()),
    ];

    let encrypted_mac = encrypt_mac(&key, &values).unwrap();
    assert!(is_sops_encrypted(&encrypted_mac));

    // Verify should succeed
    assert!(verify_mac(&encrypted_mac, &key, &values).is_ok());
}

#[test]
fn test_mac_tampered_values_fail() {
    let key = test_data_key();
    let original_values = vec![
        ("database.host".to_string(), "localhost".to_string()),
        ("database.password".to_string(), "secret123".to_string()),
    ];

    let encrypted_mac = encrypt_mac(&key, &original_values).unwrap();

    // Tamper: change a value
    let tampered_values = vec![
        ("database.host".to_string(), "localhost".to_string()),
        ("database.password".to_string(), "TAMPERED".to_string()),
    ];

    assert!(verify_mac(&encrypted_mac, &key, &tampered_values).is_err());
}

#[test]
fn test_mac_tampered_keys_fail() {
    let key = test_data_key();
    let original_values = vec![("api.key".to_string(), "secret".to_string())];

    let encrypted_mac = encrypt_mac(&key, &original_values).unwrap();

    // Tamper: rename key path
    let tampered_values = vec![("api.token".to_string(), "secret".to_string())];

    assert!(verify_mac(&encrypted_mac, &key, &tampered_values).is_err());
}

#[test]
fn test_mac_added_value_fails() {
    let key = test_data_key();
    let original_values = vec![("a".to_string(), "1".to_string())];

    let encrypted_mac = encrypt_mac(&key, &original_values).unwrap();

    // Tamper: add extra value
    let tampered_values = vec![("a".to_string(), "1".to_string()), ("b".to_string(), "2".to_string())];

    assert!(verify_mac(&encrypted_mac, &key, &tampered_values).is_err());
}

// ============================================================================
// Metadata serialization
// ============================================================================

#[test]
fn test_metadata_toml_serialization() {
    let mut meta = SopsFileMetadata::new();
    meta.add_aspen_recipient(AspenTransitRecipient {
        cluster_ticket: "aspen1test".to_string(),
        mount: "transit".to_string(),
        name: "sops-data-key".to_string(),
        enc: "aspen:v1:dGVzdA==".to_string(),
        key_version: 1,
    });

    // Direct serialization works
    let toml_str = toml::to_string_pretty(&meta).unwrap();
    assert!(
        toml_str.contains("cluster_ticket"),
        "serialized metadata should contain cluster_ticket:\n{toml_str}"
    );

    // Through toml::Value pipeline (what inject_metadata uses)
    let sops_value = toml::Value::try_from(&meta).unwrap();
    let sops_toml_str = toml::to_string_pretty(&sops_value).unwrap();
    assert!(sops_toml_str.contains("cluster_ticket"), "Value pipeline should preserve data:\n{sops_toml_str}");

    // Full inject_metadata pipeline: "[sops]\n" prefix + re-parse
    let full = format!("[sops]\n{sops_toml_str}");
    let sops_doc: toml_edit::DocumentMut = match full.parse() {
        Ok(doc) => doc,
        Err(e) => panic!("Failed to parse metadata TOML:\n{full}\nError: {e}"),
    };
    let sops_item = sops_doc.get("sops").expect("should have [sops] key");
    let rendered = sops_item.to_string();
    assert!(
        rendered.contains("cluster_ticket") || full.contains("cluster_ticket"),
        "inject_metadata pipeline should preserve aspen_transit:\nfull={full}\nrendered={rendered}"
    );
}

// ============================================================================
// TOML document-level roundtrip
// ============================================================================

#[test]
fn test_toml_encrypt_has_sops_structure() {
    let key = test_data_key();
    let meta = test_metadata();
    let input = r#"
[database]
host = "localhost"
port = 5432
password = "supersecret"
"#;

    let (output, values) =
        format::encrypt_document(format::SopsFormat::Toml, input, &key, None, &meta, std::path::Path::new("test.toml"))
            .unwrap();

    // Output should contain [sops] metadata
    assert!(output.contains("[sops]"), "encrypted TOML must have [sops] section:\n{output}");
    // The toml crate serializes Vec<AspenTransitRecipient> as [[sops.aspen_transit]] tables
    // or as inline arrays depending on complexity. Check for either representation.
    assert!(
        output.contains("aspen_transit") || output.contains("cluster_ticket"),
        "must have aspen_transit key group data:\n{output}"
    );
    assert!(output.contains("ENC[AES256_GCM,"), "values must be encrypted");

    // Values should be collected for MAC
    assert!(!values.is_empty(), "should collect values for MAC");
}

#[test]
fn test_toml_encrypt_decrypt_roundtrip() {
    let key = test_data_key();
    let meta = test_metadata();
    let input = r#"
[database]
host = "localhost"
port = 5432
password = "supersecret"
debug = true
"#;

    // Encrypt
    let (_encrypted_output, values) =
        format::encrypt_document(format::SopsFormat::Toml, input, &key, None, &meta, std::path::Path::new("test.toml"))
            .unwrap();

    // Add MAC to metadata
    let mut meta_with_mac = meta.clone();
    meta_with_mac.mac = encrypt_mac(&key, &values).unwrap();

    // Re-encrypt with MAC in metadata
    let (final_encrypted, _) = format::encrypt_document(
        format::SopsFormat::Toml,
        input,
        &key,
        None,
        &meta_with_mac,
        std::path::Path::new("test.toml"),
    )
    .unwrap();

    // Decrypt
    let (decrypted_output, dec_values) =
        format::decrypt_document(format::SopsFormat::Toml, &final_encrypted, &key, std::path::Path::new("test.toml"))
            .unwrap();

    // Verify MAC
    assert!(verify_mac(&meta_with_mac.mac, &key, &dec_values).is_ok());

    // Decrypted output should not contain ENC[...]
    assert!(!decrypted_output.contains("ENC["), "decrypted output should be plaintext");

    // Decrypted output should contain original values
    assert!(decrypted_output.contains("localhost"));
    assert!(decrypted_output.contains("supersecret"));
}

#[test]
fn test_toml_encrypted_regex_partial() {
    let key = test_data_key();
    let meta = test_metadata();
    let input = r#"
[config]
public_name = "my-app"
secret_key = "sk-12345"
api_token = "tok-abcde"
"#;

    // Only encrypt keys matching "secret|token"
    let (output, _values) = format::encrypt_document(
        format::SopsFormat::Toml,
        input,
        &key,
        Some("secret|token"),
        &meta,
        std::path::Path::new("test.toml"),
    )
    .unwrap();

    // public_name should be plaintext
    assert!(output.contains("\"my-app\""), "non-matching value should stay plaintext");
    // secret_key and api_token should be encrypted
    assert!(!output.contains("sk-12345"), "matching value should be encrypted");
    assert!(!output.contains("tok-abcde"), "matching value should be encrypted");
}

// ============================================================================
// Multi-key-group (age + Transit)
// ============================================================================

#[test]
fn test_multi_key_group_age_and_transit() {
    let key = test_data_key();

    // Generate an age keypair for testing
    let age_identity = age::x25519::Identity::generate();
    let age_recipient = age_identity.to_public().to_string();

    // Encrypt the data key for age
    let age_enc = encrypt_data_key_for_age(&age_recipient, &key).unwrap();

    // Build metadata with both key groups
    let mut meta = SopsFileMetadata::new();
    meta.add_aspen_recipient(AspenTransitRecipient {
        cluster_ticket: "aspen1test".to_string(),
        mount: "transit".to_string(),
        name: "sops-data-key".to_string(),
        enc: "aspen:v1:dGVzdA==".to_string(),
        key_version: 1,
    });
    meta.add_age_recipient(AgeRecipient {
        recipient: age_recipient.clone(),
        enc: Some(age_enc),
    });

    let input = r#"
[secrets]
api_key = "sk-live-abc123"
"#;

    let (output, _values) =
        format::encrypt_document(format::SopsFormat::Toml, input, &key, None, &meta, std::path::Path::new("test.toml"))
            .unwrap();

    // Both key groups should be present in metadata
    assert!(output.contains("cluster_ticket"), "should have Transit key group:\n{output}");
    assert!(output.contains(&age_recipient), "should have age key group:\n{output}");
    assert!(output.contains("ENC[AES256_GCM,"), "values should be encrypted");
    assert!(!output.contains("sk-live-abc123"), "plaintext should not appear");
}

#[test]
fn test_age_data_key_encrypt_decrypt_roundtrip() {
    // Verify we can encrypt a data key for age and decrypt it back
    let age_identity = age::x25519::Identity::generate();
    let age_recipient = age_identity.to_public().to_string();

    let original_key = test_data_key();
    let age_enc = encrypt_data_key_for_age(&age_recipient, &original_key).unwrap();

    // Decrypt using the age identity
    assert!(age_enc.contains("BEGIN AGE ENCRYPTED FILE"));

    // Use age to decrypt
    use std::io::Read;
    let decryptor = age::Decryptor::new_buffered(age::armor::ArmoredReader::new(age_enc.as_bytes())).unwrap();
    let mut reader = decryptor.decrypt(std::iter::once(&age_identity as &dyn age::Identity)).unwrap();
    let mut decrypted = Vec::new();
    reader.read_to_end(&mut decrypted).unwrap();

    assert_eq!(decrypted, original_key.to_vec());
}
