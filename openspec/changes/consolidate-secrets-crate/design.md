# consolidate-secrets-crate — Design

## Architecture

### Before

```
aspen-auth/
├── hmac_auth.rs          ← HMAC-SHA256 challenge-response
├── verified_auth.rs      ← derive_hmac_key, constant_time_compare
└── builder.rs            ← TokenBuilder (uses SecretKey directly)

aspen-sops/                ← Standalone crate (~24 files)
├── encrypt.rs            ← SOPS encrypt (AES-256-GCM, Transit data key)
├── decrypt.rs            ← SOPS decrypt (duplicate of aspen-secrets logic)
├── mac.rs                ← MAC computation (HMAC-SHA256)
├── format/{toml,yaml,json}.rs
├── metadata.rs           ← AspenTransitRecipient
├── client.rs             ← TransitClient
├── verified/mac.rs       ← Pure MAC functions
└── ...13 more files

aspen-secrets/
├── sops/decryptor.rs     ← age-based SOPS decrypt (DUPLICATE encrypt/decrypt logic)
├── sops/provider.rs      ← SecretsManager
├── transit/              ← Transit engine
├── kv/                   ← KV secrets engine
└── pki/                  ← PKI engine

aspen-cluster/
├── validation.rs         ← validate_cookie(), UNSAFE_DEFAULT_COOKIE
├── config/iroh_config.rs ← secret_key: Option<String> (hex parsing)
└── config/nix_cache.rs   ← signing_key_name config

aspen-secrets-handler/
└── handler/nix_cache.rs  ← Nix cache key CRUD (wraps Transit)

aspen-dht-discovery/
└── hash.rs               ← iroh_secret_to_signing_key() conversion
```

### After

```
aspen-secrets/
├── sops/
│   ├── decryptor.rs      ← Unified SOPS decrypt (age + Transit)
│   ├── encrypt.rs        ← SOPS encrypt (from aspen-sops)
│   ├── decrypt.rs        ← SOPS decrypt (from aspen-sops, replaces duplicate)
│   ├── edit.rs           ← SOPS edit workflow (from aspen-sops)
│   ├── rotate.rs         ← SOPS key rotation (from aspen-sops)
│   ├── updatekeys.rs     ← SOPS key group management (from aspen-sops)
│   ├── mac.rs            ← MAC computation (from aspen-sops)
│   ├── metadata.rs       ← SOPS metadata types (from aspen-sops)
│   ├── client.rs         ← TransitClient for SOPS (from aspen-sops)
│   ├── config.rs         ← SecretsConfig (existing)
│   ├── provider.rs       ← SecretsManager (existing)
│   └── format/
│       ├── mod.rs        ← Format detection (from aspen-sops)
│       ├── common.rs     ← Shared encrypt/decrypt value helpers
│       ├── toml.rs       ← TOML tree walk (from aspen-sops)
│       ├── yaml.rs       ← YAML support (from aspen-sops)
│       └── json.rs       ← JSON support (from aspen-sops)
│
├── hmac/
│   ├── mod.rs            ← HMAC auth types + protocol (from aspen-auth)
│   └── challenge.rs      ← Challenge-response protocol (from aspen-auth)
│
├── cookie/
│   └── mod.rs            ← Cookie validation + key derivation (from aspen-cluster)
│
├── identity/
│   └── mod.rs            ← NodeIdentityProvider (new)
│
├── nix_cache/
│   └── mod.rs            ← Nix cache signing key lifecycle (from aspen-secrets-handler)
│
├── verified/
│   ├── mod.rs            ← (existing)
│   ├── auth.rs           ← derive_hmac_key, constant_time_compare (from aspen-auth)
│   ├── mac.rs            ← Pure SOPS MAC functions (from aspen-sops)
│   └── cookie.rs         ← Pure cookie validation (from aspen-cluster)
│
├── backend.rs            ← (existing, unchanged)
├── constants.rs          ← (existing, extended with HMAC/cookie constants)
├── error.rs              ← (existing, extended with new error variants)
├── kv/                   ← (existing, unchanged)
├── transit/              ← (existing, unchanged)
├── pki/                  ← (existing, unchanged)
├── mount_registry.rs     ← (existing, unchanged)
└── lib.rs                ← Extended re-exports

aspen-sops/                ← Becomes thin CLI-only wrapper
├── src/
│   ├── main.rs           ← CLI entry point (uses aspen-secrets::sops)
│   └── cli.rs            ← Clap definitions
└── Cargo.toml            ← Depends on aspen-secrets instead of duplicating

aspen-auth/                ← Re-exports from aspen-secrets
├── hmac_auth.rs          ← pub use aspen_secrets::hmac::*;
├── verified_auth.rs      ← pub use aspen_secrets::verified::auth::*;
└── ...                   ← Token/capability code unchanged

aspen-cluster/             ← Delegates to aspen-secrets
├── validation.rs         ← Calls aspen_secrets::cookie::* (re-exports old API)
└── config/               ← Calls aspen_secrets::identity::* for key loading
```

## Module Design

### `aspen-secrets::sops` — Unified SOPS Operations

Merges the two SOPS implementations. The existing `decryptor.rs` age-based decrypt stays (node bootstrap). The `aspen-sops` library modules move in alongside it.

**Key change**: `format/common.rs` extracts the shared `encrypt_sops_value()` and `decrypt_sops_value()` functions that were duplicated between `aspen-secrets/sops/decryptor.rs` and `aspen-sops/format/toml.rs`.

```rust
// aspen-secrets/src/sops/format/common.rs
pub fn encrypt_sops_value(plaintext: &str, data_key: &[u8; 32], value_type: &str) -> Result<String>;
pub fn decrypt_sops_value(encrypted: &str, data_key: &[u8; 32]) -> Result<String>;
pub fn is_sops_encrypted(s: &str) -> bool;
```

Both `decryptor.rs` (age path) and `decrypt.rs` (Transit path) call these shared helpers.

### `aspen-secrets::hmac` — HMAC Authentication Primitives

Moves from `aspen-auth`. Zero dependency on token/capability types.

```rust
// aspen-secrets/src/hmac/mod.rs
pub use challenge::{AuthChallenge, AuthResponse, AuthResult, HmacAuthenticator};
pub use crate::verified::auth::{derive_hmac_key, constant_time_compare, is_challenge_valid};

// Constants moved from aspen-auth
pub const AUTH_NONCE_SIZE: usize = 32;
pub const AUTH_HMAC_SIZE: usize = 32;
pub const AUTH_CHALLENGE_MAX_AGE_SECS: u64 = 60;
pub const AUTH_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
```

### `aspen-secrets::cookie` — Cluster Cookie Management

```rust
// aspen-secrets/src/cookie/mod.rs
pub const UNSAFE_DEFAULT_COOKIE: &str = "aspen-cookie-UNSAFE-CHANGE-ME";

pub fn validate_cookie(cookie: &str) -> Result<(), CookieError>;
pub fn validate_cookie_safety(cookie: &str) -> Result<(), CookieError>;
pub fn derive_cookie_hmac_key(cookie: &str) -> [u8; 32];  // was derive_hmac_key
pub fn derive_gossip_topic(cookie: &str) -> [u8; 32];     // new: explicit gossip derivation
```

### `aspen-secrets::identity` — Node Identity Key Lifecycle

```rust
// aspen-secrets/src/identity/mod.rs
use iroh::{SecretKey, PublicKey};

pub struct NodeIdentityProvider {
    secret_key: SecretKey,
}

impl NodeIdentityProvider {
    /// Load from hex string (config file / env var).
    pub fn from_hex(hex: &str) -> Result<Self, SecretsError>;

    /// Generate a new random identity.
    pub fn generate() -> Self;

    /// Load from file or generate if missing.
    pub async fn load_or_generate(path: &Path) -> Result<Self, SecretsError>;

    /// Get the public key (node ID).
    pub fn public_key(&self) -> PublicKey;

    /// Get the secret key (for endpoint creation).
    pub fn secret_key(&self) -> &SecretKey;

    /// Derive a DHT signing key (for aspen-dht-discovery).
    #[cfg(feature = "global-discovery")]
    pub fn dht_signing_key(&self) -> mainline::SigningKey;
}
```

### `aspen-secrets::nix_cache` — Nix Cache Signing

```rust
// aspen-secrets/src/nix_cache/mod.rs
use crate::transit::{TransitStore, CreateKeyRequest, KeyType};

pub struct NixCacheKeyManager<S: crate::SecretsBackend> {
    transit_store: Arc<TransitStore<S>>,
}

impl<S: crate::SecretsBackend> NixCacheKeyManager<S> {
    pub async fn create_signing_key(&self, cache_name: &str) -> Result<PublicKeyInfo>;
    pub async fn get_public_key(&self, cache_name: &str) -> Result<Option<PublicKeyInfo>>;
    pub async fn rotate_signing_key(&self, cache_name: &str) -> Result<PublicKeyInfo>;
    pub async fn delete_signing_key(&self, cache_name: &str) -> Result<bool>;
    pub async fn list_signing_keys(&self) -> Result<Vec<String>>;
}
```

## Feature Flags

The `sops` module (encrypt/decrypt/edit/rotate/format) is gated behind a new `sops` feature on `aspen-secrets` to avoid pulling in `toml_edit`, `serde_yaml`, `serde_json`, `chrono`, and `aspen-client` for consumers that only need the secrets engines.

```toml
[features]
default = []
sops = ["dep:toml_edit", "dep:serde_yaml", "dep:serde_json", "dep:chrono",
        "dep:aspen-client", "dep:aspen-transport", "dep:regex"]
hmac = ["dep:hmac", "dep:sha2"]     # HMAC auth (already in deps, just gating the module)
identity = []                        # Node identity (no extra deps, uses iroh)
nix-cache = []                       # Nix signing key management (uses transit)
full = ["sops", "hmac", "identity", "nix-cache"]
```

The `hmac` and `identity` features are lightweight (no new deps beyond what `aspen-secrets` already has). The `sops` feature is heavier due to the Transit client and format libraries.

## Migration Strategy

### Phase 1: Move code, add re-exports (backward compatible)

1. Copy modules into `aspen-secrets` under new paths
2. Add `pub use aspen_secrets::...` re-exports in source crates
3. All downstream code continues to work unchanged
4. Tests pass with no import changes

### Phase 2: Update downstream imports (optional, incremental)

1. Crate-by-crate, update `use aspen_auth::hmac_auth` → `use aspen_secrets::hmac`
2. Update `use aspen_cluster::validation::validate_cookie` → `use aspen_secrets::cookie::validate_cookie`
3. Re-exports stay for any stragglers

### Phase 3: Remove dead code from source crates

1. Delete original files from `aspen-auth`, `aspen-cluster`, `aspen-sops`
2. `aspen-sops` shrinks to just `main.rs` + `cli.rs`
3. Remove duplicate crypto deps from source crate `Cargo.toml` files

## Dependency Graph

```
aspen-secrets (owns all crypto)
  ├── aspen-core (types only, no crypto)
  ├── aspen-auth (token types only — aspen-secrets does NOT depend on aspen-auth)
  ├── age, aes-gcm, chacha20poly1305, blake3, ed25519-dalek, hmac, sha2, rand, zeroize
  └── [sops feature]: aspen-client, aspen-transport, toml_edit, serde_yaml, chrono

aspen-auth (authorization policy)
  ├── aspen-secrets (for hmac, cookie, verified auth)
  ├── iroh (for SecretKey/PublicKey types)
  └── postcard (token serialization)

aspen-cluster (cluster coordination)
  ├── aspen-secrets (for cookie, identity)
  └── ... (existing deps unchanged)

aspen-sops (CLI binary only)
  ├── aspen-secrets [sops feature] (all library code)
  └── clap (CLI framework)
```

**No cycles**: `aspen-secrets` depends on `aspen-core` (types) and `aspen-auth` (token types for SecretsManager). `aspen-auth` depends on `aspen-secrets` for HMAC primitives. This is NOT a cycle because:

- `aspen-secrets::hmac` has zero imports from `aspen-auth`
- `aspen-secrets::sops::provider` imports `aspen_auth::CapabilityToken` (existing, unchanged)
- The dependency is: `aspen-auth` → `aspen-secrets` (new) and `aspen-secrets` → `aspen-auth` (existing)

Wait — this IS a cycle. To break it:

**Resolution**: Extract `aspen-secrets::hmac` and `aspen-secrets::verified::auth` into a new `aspen-crypto` leaf crate that both `aspen-auth` and `aspen-secrets` depend on. OR: keep HMAC in `aspen-auth` and have `aspen-secrets` depend on `aspen-auth` (current direction).

**Chosen approach**: Keep HMAC primitives in `aspen-auth` (no move). Move only the *cookie* validation and *identity* management into `aspen-secrets` (these don't depend on `aspen-auth` types). The SOPS merge and nix-cache consolidation are the high-value wins.

### Revised Dependency Graph (no cycles)

```
aspen-secrets
  ├── aspen-core (types)
  ├── aspen-auth (token types for SecretsManager — existing dep)
  └── crypto deps (age, aes-gcm, etc.)

aspen-auth
  ├── aspen-core (types)
  ├── iroh (SecretKey/PublicKey)
  ├── hmac, sha2, blake3 (HMAC auth — stays here)
  └── rand (nonce generation)

aspen-sops (CLI binary)
  └── aspen-secrets [sops feature]

aspen-cluster
  ├── aspen-secrets (cookie validation, identity loading)
  └── aspen-auth (HMAC authenticator — existing dep)
```
