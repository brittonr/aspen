## Why

Aspen's blob/castore/cache family is high-value outside the node runtime: iroh-backed content-addressed blobs, snix castore integration, Nix binary cache metadata/signing, and cache publishing. Today those crates mix reusable storage/cache APIs with Aspen client API integration, core-shell helpers, replication RPCs, and test/runtime assumptions. That makes them harder to reuse independently and harder to reason about as a clean layer above the extracted Raft KV and protocol crates.

This change separates the reusable content-addressed storage/cache surface from Aspen-specific replication, RPC, and node integration while keeping Aspen's existing blob/cache behavior intact through explicit adapter features or shell crates.

## What Changes

- Create an extraction manifest and readiness policy for the blob/castore/cache family: `aspen-blob`, `aspen-castore`, `aspen-cache`, and related cache helper crates if needed.
- Split `aspen-blob` default reusable APIs from Aspen replication/client-RPC concerns. Iroh/iroh-blobs remain allowed because they are the backend's explicit purpose; root Aspen app/runtime crates are not.
- Split `aspen-castore` reusable snix trait adapters from Aspen core-shell circuit-breaker/runtime integration, either by using a local reusable circuit breaker helper or by gating Aspen-specific helpers behind named features.
- Split `aspen-cache` reusable Nix binary cache metadata/signing/NAR-facing APIs from Aspen cluster/testing/runtime integration.
- Add downstream-style fixtures for blob storage and cache/castore type usage without root Aspen, handlers, binaries, cluster bootstrap, or RPC handler crates.
- Preserve Aspen compatibility through explicit adapter features and representative consumer checks.

## Capabilities

### New Capabilities

- `blob-castore-cache-extraction`: Extraction boundary, feature contract, and standalone verification rails for content-addressed blob storage, snix castore adapters, and Nix cache metadata/signing crates.

### Modified Capabilities

- `architecture-modularity`: Updates crate extraction inventory and policy for the blob/castore/cache family.

## Impact

- **Crates modified**: `aspen-blob`, `aspen-castore`, `aspen-cache`, possibly `aspen-exec-cache` if the family manifest includes it.
- **Docs modified**: `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, new `docs/crate-extraction/blob-castore-cache.md`.
- **Consumers verified**: `aspen-rpc-core`/handlers with blob features, `aspen-snix`, `aspen-snix-bridge`, `aspen-nix-cache-gateway`, CI executor/cache consumers, and downstream fixtures.
- **APIs**: Reusable blob/cache APIs remain canonical; Aspen replication/RPC integration moves behind explicit adapter features or shells if needed.
- **Dependencies**: Iroh/iroh-blobs and snix/nix-compat are allowed only where they are the crate's declared backend/domain purpose. Root Aspen app/runtime shells, handler registries, node bootstrap, trust/secrets/SQL, and UI/web binaries are forbidden from reusable defaults.
