## ADDED Requirements

### Requirement: nixbase32 encoding uses nix-compat

All nixbase32 encoding and decoding in `aspen-cache` SHALL use `nix_compat::nixbase32` instead of the hand-rolled `aspen_cache::nix32` module.

#### Scenario: Encode SHA-256 hash to nix32

- **WHEN** a 32-byte SHA-256 digest is encoded to nixbase32
- **THEN** the output SHALL be 52 characters using the nix alphabet (`0123456789abcdfghijklmnpqrsvwxyz`) and SHALL match the output of `nix-hash --type sha256 --to-base32`

#### Scenario: Known cowsay hash round-trip

- **WHEN** the hex hash `dc810f96d61eb9414b986771cb4e2f51c6b879314e2f4296f8d926d0cfa35993` is converted to nix32
- **THEN** the result SHALL be `14srlg7x09nrz2b44bsf65wviiji5x7cnwb7k15l3f8yssb0z0fw`

### Requirement: Store path parsing uses nix-compat StorePath

All Nix store path parsing in `aspen-cache` SHALL use `nix_compat::store_path::StorePath` for validation and component extraction.

#### Scenario: Parse valid store path

- **WHEN** `/nix/store/w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1` is parsed
- **THEN** the hash component SHALL be `w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb` and the name SHALL be `hello-2.12.1`

#### Scenario: Reject invalid store path prefix

- **WHEN** `/usr/bin/hello` is parsed as a store path
- **THEN** parsing SHALL return an error

#### Scenario: Reject invalid hash characters

- **WHEN** `/nix/store/INVALID_UPPER-hello` is parsed as a store path
- **THEN** parsing SHALL return an error (nix-compat validates hash character set; the hand-rolled parser did not)

### Requirement: Narinfo rendering uses nix-compat NarInfo

The narinfo text output for cache entries SHALL be produced by constructing a `nix_compat::narinfo::NarInfo` struct and formatting it via its `Display` implementation.

#### Scenario: Narinfo contains required fields

- **WHEN** a `CacheEntry` with store path, NAR hash, NAR size, references, deriver, and blob hash is rendered to narinfo
- **THEN** the output SHALL contain `StorePath:`, `URL:`, `Compression:`, `NarHash:`, `NarSize:`, `References:`, `Deriver:`, and `Sig:` fields

#### Scenario: Narinfo hash is nix32-encoded

- **WHEN** a `CacheEntry` narinfo is rendered
- **THEN** the `NarHash:` field SHALL use `sha256:<nix32_encoded_hash>` format (52 chars of nix alphabet)

#### Scenario: References are store path basenames

- **WHEN** a `CacheEntry` with references `["/nix/store/abc-glibc", "/nix/store/def-gcc"]` is rendered
- **THEN** the `References:` line SHALL contain basenames `abc-glibc def-gcc` (not full paths)

### Requirement: Narinfo fingerprint uses nix-compat

Narinfo fingerprint computation for signing SHALL use `nix_compat::narinfo::fingerprint()`.

#### Scenario: Fingerprint format matches Nix specification

- **WHEN** a fingerprint is computed for a store path with hash, NAR hash, NAR size, and references
- **THEN** the format SHALL be `1;/nix/store/<hash>-<name>;sha256:<nix32_nar_hash>;<nar_size>;<comma_separated_absolute_ref_paths>`

#### Scenario: Existing signatures remain valid

- **WHEN** a narinfo fingerprint is computed using nix-compat for the same `CacheEntry` data that was previously signed with the hand-rolled implementation
- **THEN** the fingerprint string SHALL be byte-identical, and existing Ed25519 signatures SHALL verify successfully

### Requirement: Cache signing uses nix-compat signing keys

Ed25519 signing and verification of narinfo fingerprints SHALL use `nix_compat::narinfo::{SigningKey, VerifyingKey, parse_keypair}`.

#### Scenario: Sign narinfo with generated key

- **WHEN** a signing key is generated and used to sign a narinfo fingerprint
- **THEN** the signature SHALL be in Nix format `<name>:<base64_ed25519_signature>` and SHALL verify with the corresponding verifying key

#### Scenario: Parse existing Nix-format secret key

- **WHEN** a secret key string in `name:base64(secret||public)` format (64 bytes decoded) is parsed
- **THEN** it SHALL produce a valid signing key that generates correct signatures

#### Scenario: KV persistence of signing keys

- **WHEN** `ensure_signing_key()` is called and no key exists in KV
- **THEN** a new keypair SHALL be generated using nix-compat, serialized to Nix format, and stored at `_sys:nix-cache:signing-key` and `_sys:nix-cache:public-key`

### Requirement: Gateway serves narinfo via nix-compat rendering

The `aspen-nix-cache-gateway` HTTP server SHALL construct narinfo responses using nix-compat types instead of `CacheEntry::to_narinfo()`.

#### Scenario: Gateway narinfo response is valid

- **WHEN** a client requests `GET /{hash}.narinfo` and the cache entry exists
- **THEN** the response body SHALL be a valid narinfo document parseable by `nix_compat::narinfo::NarInfo::parse()`

#### Scenario: Gateway narinfo is signed

- **WHEN** a narinfo response is served
- **THEN** the `Sig:` field SHALL be present and SHALL verify against the cache's public key using `nix_compat::narinfo::VerifyingKey`

### Requirement: NAR archives produced by nix-compat writer

NAR archives for store path upload SHALL be produced by `nix_compat::nar::writer` walking the filesystem in pure Rust, replacing `nix nar dump-path` subprocess calls.

#### Scenario: NAR output is byte-identical to nix CLI

- **WHEN** a store path is serialized to NAR using `nix_compat::nar::writer`
- **THEN** the output SHALL be byte-identical to `nix-store --dump` or `nix nar dump-path` on the same path

#### Scenario: SHA-256 computed during NAR serialization

- **WHEN** a store path is serialized to NAR for cache upload
- **THEN** the SHA-256 hash SHALL be computed in the same write pass via a hashing writer, not as a separate step over the completed byte buffer

#### Scenario: No nix CLI subprocess for NAR creation

- **WHEN** a CI build output is uploaded to the blob store or snix storage
- **THEN** no `nix nar dump-path` subprocess SHALL be spawned; the NAR SHALL be produced in-process

#### Scenario: Regular files, directories, symlinks, and executables handled

- **WHEN** a store path containing regular files, executable files, directories, and symlinks is serialized
- **THEN** all node types SHALL be correctly represented in the NAR output with proper permissions (executable bit) and sorted directory entries

#### Scenario: NAR creation runs in spawn_blocking

- **WHEN** NAR serialization is invoked from an async context
- **THEN** the synchronous filesystem walk and NAR writing SHALL execute inside `tokio::task::spawn_blocking` to avoid blocking the async runtime
