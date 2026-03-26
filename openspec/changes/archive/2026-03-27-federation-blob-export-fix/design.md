## Context

The git bridge stores `SignedObject<GitObject>` in iroh-blobs via `add_bytes`, then the exporter reads them back via `get_bytes` using the BLAKE3 hash. iroh-blobs' `FsStore` uses bao (BLAKE3 Authenticated Output) encoding internally. On QEMU virtio disks, the bao tree is systematically corrupt on read-after-write — `get_bytes` returns `encode error` even though `add_bytes` succeeded moments before. Retries don't help. The in-memory blob store works fine, proving the hash mapping is correct.

## Goals / Non-Goals

**Goals:**

- Git object export works reliably regardless of iroh-blobs FsStore behavior
- Federation git clone produces a working tree with file content
- No regression in P2P blob replication between cluster nodes

**Non-Goals:**

- Fixing iroh-blobs FsStore bao encoding (upstream issue)
- Removing iroh-blobs from the architecture (still needed for P2P replication)

## Decisions

### D1: Store serialized git objects in redb KV alongside iroh-blobs

On import, store the serialized `SignedObject` bytes in KV at `forge:obj:{repo_id}:{blake3_hex}`. On export, read from KV instead of iroh-blobs. iroh-blobs `add_bytes` still happens for P2P replication but is no longer on the critical read path.

redb is Raft-replicated, single-fsync, and the most reliable storage layer. Object size is bounded by `MAX_GIT_OBJECT_SIZE` (1MB), fitting within `MAX_VALUE_SIZE`.

**Why not just fix iroh-blobs?** iroh-blobs 0.99.0 is from crates.io, not vendored. The bao corruption appears to be a filesystem interaction bug that would require deep investigation in upstream code.

### D2: KV read with iroh-blobs fallback

Exporter tries KV first (`forge:obj:{repo}:{b3}`). If not found (old data imported before this change), falls back to iroh-blobs `get_bytes`. New imports always write both.

## Risks / Trade-offs

- **[Storage duplication]** Git objects stored in both KV and iroh-blobs. Acceptable — objects are small (bounded by MAX_GIT_OBJECT_SIZE), and KV is the authoritative store.
- **[Migration]** Old repos imported before this change won't have KV objects. The iroh-blobs fallback handles this — those objects may still fail with encode error, but new imports are fixed.
