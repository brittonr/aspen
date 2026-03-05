# aspen-sops — Design

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    User / CI Pipeline                     │
│                                                          │
│   ┌─────────────────┐        ┌─────────────────────┐    │
│   │  aspen-sops CLI │        │  Go sops binary      │    │
│   │  (native mode)  │        │  (bridge mode)       │    │
│   └────────┬────────┘        └──────────┬──────────┘    │
│            │                            │               │
│     Iroh QUIC                   gRPC (Unix socket)      │
│            │                            │               │
│            │                 ┌──────────▼──────────┐    │
│            │                 │ aspen-sops keyservice│    │
│            │                 │ (gRPC ↔ Iroh bridge) │    │
│            │                 └──────────┬──────────┘    │
│            │                            │               │
│            │                     Iroh QUIC              │
│            │                            │               │
└────────────┼────────────────────────────┼───────────────┘
             │                            │
     ┌───────▼────────────────────────────▼───────┐
     │              Aspen Cluster                  │
     │                                             │
     │   SecretsTransitEncrypt / Decrypt / Datakey │
     │              Transit Engine                 │
     │         (Raft-replicated keys)              │
     └─────────────────────────────────────────────┘
```

Two operational modes:

1. **Native mode**: `aspen-sops` binary handles SOPS encrypt/decrypt directly, calling Aspen Transit via Iroh QUIC. No Go binary needed.
2. **Bridge mode**: `aspen-sops keyservice` runs a local gRPC server. The Go `sops` binary connects via `--keyservice unix://...` flag. The bridge translates gRPC encrypt/decrypt calls to Aspen Transit RPCs.

## Crate Structure

```
crates/aspen-sops/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library: encrypt/decrypt/rotate/mac
│   ├── main.rs             # CLI binary entry point
│   ├── cli.rs              # Clap command definitions
│   ├── client.rs           # Aspen Transit client (wraps aspen-client)
│   ├── encrypt.rs          # SOPS encryption: generate data key → encrypt values → write metadata
│   ├── decrypt.rs          # SOPS decryption: extract metadata → decrypt data key → decrypt values
│   ├── edit.rs             # Decrypt → $EDITOR → re-encrypt workflow
│   ├── rotate.rs           # Re-encrypt data key with latest Transit key version
│   ├── updatekeys.rs       # Add/remove key groups
│   ├── mac.rs              # SOPS MAC computation (HMAC-SHA256 over plaintext values)
│   ├── metadata.rs         # SOPS metadata types: AspenTransitRecipient, parsing/serialization
│   ├── format/
│   │   ├── mod.rs          # Format detection + dispatch
│   │   ├── toml.rs         # TOML tree walk: encrypt/decrypt values, preserve structure
│   │   ├── yaml.rs         # YAML support (v2)
│   │   └── json.rs         # JSON support (v2)
│   ├── keyservice/
│   │   ├── mod.rs          # gRPC key service server
│   │   ├── proto.rs        # Generated protobuf types (SOPS KeyService)
│   │   └── bridge.rs       # gRPC handler → Aspen Transit RPC translation
│   └── verified/
│       ├── mod.rs
│       └── mac.rs          # Pure MAC computation for Verus verification
└── tests/
    ├── encrypt_decrypt_roundtrip.rs
    ├── mac_verification.rs
    ├── key_rotation.rs
    └── sops_compatibility.rs
```

## SOPS File Format

### Encrypted Value Format

SOPS encrypts individual values while keeping keys in plaintext. Each encrypted value has the format:

```
ENC[AES256_GCM,data:<base64-ciphertext>,iv:<base64-iv>,tag:<base64-tag>,type:<original-type>]
```

This is already parsed in `aspen-secrets/src/sops/decryptor.rs`. The encryption side produces the same format.

### SOPS Metadata — Aspen Transit Key Group

A new key group type `aspen_transit` in the `[sops]` metadata table:

```toml
[sops]
lastmodified = "2026-03-03T21:00:00Z"
mac = "ENC[AES256_GCM,data:...,iv:...,tag:...,type:str]"
version = "3.9.0"

[[sops.aspen_transit]]
# Aspen cluster ticket (base32-encoded, contains node addresses + public keys)
cluster_ticket = "aspen1q..."
# Transit mount point (default: "transit")
mount = "transit"
# Transit key name used to encrypt the data key
name = "sops-data-key"
# Encrypted data key (aspen:v<version>:<base64> format, from Transit encrypt)
enc = "aspen:v3:c2VjcmV0IGRhdGEga2V5..."
# Transit key version used (for rotation tracking)
key_version = 3

# Multiple Aspen Transit recipients supported (multi-cluster)
[[sops.aspen_transit]]
cluster_ticket = "aspen1r..."
mount = "transit"
name = "sops-data-key"
enc = "aspen:v1:YW5vdGhlciBjbHVzdGVy..."
key_version = 1

# Age recipients can coexist for offline fallback
[[sops.age]]
recipient = "age1..."
enc = "-----BEGIN AGE ENCRYPTED FILE-----\n..."
```

### Data Key Flow (Envelope Encryption)

**Encrypt**:

1. Generate a random 32-byte data key via `SecretsTransitDatakey` RPC
2. Transit encrypts the data key → returns `aspen:v<N>:<ciphertext>` (stored in `sops.aspen_transit[].enc`)
3. Data key encrypts each value using AES-256-GCM (standard SOPS per-value encryption)
4. Compute MAC over all plaintext values (HMAC-SHA256 with data key)
5. Encrypt the MAC using the data key, store in `sops.mac`
6. **Zeroize** the plaintext data key from memory

**Decrypt**:

1. Read `sops.aspen_transit` metadata
2. Call `SecretsTransitDecrypt` RPC with `enc` ciphertext → get plaintext data key
3. Verify MAC (decrypt `sops.mac`, recompute over all encrypted values' plaintext)
4. Decrypt each `ENC[...]` value using the data key
5. **Zeroize** the data key from memory

**Rotate** (after Transit key rotation):

1. Decrypt data key with old Transit key version
2. Call `SecretsTransitRewrap` RPC → re-encrypt data key with latest version
3. Update `enc` and `key_version` in metadata
4. Values unchanged (same data key, just re-wrapped)

## Transit Client

Wraps `aspen-client` to provide typed Transit operations:

```rust
pub struct TransitClient {
    client: AspenClient,
    mount: String,
}

impl TransitClient {
    /// Connect to an Aspen cluster.
    pub async fn connect(cluster_ticket: &str, mount: &str) -> Result<Self>;

    /// Generate a data key (for SOPS encryption).
    /// Returns (plaintext_data_key, encrypted_data_key, key_version).
    pub async fn generate_data_key(
        &self,
        key_name: &str,
        bits: u32,
    ) -> Result<(Zeroizing<Vec<u8>>, String, u32)>;

    /// Decrypt an encrypted data key.
    pub async fn decrypt_data_key(
        &self,
        key_name: &str,
        ciphertext: &str,
    ) -> Result<Zeroizing<Vec<u8>>>;

    /// Rewrap a data key with the latest Transit key version.
    pub async fn rewrap_data_key(
        &self,
        key_name: &str,
        ciphertext: &str,
    ) -> Result<(String, u32)>;
}
```

Key detail: the plaintext data key is wrapped in `Zeroizing<Vec<u8>>` (from the `zeroize` crate) so it's wiped from memory on drop.

## MAC Computation

SOPS computes an HMAC-SHA256 over all plaintext values to detect tampering of the file structure (adding/removing/reordering keys). The MAC covers:

1. All plaintext values in alphabetical key path order
2. Key paths themselves (prevents key renaming attacks)
3. Does NOT cover the `[sops]` metadata section

```rust
// In verified/mac.rs — pure function, no I/O
pub fn compute_sops_mac(
    values: &[(String, String)],  // (key_path, plaintext_value) sorted
    data_key: &[u8; 32],
) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(data_key).expect("valid key size");
    for (path, value) in values {
        mac.update(path.as_bytes());
        mac.update(value.as_bytes());
    }
    mac.finalize().into_bytes().into()
}
```

This goes in `src/verified/mac.rs` with Verus specs in `verus/mac_spec.rs`.

## gRPC Key Service Bridge

SOPS's external key service protocol (protobuf):

```protobuf
service KeyService {
    rpc Encrypt(EncryptRequest) returns (EncryptResponse);
    rpc Decrypt(DecryptRequest) returns (DecryptResponse);
}

message EncryptRequest {
    Key key = 1;
}

message EncryptResponse {
    Key key = 1;
}

message DecryptRequest {
    Key key = 1;
}

message DecryptResponse {
    Key key = 1;
}

message Key {
    bytes encrypted_key = 1;
    // ... additional metadata fields
}
```

The bridge server:

```rust
pub struct AspenKeyServiceBridge {
    transit_client: TransitClient,
    key_name: String,
}

impl KeyService for AspenKeyServiceBridge {
    async fn encrypt(&self, request: EncryptRequest) -> Result<EncryptResponse> {
        // request.key.encrypted_key = plaintext data key bytes
        let (_, ciphertext, _) = self.transit_client
            .encrypt_data(&self.key_name, &request.key.encrypted_key)
            .await?;
        Ok(EncryptResponse {
            key: Key { encrypted_key: ciphertext.into_bytes(), .. }
        })
    }

    async fn decrypt(&self, request: DecryptRequest) -> Result<DecryptResponse> {
        // request.key.encrypted_key = Transit ciphertext bytes
        let ciphertext_str = String::from_utf8(request.key.encrypted_key)?;
        let plaintext = self.transit_client
            .decrypt_data_key(&self.key_name, &ciphertext_str)
            .await?;
        Ok(DecryptResponse {
            key: Key { encrypted_key: plaintext.to_vec(), .. }
        })
    }
}
```

Launched via:

```bash
aspen-sops keyservice \
    --cluster-ticket aspen1q... \
    --transit-key sops-data-key \
    --socket /tmp/aspen-sops.sock
```

Then used with Go SOPS:

```bash
sops --keyservice unix:///tmp/aspen-sops.sock decrypt secrets.sops.toml
```

## CLI Design

```
aspen-sops encrypt [OPTIONS] <FILE>
    --cluster-ticket <TICKET>    Aspen cluster ticket (or ASPEN_CLUSTER_TICKET env)
    --transit-key <NAME>         Transit key name for data key encryption [default: sops-data-key]
    --transit-mount <MOUNT>      Transit mount point [default: transit]
    --age <RECIPIENT>...         Also encrypt for age recipients (offline fallback)
    --encrypted-regex <REGEX>    Only encrypt values matching regex [default: encrypt all]
    --in-place                   Encrypt file in place (otherwise stdout)

aspen-sops decrypt [OPTIONS] <FILE>
    --cluster-ticket <TICKET>    Aspen cluster ticket (or ASPEN_CLUSTER_TICKET env)
    --output <FILE>              Output to file (default: stdout)
    --extract <PATH>             Extract a single value by dotted path

aspen-sops edit [OPTIONS] <FILE>
    --cluster-ticket <TICKET>    Aspen cluster ticket
    --editor <EDITOR>            Editor command [default: $EDITOR or vi]

aspen-sops rotate [OPTIONS] <FILE>
    --cluster-ticket <TICKET>    Aspen cluster ticket
    --in-place                   Rotate in place

aspen-sops updatekeys [OPTIONS] <FILE>
    --cluster-ticket <TICKET>    Add Aspen Transit key group
    --transit-key <NAME>         Transit key name
    --add-age <RECIPIENT>        Add age recipient
    --remove-age <RECIPIENT>     Remove age recipient
    --in-place                   Update in place

aspen-sops keyservice [OPTIONS]
    --cluster-ticket <TICKET>    Aspen cluster ticket
    --transit-key <NAME>         Transit key name [default: sops-data-key]
    --socket <PATH>              Unix socket path [default: /tmp/aspen-sops.sock]
```

## Dependencies

```toml
[dependencies]
# Aspen client for Transit RPC
aspen-client = { path = "../aspen-client" }
aspen-client-api = { path = "../aspen-client-api", features = ["secrets"] }
aspen-transport = { path = "../aspen-transport" }

# SOPS-compatible encryption
aes-gcm = "0.10"          # AES-256-GCM for SOPS value encryption
hmac = "0.12"              # HMAC-SHA256 for SOPS MAC
sha2 = "0.10"              # SHA-256 for MAC

# Key material safety
zeroize = { version = "1.8", features = ["derive"] }

# File formats
toml = "0.8"               # TOML parsing/serialization (v1)
toml_edit = "0.22"         # Structure-preserving TOML editing

# gRPC key service (optional feature)
tonic = { version = "0.12", optional = true }
prost = { version = "0.13", optional = true }
tokio-stream = { version = "0.1", optional = true }

# CLI
clap = { version = "4", features = ["derive"] }

# Iroh networking
iroh = { version = "0.95.1" }

# Serialization
serde = { version = "1.0", features = ["derive"] }
base64 = "0.22"

# Age support (for hybrid encryption)
age = { version = "0.11", features = ["armor"], optional = true }

# Error handling
snafu = "0.8.9"
tracing = "0.1"
tracing-subscriber = "0.3"

# Async
tokio = { version = "1.48", features = ["full"] }

[features]
default = ["age-fallback"]
keyservice = ["tonic", "prost", "tokio-stream"]  # gRPC bridge
age-fallback = ["age"]                            # age recipient support
full = ["keyservice", "age-fallback"]
```

## Security Considerations

1. **Data key zeroization**: Plaintext data keys wrapped in `Zeroizing<Vec<u8>>` — automatically wiped on drop.
2. **No key material on disk**: Only the Transit-encrypted data key is stored in the SOPS file. Plaintext key exists only in memory during encrypt/decrypt.
3. **MAC verification before use**: Always verify MAC before returning decrypted values to prevent tampered-file attacks.
4. **Network requirement**: Transit operations require Aspen cluster connectivity. For offline scenarios, use `--age` flag to add an age recipient as fallback.
5. **Transit key access control**: Access to Transit encrypt/decrypt is governed by Aspen's capability token system. The `aspen-sops` binary uses a capability token (from env or config) for authentication.

## Tiger Style Compliance

- **Verified core**: `src/verified/mac.rs` for MAC computation — pure, deterministic, Verus-verified
- **Resource bounds**: Max file size (1 MB), max value count (10,000), max key path length (512 bytes)
- **No `.unwrap()` in production**: All errors propagated via snafu `context()`
- **Explicitly sized types**: `u32` for key versions, `u64` for timestamps
- **Functions under 70 lines**: Complex operations split into helpers
