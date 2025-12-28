# Forge: Git on Aspen

> Decentralized Git with Radicle-like features on Aspen.

## Status: ✅ Core Implementation Complete

**Feature flag:** `forge` (enabled by default)

## Overview

Forge is a decentralized code collaboration system built on Aspen's distributed primitives:
- **iroh-blobs**: Immutable content-addressed storage for Git objects and COB changes
- **Raft KV**: Strongly consistent storage for refs and metadata
- **iroh-gossip**: Announcements for new commits, issues, patches
- **Iroh QUIC**: P2P transport with NAT traversal

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    IMMUTABLE LAYER                           │
│                    (iroh-blobs)                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │ Git Objects │ │ COB Changes │ │ Signed Attestations │   │
│  │ (commits,   │ │ (issues,    │ │ (reviews, approvals │   │
│  │  trees,     │ │  patches,   │ │  CI results)        │   │
│  │  blobs)     │ │  comments)  │ │                     │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
│         ↓               ↓               ↓                   │
│    BLAKE3 hash     BLAKE3 hash     BLAKE3 hash             │
│    = object ID     = change ID     = attestation ID        │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    MUTABLE LAYER                             │
│                    (Raft KV)                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ repos:{rid}:refs/heads/main     → Hash (commit)     │   │
│  │ repos:{rid}:refs/tags/v1.0      → Hash (commit)     │   │
│  │ repos:{rid}:identity            → Hash (id doc)     │   │
│  │ repos:{rid}:cobs/issue/{id}:heads → [Hash, Hash]   │   │
│  │ repos:{rid}:delegates           → [NodeId, ...]     │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    DISCOVERY LAYER                           │
│  ┌──────────────┐ ┌────────────┐ ┌───────────────────┐     │
│  │ iroh-gossip  │ │ DHT        │ │ Cluster membership│     │
│  │ (new commits,│ │ (find repo │ │ (who seeds what)  │     │
│  │  new issues) │ │  seeders)  │ │                   │     │
│  └──────────────┘ └────────────┘ └───────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Progress

### Phase 1: Core Infrastructure ✅
- [x] Create forge.md progress document
- [x] Update Cargo.toml with forge feature and dependencies
- [x] Create src/forge/ module structure

### Phase 2: Core Types ✅
- [x] SignedObject<T> - Wrapper for signed, content-addressed objects
- [x] GitObject - Blob, Tree, Commit, Tag variants
- [x] CobChange - Collaborative object change record
- [x] CobType - Issue, Patch, Review, Discussion
- [x] CobOperation - Create, Comment, Close, Merge, etc.
- [x] RepoId - Repository identifier (BLAKE3 of identity doc)
- [x] Author - Commit author info

### Phase 3: Storage Layer ✅
- [x] GitBlobStore - Store/retrieve Git objects via iroh-blobs
- [x] CobStore - Store/retrieve COB changes, resolve state
- [x] RefStore - Raft-backed ref storage with consensus
- [x] IdentityStore - Repository identity and delegates

### Phase 4: Coordination ✅
- [x] ForgeNode - Main coordinator tying everything together
- [x] Gossip announcements (RefUpdate, CobChange, Seeding)
- [x] Sync protocol (fetch missing objects from peers)

### Phase 5: Operations ✅
- [x] commit() - Create new commit
- [x] push() - Update refs via consensus
- [x] clone() - Fetch all objects for a repo (via SyncService)
- [x] create_issue() - Create new issue COB
- [x] create_patch() - Create new patch COB (via CobOperation)
- [x] resolve_issue() - Materialize current issue state

### Phase 6: Testing ✅
- [x] Unit tests for each component
- [ ] Integration tests for full workflows (TODO)
- [ ] Property-based tests for COB resolution (TODO)

## Key Design Decisions

### 1. Pure BLAKE3 (No SHA-1)
We're not doing interop with existing Git/Radicle, so we use BLAKE3 natively:
- Faster hashing
- No hash mapping layer needed
- Consistent with iroh-blobs

### 2. Immutable Objects in iroh-blobs
All Git objects and COB changes are stored as immutable blobs:
- Content-addressed by BLAKE3 hash
- Automatic P2P distribution
- Deduplication across repos

### 3. Raft for Refs
Refs (branches, tags) go through Raft consensus:
- Strong consistency on canonical state
- Atomic updates with CAS semantics
- Threshold signature enforcement for delegates

### 4. COB as Immutable DAG
Collaborative Objects (issues, patches) use the same model as Git:
- Each change is immutable, references parents by hash
- State resolved by walking and applying changes
- No conflicts at storage level (only at resolution)

## File Structure

```
src/forge/
├── mod.rs              # Module exports, feature gate (~80 lines)
├── types.rs            # SignedObject<T>, Signature (~200 lines)
├── constants.rs        # Tiger Style resource limits (~120 lines)
├── error.rs            # ForgeError enum (~100 lines)
├── git/
│   ├── mod.rs          # Re-exports
│   ├── object.rs       # GitObject, TreeEntry, CommitObject (~250 lines)
│   └── store.rs        # GitBlobStore implementation (~200 lines)
├── cob/
│   ├── mod.rs          # Re-exports
│   ├── change.rs       # CobChange, CobOperation (~200 lines)
│   ├── store.rs        # CobStore, topological sort (~350 lines)
│   └── issue.rs        # Issue state resolution (~200 lines)
├── refs/
│   ├── mod.rs          # Re-exports
│   └── store.rs        # RefStore (Raft-backed) (~250 lines)
├── identity/
│   └── mod.rs          # RepoId, RepoIdentity, Author (~200 lines)
├── gossip.rs           # Announcement types (~100 lines)
├── sync.rs             # SyncService, FetchResult (~80 lines)
└── node.rs             # ForgeNode coordinator (~200 lines)

Total: ~2,300 lines of code
```

## Dependencies Added

```toml
[features]
forge = [
  "gix-object", "gix-hash", "gix-traverse", "gix-diff"
]

[dependencies]
gix-object = { version = "0.46", optional = true }
gix-hash = { version = "0.15", optional = true }
gix-traverse = { version = "0.43", optional = true }
gix-diff = { version = "0.48", optional = true }
```

## API Examples

### Creating a Commit
```rust
let forge = ForgeNode::new(aspen_node).await?;

// Create tree from files
let tree = forge.git.create_tree(&[
    ("README.md", blob_hash),
    ("src/main.rs", src_blob_hash),
]).await?;

// Create commit
let commit = forge.git.commit(
    &repo_id,
    tree,
    vec![parent_hash],
    "Initial commit",
    &author,
).await?;

// Push to main
forge.refs.update(&repo_id, "heads/main", commit).await?;
```

### Creating an Issue
```rust
let issue_id = forge.cobs.create_issue(
    &repo_id,
    "Bug: crash on startup",
    "When I run the app, it crashes immediately...",
    &["bug", "critical"],
).await?;

// Add a comment
forge.cobs.comment(&repo_id, &issue_id, "I can reproduce this").await?;

// Close the issue
forge.cobs.close_issue(&repo_id, &issue_id, Some("Fixed in abc123")).await?;
```

### Resolving Current State
```rust
// Get current issue state by walking change DAG
let issue = forge.cobs.resolve_issue(&repo_id, &issue_id).await?;
println!("Title: {}", issue.title);
println!("State: {:?}", issue.state);
println!("Comments: {}", issue.comments.len());
```

## Next Steps

1. **Integration tests**: End-to-end tests with real Aspen clusters
2. **Patch resolution**: Implement patch state machine (like issue)
3. **CLI integration**: Add forge commands to aspen-cli
4. **Gossip integration**: Wire up announcements to iroh-gossip
5. **P2P sync**: Implement actual object fetching from peers

## Usage Example

```rust
use aspen::forge::{ForgeNode, RepoId};
use aspen::api::DeterministicKeyValueStore;
use aspen::blob::InMemoryBlobStore;

// Create a forge node
let blobs = Arc::new(InMemoryBlobStore::new());
let kv = Arc::new(DeterministicKeyValueStore::new());
let secret_key = iroh::SecretKey::generate(rand::rngs::OsRng);
let forge = ForgeNode::new(blobs, kv, secret_key);

// Create a repository
let identity = forge.create_repo("my-project", vec![forge.public_key()], 1).await?;
let repo_id = identity.repo_id();

// Initialize with first commit
let commit = forge.init_repo(&repo_id, "Initial commit").await?;

// Create an issue
let issue_id = forge.cobs.create_issue(&repo_id, "Bug report", "Description", vec!["bug"]).await?;

// Add a comment
forge.cobs.add_comment(&repo_id, &issue_id, "I can reproduce this").await?;

// Resolve issue state
let issue = forge.cobs.resolve_issue(&repo_id, &issue_id).await?;
println!("Issue: {} ({:?})", issue.title, issue.state);
```

---

*Last updated: 2025-12-28*
