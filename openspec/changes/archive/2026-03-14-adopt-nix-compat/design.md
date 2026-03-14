## Context

`aspen-cache` contains ~370 lines of hand-rolled Nix primitives across three modules (`nix32.rs`, `signing.rs`, narinfo methods on `CacheEntry`). The CI executors shell out to `nix nar dump-path` in 3 places to create NAR archives from store paths.

The snix project provides a complete stack:

- **`nix-compat`**: Format types — `nixbase32`, `narinfo::{NarInfo, SigningKey, VerifyingKey, fingerprint, parse_keypair}`, `store_path::StorePath`, `nar::writer`, `nix_http` (MIME types, URL parsing)
- **`snix-store`**: Higher-level operations — `nar::write_nar` (async NAR rendering from castore nodes), `nar::ingest_nar_and_hash` (NAR → castore decomposition with simultaneous SHA-256)
- **`snix/nar-bridge`**: Reference Nix binary cache HTTP server using axum — demonstrates the `PathInfo → NarInfo → Display` pattern for narinfo serving
- **`snix/nix-daemon`**: `NixDaemonIO` implementation — handles `query_path_info`, `add_to_store_nar` backed by snix services. This is the server side of the nix daemon protocol (rio-build implements the client side)

We already depend on nix-compat (workspace dep at git rev `180bfc4`) and snix-store across 4 crates. `aspen-cache` predates this integration.

## Goals / Non-Goals

**Goals:**

- Replace all hand-rolled Nix encoding/parsing/signing in `aspen-cache` with `nix_compat` equivalents
- Replace `nix nar dump-path` subprocess calls with `nix_compat::nar::writer` for pure-Rust NAR creation
- Compute SHA-256 of NAR archives during serialization (single pass) instead of as a separate step
- Maintain identical wire-format compatibility (narinfo output, signatures, nix32 encoding)
- Match snix's `nar-bridge` pattern for narinfo rendering in the gateway

**Non-Goals:**

- Changing the KV storage schema or `CacheEntry` format
- Adopting the nix daemon protocol for builds (that's a separate rio-build-style change)
- Replacing `nix path-info --json` calls (nix-compat has no client-side daemon protocol for this; snix/nix-daemon is server-side only)
- Replacing `nix build` or `nix flake archive` calls
- Touching the snix integration crates (`aspen-snix`, `aspen-castore`)

## Decisions

### 1. Keep `CacheEntry` as the storage type, convert to `NarInfo` at render time

**Decision**: `CacheEntry` remains the serde struct stored in KV. When narinfo text is needed (gateway response), construct a `nix_compat::narinfo::NarInfo` from `CacheEntry` fields and use its `Display` impl. This mirrors snix's `nar-bridge` pattern where `PathInfo::to_narinfo()` produces a `NarInfo` for rendering.

**Rationale**: `NarInfo` uses borrowed references (`StorePathRef<'a>`, `&'a str`) — designed for parsing/rendering, not persistence. `CacheEntry` uses owned `String` fields with serde — right shape for KV storage. snix's `nar-bridge` takes the same approach: `PathInfo` is the storage type, `NarInfo` is the rendering type.

### 2. Replace `CacheSigningKey`/`CacheVerifyingKey` with thin wrappers around nix-compat

**Decision**: Create `CacheKeyPair` wrapping `nix_compat::narinfo::SigningKey<ed25519_dalek::SigningKey>` with KV persistence via `ensure_signing_key()`. For verification, use `nix_compat::narinfo::VerifyingKey` directly.

**Rationale**: The KV persistence logic is aspen-specific. The cryptographic operations belong in nix-compat. nix-compat's `parse_keypair()` already handles the `name:base64(secret||public)` format parsing. ~50 lines of wrapper vs ~280 lines of reimplemented crypto.

### 3. Replace `nix nar dump-path` with `nix_compat::nar::writer` in `spawn_blocking`

**Decision**: Write a `dump_path_nar(store_path) -> (Vec<u8>, [u8; 32])` helper that walks the filesystem using `nix_compat::nar::writer::open()` and simultaneously computes SHA-256 via a `HashingWriter<W>` wrapper. Run in `tokio::task::spawn_blocking` since nix-compat's NAR writer is synchronous.

**Rationale**: nix-compat's NAR writer (`nar::writer::sync`) produces byte-identical output to `nix-store --dump`. The API is a state machine: `open()` → `Node` → `file()`/`directory()`/`symlink()`. Walking the filesystem and calling these methods is straightforward — rio-build's `dump_path_streaming()` does exactly this in ~80 lines.

The `HashingWriter` computes SHA-256 in the same write pass, eliminating the current two-pass approach (dump NAR bytes, then hash them). For large outputs (hundreds of MB), this halves memory bandwidth.

**Call sites replaced:**

1. `aspen-ci-executor-nix/src/cache.rs:107` — blob store upload
2. `aspen-ci-executor-nix/src/snix.rs:63` — snix storage upload
3. `aspen-ci-executor-shell/src/local_executor/snix.rs:109` — local executor snix upload

### 4. Use `StorePath::from_absolute_path()` for store path parsing

**Decision**: Replace `parse_store_path()` with nix-compat's `StorePath::from_absolute_path()`. This validates hash characters (exactly 32 nixbase32 chars from the nix alphabet) and name constraints. Wrap for backward compatibility where `(String, String)` return is needed.

### 5. Drop `nix32.rs` entirely, re-export `nixbase32`

**Decision**: Delete `aspen_cache::nix32`. Re-export `nix_compat::nixbase32` from `aspen-cache`. Update the `hex_to_nix32()` call site in `cache.rs` to use `nixbase32::encode(&hex::decode(hex_part))` directly.

## Risks / Trade-offs

- **[Fingerprint format]** → Verified: both implementations produce `1;/nix/store/<hash>-<name>;sha256:<nix32_hash>;<size>;<comma_refs>`. Existing signatures valid.
- **[NarHash format conversion]** → `CacheEntry.nar_hash` stores `"sha256:<nix32>"`. `NarInfo` expects `[u8; 32]`. Conversion via `nixbase32::decode_fixed::<32>()` at render time.
- **[NAR byte identity]** → nix-compat's NAR writer is verified against `nix-store --dump` in its own test suite. rio-build independently verified byte identity. We add our own golden test comparing output against the subprocess for a test store path.
- **[spawn_blocking overhead]** → NAR serialization is I/O-bound (filesystem reads). `spawn_blocking` is the standard pattern for sync I/O in tokio. No worse than the current subprocess spawn, and avoids process creation overhead.
- **[nix-compat version pinning]** → Pinned to git rev `180bfc4`. Already shared across 4 crates, so any breakage caught regardless.
- **[Breaking `CacheSigningKey` API]** → 3 call sites: `ensure_signing_key()`, gateway startup, rpc-handlers test. All internal.
