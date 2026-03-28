## Context

The native build pipeline in `aspen-ci-executor-nix` has two `nix-store --realise` subprocess calls that fetch missing store paths from upstream substituters. These are the last subprocess escapes on the build path — the zero-subprocess pipeline already skips them when all inputs are present (previous commit `633a96c40`), but the fallback remains.

Aspen already has all the building blocks to replace this:

- `PathInfoService` stores path metadata (NAR hash, size, references)
- `BlobService` + `DirectoryService` store content-addressed file data
- `SimpleRenderer` (snix-store) renders castore Nodes back to NAR bytes
- `nix_compat::nar::writer` / `nix_compat::nar::reader` for NAR serialization
- `aspen_cache::nar::dump_path_nar` for filesystem → NAR conversion

The missing piece: a function that takes a store path known to PathInfoService, renders its content from BlobService/DirectoryService, and writes it to `/nix/store/<hash>-<name>` on the local filesystem.

## Goals / Non-Goals

**Goals:**

- Eliminate `nix-store --realise` from the native build path entirely
- Materialize missing store paths from Aspen's cluster store (PathInfoService + BlobService + DirectoryService) to local `/nix/store`
- Keep the existing "all present → skip" fast path unchanged
- Gate the old subprocess behind `nix-cli-fallback` feature as a last resort

**Non-Goals:**

- Fetching from upstream binary caches (cache.nixos.org) — that requires HTTP client + narinfo parsing, a separate concern
- Replacing `nix-store --realise` for the full `nix build` subprocess fallback path (#6) — that path already shells out to `nix build` anyway
- Writing a full nix-daemon client — too complex for this scope

## Decisions

### D1: Render NAR from castore, write to filesystem via nix_compat

Use `SimpleRenderer::calculate_nar` to get NAR bytes from a castore Node, then unpack to the target path using `nix_compat::nar::reader`. This reuses existing infrastructure rather than implementing a new castore-to-filesystem converter.

Alternative considered: walk the castore directory tree and write files directly. Rejected because `nix_compat::nar::reader` already handles the full NAR format (files, directories, symlinks, permissions) correctly, and the NAR → filesystem conversion is well-tested in nix-compat.

Actually, `nix_compat::nar::reader` parses NAR format but doesn't write to the filesystem. We need a NAR-to-filesystem unpacker. Since we already have `copy_tree` for filesystem-to-filesystem copies, the simplest approach is to write a `materialize_from_castore` function that walks the castore `Node` tree (using `DirectoryService` to resolve directory digests) and writes files/dirs/symlinks directly. No NAR roundtrip needed.

### D2: Walk castore Node tree directly, skip NAR serialization

For materializing to disk, skip the NAR representation entirely. Walk the `Node` tree:

- `Node::File` → read blob content from `BlobService`, write to file, set permissions
- `Node::Directory` → get children from `DirectoryService`, recurse
- `Node::Symlink` → create symlink

This is more efficient than NAR roundtrip (no serialization/deserialization overhead) and simpler to implement since we already have the castore service handles.

### D3: Fallback chain: castore → nix-store --realise → error

1. Check if path exists on disk (existing fast path)
2. If missing, try materializing from PathInfoService + castore services
3. If castore doesn't have it (not in cluster store), fall back to `nix-store --realise` if `nix-cli-fallback` feature is enabled
4. If all else fails, return error

### D4: New module `materialize.rs` in aspen-ci-executor-nix

Keep the materializer in the executor crate rather than `aspen-snix`, since it's specific to the build pipeline's need to populate `/nix/store`. The function signature:

```rust
pub async fn materialize_store_paths(
    paths: &[String],  // absolute /nix/store paths
    pathinfo_service: &dyn PathInfoService,
    blob_service: &dyn BlobService,
    directory_service: &dyn DirectoryService,
) -> Result<MaterializeReport, io::Error>
```

## Risks / Trade-offs

- **[Permission errors on /nix/store]** → Same as `copy_tree`: log warning, non-fatal since PathInfoService is primary. The materializer is best-effort for local builds, not a requirement for the distributed pipeline.
- **[Large store paths]** → Blob content is streamed, not loaded into memory. Directory trees are walked lazily. Tiger Style bounds: MAX_MATERIALIZE_PATHS = 10,000, MAX_SINGLE_BLOB_SIZE = 256 MB.
- **[Missing content in castore]** → Some paths may exist in PathInfoService metadata but lack actual blob/directory content (e.g., only narinfo imported, not full content). The materializer checks for this and falls back gracefully.
