## Why

Aspen can store Nix build artifacts (NARs in blob store, metadata in cache index) and CI can upload them after builds, but there's no way for `nix build` to **pull** store paths back from aspen. The download path is completely missing. This means CI builds can't benefit from cached artifacts, and users can't fetch pre-built packages — defeating the purpose of the distributed binary cache.

The snix gRPC bridge (`aspen-snix-bridge`) already provides BlobService/DirectoryService/PathInfoService over Unix sockets, and the cache index (`aspen-cache`) tracks narinfo metadata in Raft KV. The pieces exist; they just need to be wired together so that standard Nix tooling can substitute from aspen's cache.

## What Changes

- **New `aspen-nix-cache-gateway` binary**: Lightweight HTTP/1.1 server implementing the Nix binary cache protocol (`/nix-cache-info`, `/{hash}.narinfo`, `/nar/{hash}.nar`). Translates HTTP requests → aspen client RPC calls. Runs locally alongside build machines. This is the same pattern as `git-remote-aspen` — a protocol bridge for ecosystem compatibility.
- **Cache signing**: Generate Ed25519 keypair at cluster init, sign narinfo responses, distribute public key via cluster RPC. Without signatures, Nix rejects substituted paths.
- **CI builder integration**: Configure Nix substituter in CI executor environment — inject `substituters` and `trusted-public-keys` into worker Nix config so `nix build` transparently pulls from aspen's cache.
- **Store path upload improvements**: Ensure CI's `upload_store_paths` correctly populates all narinfo fields needed by the gateway (NAR hash, NAR size, references, deriver, signatures).

## Capabilities

### New Capabilities

- `nix-cache-gateway`: HTTP/1.1 server implementing Nix binary cache protocol, translating requests to aspen RPC. Serves narinfo files and NAR downloads via blob tickets.
- `nix-cache-signing`: Ed25519 keypair generation, narinfo signing, and public key distribution for cache trust.
- `nix-substituter-config`: CI executor integration that configures `nix.conf` substituters and trusted public keys for transparent cache hits during builds.

### Modified Capabilities

- `ci`: CI executor environment gains automatic substituter configuration when a cache gateway is available.

## Impact

- **New crate**: `aspen-nix-cache-gateway` (binary, HTTP server using hyper)
- **Modified crates**: `aspen-cache` (signing support), `aspen-ci-executor-nix` (substituter config), `aspen-nix-handler` (public key RPC), `aspen-secrets` (cache keypair storage)
- **Dependencies**: `hyper` (HTTP/1.1 only — no axum, minimal surface), `ed25519-dalek` (signing, already in workspace)
- **NixOS VM test**: End-to-end test — CI builds a derivation, uploads to cache, separate machine substitutes from cache gateway
- **No breaking changes**: All additive
