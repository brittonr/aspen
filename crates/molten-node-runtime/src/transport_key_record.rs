use crate::error::Failure;
use crate::error::Result;
use crate::fabric_crypto_identity::file_adapter::ED25519_SECRET_BYTES;
use crate::fabric_crypto_identity::file_adapter::FIRST_KEY_GENERATION;
use crate::fabric_crypto_identity::file_adapter::KeyRecord;
use crate::fabric_crypto_identity::file_adapter::encode_key_record;

pub(crate) fn generate() -> Vec<u8> {
    encode_key_record(&KeyRecord {
        generation: FIRST_KEY_GENERATION,
        secret_key: iroh::SecretKey::generate(),
    })
    .to_vec()
}

pub(crate) fn from_secret_hex(secret: &str) -> Result<Vec<u8>> {
    let secret_bytes = decode_secret_hex(secret)?;
    Ok(encode_key_record(&KeyRecord {
        generation: FIRST_KEY_GENERATION,
        secret_key: iroh::SecretKey::from_bytes(&secret_bytes),
    })
    .to_vec())
}

fn decode_secret_hex(secret: &str) -> Result<[u8; ED25519_SECRET_BYTES]> {
    const HEX_CHARS_PER_BYTE: usize = 2;
    const HEX_RADIX: u32 = 16;
    const EXPECTED_HEX_CHARS: usize = ED25519_SECRET_BYTES * HEX_CHARS_PER_BYTE;
    let secret = secret.trim();
    if secret.len() != EXPECTED_HEX_CHARS {
        return Err(Failure::invalid_harness(format!(
            "explicit Ed25519 secret must contain exactly {EXPECTED_HEX_CHARS} lowercase hexadecimal characters"
        )));
    }
    let mut bytes = [0u8; ED25519_SECRET_BYTES];
    for (index, slot) in bytes.iter_mut().enumerate() {
        let offset = index
            .checked_mul(HEX_CHARS_PER_BYTE)
            .ok_or_else(|| Failure::invalid_harness("secret hex offset overflow"))?;
        let pair = &secret[offset..offset + HEX_CHARS_PER_BYTE];
        if !pair.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f')) {
            return Err(Failure::invalid_harness("explicit Ed25519 secret must use lowercase hexadecimal characters"));
        }
        *slot = u8::from_str_radix(pair, HEX_RADIX)
            .map_err(|_| Failure::invalid_harness("explicit Ed25519 secret contains malformed hex"))?;
    }
    Ok(bytes)
}
