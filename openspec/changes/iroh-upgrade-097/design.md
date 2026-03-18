## Context

Aspen uses iroh for all inter-node and client communication. The iroh dependency spans 49 crates, 134 source files, and 290 import sites. Upstream iroh has released two minor versions (0.96, 0.97) since aspen's pin at 0.95.1, each with significant breaking changes. The `h3-iroh` crate from `iroh-experiments` enables HTTP/3 over iroh QUIC endpoints but requires iroh 0.96+.

The upgrade must be done as a single atomic change — partial upgrades leave iroh version conflicts across the workspace. iroh-blobs 0.99 and iroh-docs 0.97 must be upgraded in lockstep since they depend on iroh 0.97.

## Goals / Non-Goals

**Goals:**
- Upgrade iroh to 0.97.0, iroh-blobs to 0.99.0, iroh-docs to 0.97.0
- Enable h3-iroh dependency (un-comment disabled code)
- All existing tests pass after migration
- Migrate nix-cache-gateway from axum TCP to h3-iroh serving

**Non-Goals:**
- Forge web frontend (separate change, built on h3-iroh after this lands)
- Exploiting new iroh features (multipath, custom transports, endpoint hooks) — just migrate the API, don't adopt new capabilities yet
- Upgrading snix's iroh dependency (snix is vendored, pins its own iroh version)

## Decisions

**1. Upgrade order: version bumps → compile fixes → test fixes**

Bump all Cargo.toml versions first, then fix compilation errors crate-by-crate in dependency order (aspen-core → aspen-transport → aspen-cluster → aspen-raft-network → everything else). Run tests last.

**2. Use a worktree for isolation**

Create a `git worktree` at `/tmp/pi-worktrees/aspen/iroh-097` to avoid breaking the main working tree during the multi-crate migration.

**3. Discovery → AddressLookup migration**

The `Discovery` trait is renamed to `AddressLookup`. Aspen has custom discovery implementations in `aspen-dht-discovery` and `aspen-cluster`. Rename all trait impls and update ALPN constants if any changed.

**4. Endpoint::add_node_addr removal**

Replace calls to `Endpoint::add_node_addr` with the equivalent static address provider pattern. Aspen uses this in bootstrap and cluster join flows — switch to `StaticProvider` or direct `AddressLookup` registration.

**5. noq (quinn fork) adaptation**

iroh 0.97 switched from quinn to noq. Most quinn types are re-exported through iroh, so the impact is limited to code that imported quinn directly. Check for any direct `quinn::` imports in aspen.

**6. h3-iroh integration for nix-cache-gateway**

Replace the axum TCP listener in `aspen-nix-cache-gateway` with h3-iroh's axum feature, which runs axum's router over HTTP/3 QUIC. The gateway will listen on the iroh endpoint's ALPN instead of a separate TCP port. Keep axum as the router framework — only the transport layer changes.

**7. iroh-tickets crate**

Tickets moved from `iroh-base` to `iroh-tickets`. Update imports. Aspen uses tickets in `aspen-transport` and `aspen-cluster`.

## Risks / Trade-offs

**Risk: snix iroh version conflict.** snix is vendored and pins its own iroh version. If snix internally uses iroh types that cross the API boundary with aspen code, version mismatches will cause compilation errors. Mitigation: snix's iroh usage is behind its own trait abstractions (BlobService, DirectoryService) — no iroh types leak into aspen's snix integration layer.

**Risk: NixOS VM tests.** Cluster formation, blob transfer, and gossip tests exercise the iroh networking stack end-to-end. These are the most likely to break from subtle behavioral changes in the new iroh version. Mitigation: run the full VM test suite after migration.

**Risk: h3-iroh maturity.** h3-iroh is in `iroh-experiments`, not a stable crate. API may change. Mitigation: isolate h3-iroh usage behind a feature flag (`h3-serving`) so it can be disabled if issues arise.

**Trade-off: Big-bang vs incremental.** An incremental upgrade (0.95→0.96, then 0.96→0.97) would be safer but doubles the migration work since both versions have breaking changes. The big-bang approach (0.95→0.97) is riskier but saves significant effort. Chosen: big-bang, since aspen has strong test coverage.
