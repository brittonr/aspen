## Context

Git tree objects contain entries with a mode, name, and 20-byte SHA-1 hash. Mode `160000` denotes a gitlink — a reference to a commit in an external submodule repository. Unlike blobs and subtrees, the referenced commit doesn't exist in the current repository's object store.

Aspen Forge stores git objects internally using BLAKE3 hashes. During import, each tree entry's SHA-1 is translated to a BLAKE3 hash via a mapping store. Gitlink entries can't be translated because the referenced commit isn't local, so the importer was skipping them entirely.

The `TreeEntry.hash` field is `[u8; 32]` (BLAKE3 sized). Gitlink SHA-1 hashes are 20 bytes.

## Goals / Non-Goals

**Goals:**

- Byte-identical tree round-trip through import → Forge storage → export for trees containing gitlinks
- No disruption to existing non-gitlink tree handling
- DAG traversal correctly skips gitlink references (external commits)

**Non-Goals:**

- Cloning or resolving submodule content
- Supporting recursive submodule operations in Forge
- Migration of previously-imported repos (re-push required)

## Decisions

**Store raw SHA-1 zero-padded to 32 bytes.** Gitlink entries store the original 20-byte SHA-1 in the first 20 bytes of the 32-byte `hash` field, with the remaining 12 bytes zeroed. This avoids changing the `TreeEntry` struct layout or adding a union type. The mode field (`0o160000`) distinguishes gitlinks from regular entries.

**Export uses stored SHA-1 directly.** During export, gitlink entries bypass the BLAKE3→SHA-1 mapping lookup. The 20-byte SHA-1 is extracted from `hash[..20]` and written to the tree content.

**DAG walk skips gitlink entries.** Both `export_commit_dag_queue_deps` and `get_object_deps` filter out gitlink entries, since the referenced commits are external and can't be resolved.

**Sort order unchanged.** Git's `base_name_compare` uses `S_ISDIR()` which is false for mode 160000. Gitlinks sort like regular files (NUL-terminated), not directories (`/`-terminated).

## Risks / Trade-offs

**Zero-padding ambiguity.** A BLAKE3 hash that happens to end with 12 zero bytes would be indistinguishable from a zero-padded SHA-1. In practice this is astronomically unlikely (2^-96), and the mode field provides disambiguation.

**No migration path.** Repos imported before this fix have trees missing gitlink entries. They must be re-pushed to get correct round-trip. This is acceptable since federation clone was broken for these repos anyway.
