# Pijul Integration for Aspen

## Summary

Pijul is embedded into Aspen as a native VCS format, separate from Git. Both use BLAKE3 for hashing, enabling zero-overhead hash sharing.

**Key Decisions**:
- GPL-2.0-or-later license for Aspen to enable direct libpijul embedding
- Shared COB system (issues, patches, reviews) for both Git and Pijul repos
- Configurable working directory layout (standard `.pijul/` or Aspen-managed)
- Aspen CLI/API is the primary interface; pijul CLI remote helper is optional future work

## What Aspen Adds to Pijul

| Layer | Vanilla Pijul | With Aspen |
|-------|---------------|------------|
| Change Storage | Local filesystem | iroh-blobs: P2P, NAT traversal, content-addressed |
| Ref/Channel Storage | Local sanakirja | Raft: strongly consistent across cluster |
| Networking | SSH/HTTP remotes | QUIC P2P with mDNS/DNS/DHT discovery |
| Replication | Manual push/pull | Automatic sync, gossip announcements |
| Collaboration | None | COBs: issues, patches, reviews |
| Identity | name/email | Ed25519 keys, delegate multisig |

**Use Cases**: P2P code hosting, cluster storage, local with sync to cluster.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        PIJUL MODULE                               │
│  src/pijul/                                                       │
│  ┌────────────────┐  ┌─────────────────┐  ┌──────────────────┐  │
│  │ PristineStore  │  │ AspenChangeStore│  │ PijulRefStore    │  │
│  │ (sanakirja)    │  │ (ChangeStore)   │  │ (Raft KV)        │  │
│  │ node-local     │  │ wraps BlobStore │  │ channel heads    │  │
│  └────────────────┘  └─────────────────┘  └──────────────────┘  │
│          │                   │                    │              │
│          ▼                   ▼                    ▼              │
│    .pijul/pristine/    iroh-blobs          Raft consensus       │
│    (sanakirja DB)      (P2P ready)         (strongly consistent)│
└──────────────────────────────────────────────────────────────────┘
```

**Storage Strategy**:
- Sanakirja for pristine state (libpijul's internal file graph)
- iroh-blobs for changes (P2P distributable)
- Raft KV for channel heads (strongly consistent)

## Module Structure

```
src/pijul/
├── mod.rs              # Module root, feature gate, re-exports
├── constants.rs        # Tiger Style resource limits
├── error.rs            # PijulError enum (snafu-based)
├── types.rs            # ChangeHash, Channel, PijulRepoIdentity, etc.
├── change_store.rs     # AspenChangeStore - iroh-blobs backed storage
├── refs.rs             # PijulRefStore - Raft KV backed channel heads
├── store.rs            # PijulStore - high-level coordinator
├── pristine.rs         # [Phase 2] Sanakirja pristine wrapper
├── sync.rs             # [Phase 2] P2P sync of changes
└── gossip.rs           # [Phase 2] PijulAnnouncement types
```

## Key Types

### ChangeHash

```rust
/// Pijul change hash (BLAKE3, same as Aspen/iroh-blobs)
pub struct ChangeHash(pub [u8; 32]);

impl ChangeHash {
    // Zero-copy conversion to iroh_blobs::Hash
    pub fn to_iroh_hash(&self) -> iroh_blobs::Hash {
        iroh_blobs::Hash::from_bytes(self.0)
    }
}
```

### PijulRepoIdentity

```rust
pub struct PijulRepoIdentity {
    pub name: String,
    pub description: Option<String>,
    pub default_channel: String,
    pub delegates: Vec<iroh::PublicKey>,
    pub threshold: u32,
    pub created_at_ms: u64,
}
```

### PijulStore

```rust
pub struct PijulStore<B: BlobStore, K: KeyValueStore + ?Sized> {
    changes: Arc<AspenChangeStore<B>>,
    refs: Arc<PijulRefStore<K>>,
    kv: Arc<K>,
    data_dir: PathBuf,
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> PijulStore<B, K> {
    pub async fn create_repo(&self, identity: PijulRepoIdentity) -> PijulResult<RepoId>;
    pub async fn store_change(&self, repo_id: &RepoId, channel: &str,
        change_bytes: &[u8], metadata: ChangeMetadata) -> PijulResult<ChangeHash>;
    pub async fn get_change(&self, hash: &ChangeHash) -> PijulResult<Option<Vec<u8>>>;
    pub async fn list_channels(&self, repo_id: &RepoId) -> PijulResult<Vec<Channel>>;
}
```

## Usage

```rust
use aspen::pijul::{PijulStore, PijulRepoIdentity};

// Create a Pijul store
let store = PijulStore::new(blob_store, kv_store, data_dir);

// Create a new repository
let identity = PijulRepoIdentity::new("my-project", delegates);
let repo_id = store.create_repo(identity).await?;

// Store a change (pre-serialized in libpijul format)
let metadata = ChangeMetadata { /* ... */ };
let change_hash = store.store_change(&repo_id, "main", &change_bytes, metadata).await?;

// Get channel head
let channel = store.get_channel(&repo_id, "main").await?;
```

## Feature Flag

Enable with `--features pijul`:

```toml
[features]
pijul = ["forge", "dep:libpijul", "dep:sanakirja", "dep:zstd-seekable", "dep:lru"]
```

## Implementation Status

### Phase 1 (Complete)
- [x] License change to GPL-2.0-or-later
- [x] libpijul, sanakirja, zstd-seekable dependencies
- [x] Core types (ChangeHash, Channel, PijulRepoIdentity)
- [x] AspenChangeStore (iroh-blobs backed)
- [x] PijulRefStore (Raft KV backed)
- [x] PijulStore coordinator
- [x] Unit tests (10 passing)

### Phase 2 (Planned)
- [ ] PristineManager (sanakirja integration)
- [ ] Full libpijul Change serialization/deserialization
- [ ] PijulSyncService (P2P sync)
- [ ] PijulAnnouncement gossip types
- [ ] Integration tests

## Dependencies

- libpijul 1.0.0-beta.10 (GPL-2.0-or-later)
- sanakirja 1.4
- zstd-seekable 0.1

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| libpijul API instability (beta) | Pin exact version, consider vendoring |
| Sanakirja incompatible with madsim | Skip sanakirja tests in simulation |
| ed25519-dalek version conflict | Feature-gate or adapter layer |
| Pristine rebuild time | Parallel change application, checkpoints |

## References

- [Pijul Manual](https://pijul.org/manual/)
- [Pijul Model](https://pijul.org/model/)
- [libpijul docs](https://docs.rs/libpijul/)
- [Sanakirja](https://pijul.org/posts/2024-02-13-mainframe/)
