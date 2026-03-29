## Context

The federation sync protocol transfers git objects between clusters. Each cluster stores git objects in a Forge-internal format (`SignedObject<GitObject>`) keyed by BLAKE3 envelope hashes. The c2e index (`forge:c2e:{repo}:{key}` → `envelope_hash_hex`) exists to support incremental sync — mapping a wire-visible identifier to the local envelope hash so the exporter can skip objects the remote already has.

Three hash domains are in play:

- **SHA-1** (20 bytes): Canonical git object identity. Deterministic from git bytes.
- **Content hash** (32 bytes): `blake3(git_content_without_header)`. Depends on exact byte representation.
- **Envelope BLAKE3** (32 bytes): `blake3(SignedObject serialization)`. Depends on signing key, HLC timestamp, and content.

Current state: the c2e index is keyed by content hash, but two bugs make it non-functional:

1. The remote's `have_hashes` are envelope BLAKE3 hashes, not content hashes (hash domain confusion)
2. Content hashes differ between import and export paths (tree sort divergence)

## Goals / Non-Goals

**Goals:**

- c2e index correctly maps wire identifiers to local envelope hashes
- Incremental federation sync skips objects the remote already has
- Tree objects round-trip with identical bytes (correct SHA-1 on export)
- Commit messages round-trip without losing trailing newline edge cases

**Non-Goals:**

- Wire protocol changes (SyncObject.hash stays as blake3 for content verification)
- SHA-256 git support (SHA-1 is sufficient for current git ecosystem)
- Rewriting the federation sync batching/pagination logic

## Decisions

### D1: Key c2e index by SHA-1 hex instead of content hash

SHA-1 is the only hash that is guaranteed identical on both sides by construction. Git correctness requires it. Content hashes depend on byte-exact serialization, which fails when the internal format normalizes (tree sort, commit trailing newlines).

**KV key format**: `forge:c2e:{repo_hex}:{sha1_hex}` → `envelope_blake3_hex`

The importer already has the SHA-1 available (`import_object_store_blob` returns it). The exporter already computes SHA-1 via `converter.export_object()` (currently stored in `_sha1`).

**Alternative considered**: Fix serialization to be byte-identical. This is also needed (see D3) but insufficient as a standalone fix — future normalization changes would silently break c2e again. SHA-1 keying is robust against serialization drift.

### D2: Fix have_set to use SHA-1 domain

`collect_local_blake3_hashes()` currently scans `forge:hashmap:b3:{repo}:` which returns envelope BLAKE3 hashes. Change it to scan `forge:hashmap:sha1:{repo}:` which returns SHA-1 hashes. These are 20 bytes, but the have_set is `HashSet<[u8; 32]>` — zero-pad SHA-1 to 32 bytes for the existing wire format.

On the export side, convert `have_set` entries (SHA-1 padded to 32 bytes) back to 20-byte SHA-1 for c2e lookup. Or: change the federation sync protocol's `have_hashes` field to use `[u8; 20]` for git objects. The simpler path is to just send SHA-1 hex in a new field or zero-pad.

**Chosen approach**: Keep `have_hashes` as `Vec<[u8; 32]>` on the wire but populate with `sha1_padded_to_32`. The exporter truncates to 20 bytes for c2e lookup. This avoids a wire format change while fixing the domain confusion.

### D3: Fix TreeObject sort to match git

Git's tree sort uses `base_name_compare` which appends `/` to directory names for comparison:

```
foo (dir)  → compared as "foo/"
foo.c (file) → compared as "foo.c"
```

This means directories sort differently than files with similar name prefixes. Rust's `str::cmp` doesn't account for mode.

Fix `TreeObject::new()` to sort using git's rule: compare entries by name, but for directory entries (mode `040000`), append `/` during comparison. This matches the behavior documented in git's `tree-diff.c`.

This is required for correctness independent of c2e — without it, exported trees have wrong SHA-1 hashes, which breaks git clone from federation mirrors.

### D4: Preserve commit message bytes through round-trip

The import path uses `lines.collect::<Vec<_>>().join("\n")` which drops the trailing newline. The export path conditionally adds one back. This works for standard commits but fails for edge cases (messages ending in multiple newlines, messages with no trailing newline).

Fix: use `splitn(2, "\n\n")` to separate headers from message body, preserving the message as-is. Or index into the raw bytes after the blank-line separator.

## Risks / Trade-offs

**[Risk] Existing c2e entries become stale** → Stale entries harmlessly miss on lookup (SHA-1 key won't match old content-hash key). No migration needed — new imports write correct keys, old entries are orphaned and eventually overwritten or ignored.

**[Risk] TreeObject sort change affects merge logic** → `git/merge.rs` creates TreeObjects with `TreeObject::new()`. The merge logic compares entries by name, which still works. The only change is that the resulting tree's entry order matches git's sort, which is the correct behavior for merge results too.

**[Risk] have_set zero-padding is fragile** → If SHA-1 is 20 bytes zero-padded to 32, any code that treats it as a full BLAKE3 will fail. Mitigated by documenting the encoding and adding a helper function `sha1_to_have_hash([u8; 20]) -> [u8; 32]`.
