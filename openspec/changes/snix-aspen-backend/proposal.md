## Why

Aspen's self-hosting pipeline needs to serve Nix store paths from its distributed Raft cluster to microVMs running CI builds. The snix-store VirtioFS daemon already boots and serves /nix/store, but it currently uses local-only backends (memory, redb). To make the Nix store distributed and replicated, we need adapter implementations of snix's three storage service traits (BlobService, DirectoryService, PathInfoService) backed by Aspen's Raft KV and iroh-blobs.

## What Changes

- New crate `aspen-snix-backend` implementing snix's `BlobService`, `DirectoryService`, and `PathInfoService` traits with Aspen storage backends
- `BlobService` adapter: delegates to Aspen's iroh-blobs content-addressed store
- `DirectoryService` adapter: stores directory tree nodes in Aspen Raft KV under `snix:dir:` prefix
- `PathInfoService` adapter: stores path info entries in Aspen Raft KV under `snix:pathinfo:` prefix
- Wire the backends into `aspen-ci-executor-nix` so CI builds store artifacts in the distributed store
- Update `snix-store virtiofs` invocation in snix-boot to accept Aspen-backed service URLs

## Capabilities

### New Capabilities

- `snix-blob-backend`: BlobService implementation backed by Aspen's iroh-blobs, supporting has/open_read/open_write/chunks operations via the distributed blob store
- `snix-directory-backend`: DirectoryService implementation backed by Aspen Raft KV, storing BLAKE3-keyed directory nodes with protobuf encoding
- `snix-pathinfo-backend`: PathInfoService implementation backed by Aspen Raft KV, storing PathInfo entries keyed by store path output hash

### Modified Capabilities

## Impact

- New crate: `crates/aspen-snix-backend/` (~300-500 lines per trait adapter)
- Dependencies: `snix-castore`, `snix-store`, `aspen-core`, `aspen-client-api`, `iroh-blobs`
- `crates/aspen-ci-executor-nix/src/config.rs`: add Aspen-backed service constructors
- `nix/snix-boot/default.nix`: support passing gRPC service URLs for distributed backends
- Existing snix-store binary is unchanged — adapters plug in via snix's composition/registry system
