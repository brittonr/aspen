## 1. Add nix-compat dependency to aspen-cache

- [x] 1.1 Add `nix-compat = { workspace = true }` to `crates/aspen-cache/Cargo.toml` dependencies
- [x] 1.2 Remove direct `ed25519-dalek`, `rand_core_06`, `base64`, and `hex` deps from `crates/aspen-cache/Cargo.toml` (nix-compat provides these transitively or they're no longer needed)
- [x] 1.3 Verify `cargo check -p aspen-cache` compiles with the new dep

## 2. Replace nix32 module with nix-compat nixbase32

- [x] 2.1 Delete `crates/aspen-cache/src/nix32.rs`
- [x] 2.2 Add `pub use nix_compat::nixbase32;` re-export in `crates/aspen-cache/src/lib.rs` (replaces `pub mod nix32`)
- [x] 2.3 Update all callers of `aspen_cache::nix32::encode` to use `nix_compat::nixbase32::encode` (search: `nix32::encode`, `nix32::hex_to_nix32`)
- [x] 2.4 Replace `hex_to_nix32()` call sites with inline `nixbase32::encode(&hex::decode(hex_data))` or a thin helper
- [x] 2.5 Run existing nix32 tests (cowsay hash vector, SHA-256 length) against `nixbase32::encode` to confirm identical output

## 3. Replace store path parsing with nix-compat StorePath

- [x] 3.1 Rewrite `parse_store_path()` in `crates/aspen-cache/src/index.rs` to use `nix_compat::store_path::StorePath::from_absolute_path()` internally, returning `(String, String)` for backward compat
- [x] 3.2 Update `validate_store_hash()` in `KvCacheIndex` to use `StorePath` validation instead of manual length/character checks
- [x] 3.3 Verify existing `parse_store_path` tests pass with the new implementation
- [x] 3.4 Add test for invalid hash character rejection (uppercase, invalid nix32 chars like `e`, `o`, `t`, `u`)

## 4. Replace signing with nix-compat narinfo signing keys

- [x] 4.1 Rewrite `CacheSigningKey` as a wrapper around `nix_compat::narinfo::SigningKey<ed25519_dalek::SigningKey>` — keep `generate()`, `from_nix_format()`, `to_nix_secret_key()`, `to_nix_public_key()`, `sign_fingerprint()`, `name()` methods with identical signatures
- [x] 4.2 Rewrite `CacheVerifyingKey` as a wrapper around `nix_compat::narinfo::VerifyingKey` — keep `from_nix_format()` and `verify_signature()` methods
- [x] 4.3 Keep `ensure_signing_key()` async function with same KV persistence behavior, using the new wrapper types internally
- [x] 4.4 Remove `split_nix_key()` and `validate_cache_name()` helper functions (nix-compat handles key format parsing)
- [x] 4.5 Run existing signing tests (generate+roundtrip, sign+verify, wrong fingerprint, wrong name, invalid formats) to confirm behavior matches

## 5. Replace narinfo rendering with nix-compat NarInfo

- [x] 5.1 Add `CacheEntry::to_nix_narinfo()` method that constructs a `nix_compat::narinfo::NarInfo` from entry fields — decode `nar_hash` string to `[u8; 32]` via `nixbase32::decode_fixed`, parse `store_path` to `StorePathRef`, parse references to `Vec<StorePathRef>`
- [x] 5.2 Replace `CacheEntry::to_narinfo(&self, signature: Option<&str>) -> String` to use the new `to_nix_narinfo()` internally with `Display` formatting
- [x] 5.3 Replace `CacheEntry::fingerprint()` to use `nix_compat::narinfo::fingerprint()` — parse store path and references to `StorePathRef`, decode nar_hash to `[u8; 32]`
- [x] 5.4 Run existing narinfo tests (complete entry, minimal entry, with signature, fingerprint basic, fingerprint with sorted refs, roundtrip parseable) to confirm output matches
- [x] 5.5 Add test: parse the rendered narinfo string back with `nix_compat::narinfo::NarInfo::parse()` to verify it's valid nix-compat-parseable narinfo

## 6. Update gateway to use nix-compat rendering

- [x] 6.1 Update `handle_narinfo()` in `crates/aspen-nix-cache-gateway/src/server.rs` to use the updated `CacheEntry::to_narinfo()` (which now uses nix-compat internally)
- [x] 6.2 Verify the gateway test in `crates/aspen-nix-cache-gateway/tests/gateway_test.rs` still passes
- [x] 6.3 Update the signing test in `crates/aspen-rpc-handlers/tests/unit_nix_cache_signing.rs` if it references old signing types

## 7. Replace nix nar dump-path with nix-compat NAR writer

- [x] 7.1 Create `crates/aspen-cache/src/nar.rs` module with a `dump_path_nar(path: &Path) -> Result<(Vec<u8>, [u8; 32])>` function that walks the filesystem using `nix_compat::nar::writer::open()` and computes SHA-256 via a `HashingWriter` wrapper in the same pass
- [x] 7.2 Implement the `HashingWriter<W: Write>` wrapper that hashes all bytes written through it with `sha2::Sha256` (same pattern as rio-build's `CountingWriter` but for hashing)
- [x] 7.3 Handle all three NAR node types: regular files (with executable bit detection via `PermissionsExt::mode()`), directories (sorted entries via `read_dir` + sort by `file_name`), symlinks (via `read_link`)
- [x] 7.4 Add unit test: roundtrip a tempdir through `dump_path_nar` and verify the output parses back with `nix_compat::nar::reader`
- [x] 7.5 Add golden test: compare `dump_path_nar` output against `nix-store --dump` for a known store path (skip if nix not available, like rio-build does)
- [x] 7.6 Add `dump_path_nar_async(path: PathBuf) -> Result<(Vec<u8>, [u8; 32])>` that wraps `dump_path_nar` in `tokio::task::spawn_blocking`

## 8. Wire NAR writer into CI executors

- [x] 8.1 Add `nix-compat = { workspace = true }` to `crates/aspen-ci-executor-nix/Cargo.toml` as unconditional dep (currently optional behind `snix` feature)
- [x] 8.2 Replace `Command::new(&self.config.nix_binary).args(["nar", "dump-path", store_path])` in `crates/aspen-ci-executor-nix/src/cache.rs` with `dump_path_nar_async`. Use the returned `[u8; 32]` SHA-256 directly instead of computing it separately with `sha2::Sha256`
- [x] 8.3 Replace the same subprocess call in `crates/aspen-ci-executor-nix/src/snix.rs`
- [x] 8.4 Add `nix-compat = { workspace = true }` to `crates/aspen-ci-executor-shell/Cargo.toml` as unconditional dep (currently optional behind `snix` feature)
- [x] 8.5 Replace the same subprocess call in `crates/aspen-ci-executor-shell/src/local_executor/snix.rs`
- [x] 8.6 Remove `sha2` direct dependency from `aspen-ci-executor-nix` if no other call sites use it (the HashingWriter in aspen-cache handles hashing now)

## 9. Clean up and verify

- [x] 9.1 Remove any remaining imports of old `aspen_cache::nix32`, `aspen_cache::signing` internals that reference deleted code
- [x] 9.2 Run `cargo clippy -p aspen-cache -p aspen-nix-cache-gateway -p aspen-nix-handler -p aspen-rpc-handlers -p aspen-ci-executor-nix -p aspen-ci-executor-shell -- --deny warnings`
- [x] 9.3 Run `cargo nextest run -E 'package(aspen-cache) | package(aspen-nix-cache-gateway) | package(aspen-nix-handler) | package(aspen-rpc-handlers) | package(aspen-ci-executor-nix) | package(aspen-ci-executor-shell)'`
- [x] 9.4 Run `nix run .#rustfmt` to format
- [x] 9.5 Verify no remaining `Command::new("nix").args(["nar", "dump-path"` patterns exist: `rg 'nar.*dump.path' crates/ --glob '*.rs'`
