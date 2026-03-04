//! SOPS file decryption using Aspen Transit (with age fallback).
//!
//! Decrypts a SOPS-encrypted file (TOML, JSON, or YAML):
//! 1. Extract SOPS metadata
//! 2. Decrypt data key via Transit (or age fallback)
//! 3. Verify MAC
//! 4. Decrypt all values
//! 5. Remove [sops] section from output

use std::path::PathBuf;

use tracing::debug;
use tracing::info;
use tracing::warn;
use zeroize::Zeroizing;

use super::client::TransitClient;
use super::format;
use super::mac::verify_mac;
use super::metadata::SopsFileMetadata;
use super::sops_constants::MAX_SOPS_FILE_SIZE;
use super::sops_error::Result;
use super::sops_error::SopsError;

/// Configuration for decrypting a file.
#[derive(Debug, Clone)]
pub struct DecryptConfig {
    /// Path to the SOPS-encrypted input file.
    pub input_path: PathBuf,
    /// Aspen cluster ticket (optional — falls back to metadata ticket).
    pub cluster_ticket: Option<String>,
    /// Output file path (default: stdout).
    pub output_path: Option<PathBuf>,
    /// Extract a single value by dotted path.
    pub extract_path: Option<String>,
    /// Path to age identity file for fallback decryption.
    pub age_identity: Option<PathBuf>,
}

impl Default for DecryptConfig {
    fn default() -> Self {
        Self {
            input_path: PathBuf::new(),
            cluster_ticket: None,
            output_path: None,
            extract_path: None,
            age_identity: None,
        }
    }
}

/// Decrypt a SOPS-encrypted file.
///
/// Returns the decrypted file contents as a string.
pub async fn decrypt_file(config: &DecryptConfig) -> Result<String> {
    // Detect format
    let fmt = format::detect_format(&config.input_path)?;

    // Read file
    let contents = tokio::fs::read_to_string(&config.input_path).await.map_err(|e| SopsError::FileRead {
        path: config.input_path.clone(),
        source: e,
    })?;

    if contents.len() > MAX_SOPS_FILE_SIZE {
        return Err(SopsError::FileTooLarge {
            path: config.input_path.clone(),
            size_bytes: contents.len() as u64,
            max_bytes: MAX_SOPS_FILE_SIZE as u64,
        });
    }

    // Extract metadata (format-agnostic)
    let metadata = format::extract_metadata(fmt, &contents, &config.input_path)?.ok_or(SopsError::InvalidMetadata {
        reason: "no sops section found — file may not be encrypted".into(),
    })?;

    // Decrypt the data key
    let data_key = decrypt_data_key(config, &metadata).await?;
    let data_key_array = to_key_array(&data_key)?;

    // Decrypt all values and remove metadata (format-agnostic)
    let (output, values) = format::decrypt_document(fmt, &contents, &data_key_array, &config.input_path)?;

    // Verify MAC
    if !metadata.mac.is_empty() {
        verify_mac(&metadata.mac, &data_key_array, &values)?;
        debug!("MAC verification passed");
    } else {
        warn!("No MAC found in SOPS metadata — skipping verification");
    }

    // Data key zeroized on drop
    drop(data_key);

    // Handle --extract
    if let Some(ref extract_path) = config.extract_path {
        return format::extract_value(fmt, &output, extract_path);
    }

    // Write to file if requested
    if let Some(ref output_path) = config.output_path {
        tokio::fs::write(output_path, output.as_bytes()).await.map_err(|e| SopsError::FileWrite {
            path: output_path.clone(),
            source: e,
        })?;
        info!(path = %output_path.display(), format = ?fmt, "Decrypted file written");
    }

    Ok(output)
}

/// Decrypt the data key using available key groups.
///
/// Tries Aspen Transit first, then falls back to age if available.
async fn decrypt_data_key(config: &DecryptConfig, metadata: &SopsFileMetadata) -> Result<Zeroizing<Vec<u8>>> {
    // Try Aspen Transit recipients
    if metadata.has_aspen_transit() {
        for recipient in &metadata.aspen_transit {
            let ticket = config.cluster_ticket.as_deref().unwrap_or(&recipient.cluster_ticket);

            match try_transit_decrypt(ticket, &recipient.mount, &recipient.name, &recipient.enc).await {
                Ok(key) => {
                    debug!(key = recipient.name, version = recipient.key_version, "Decrypted data key via Transit");
                    return Ok(key);
                }
                Err(e) => {
                    warn!(
                        key = recipient.name,
                        error = %e,
                        "Transit decrypt failed, trying next key group"
                    );
                    continue;
                }
            }
        }
    }

    // Try age fallback
    if metadata.has_age()
        && let Some(ref identity_path) = config.age_identity
    {
        match try_age_decrypt(identity_path, metadata).await {
            Ok(key) => {
                debug!("Decrypted data key via age fallback");
                return Ok(key);
            }
            Err(e) => {
                warn!(error = %e, "Age fallback decrypt failed");
            }
        }
    }

    Err(SopsError::NoMatchingKeyGroup)
}

/// Try to decrypt the data key using Transit.
async fn try_transit_decrypt(
    cluster_ticket: &str,
    mount: &str,
    key_name: &str,
    encrypted_data_key: &str,
) -> Result<Zeroizing<Vec<u8>>> {
    let client = TransitClient::connect(cluster_ticket, Some(mount)).await?;
    client.decrypt_data_key(key_name, encrypted_data_key).await
}

/// Try to decrypt the data key using age.
async fn try_age_decrypt(identity_path: &std::path::Path, metadata: &SopsFileMetadata) -> Result<Zeroizing<Vec<u8>>> {
    let identity_contents = tokio::fs::read_to_string(identity_path).await.map_err(|e| SopsError::AgeError {
        reason: format!("failed to read age identity: {e}"),
    })?;

    let identity = parse_age_identity(&identity_contents)?;

    for recipient in &metadata.age {
        let Some(ref enc) = recipient.enc else {
            continue;
        };

        match decrypt_age_ciphertext(enc, &identity) {
            Ok(key) => return Ok(Zeroizing::new(key)),
            Err(e) => {
                warn!(recipient = %recipient.recipient, error = %e, "Age recipient failed");
                continue;
            }
        }
    }

    Err(SopsError::NoMatchingKeyGroup)
}

/// Parse an age identity from file contents.
fn parse_age_identity(contents: &str) -> Result<age::x25519::Identity> {
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("AGE-SECRET-KEY-1") {
            return line.parse::<age::x25519::Identity>().map_err(|e| SopsError::AgeError {
                reason: format!("invalid age identity: {e}"),
            });
        }
    }
    Err(SopsError::AgeError {
        reason: "no AGE-SECRET-KEY-1 found in identity".into(),
    })
}

/// Decrypt age armored ciphertext.
fn decrypt_age_ciphertext(ciphertext: &str, identity: &age::x25519::Identity) -> Result<Vec<u8>> {
    use std::io::Read;

    let decryptor =
        age::Decryptor::new_buffered(age::armor::ArmoredReader::new(ciphertext.as_bytes())).map_err(|e| {
            SopsError::AgeError {
                reason: format!("failed to parse age ciphertext: {e}"),
            }
        })?;

    if decryptor.is_scrypt() {
        return Err(SopsError::AgeError {
            reason: "passphrase-encrypted files not supported".into(),
        });
    }

    let mut reader =
        decryptor
            .decrypt(std::iter::once(identity as &dyn age::Identity))
            .map_err(|e| SopsError::AgeError {
                reason: format!("age decryption failed: {e}"),
            })?;

    let mut plaintext = Vec::new();
    reader.read_to_end(&mut plaintext).map_err(|e| SopsError::AgeError {
        reason: format!("failed to read decrypted age data: {e}"),
    })?;

    Ok(plaintext)
}

/// Convert a data key Vec to a fixed-size array.
fn to_key_array(key: &[u8]) -> Result<[u8; 32]> {
    if key.len() != 32 {
        return Err(SopsError::TransitDecrypt {
            key_name: String::new(),
            reason: format!("data key has wrong length: {} (expected 32)", key.len()),
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(key);
    Ok(arr)
}
