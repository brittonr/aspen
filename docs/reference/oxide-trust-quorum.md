# Reference: Oxide Trust Quorum

> Source: [oxidecomputer/omicron/trust-quorum](https://github.com/oxidecomputer/omicron/tree/f232730b4e177944d59197edb2b48725adb1aced/trust-quorum)
> License: MPL-2.0
> Commit: `f232730b4e177944d59197edb2b48725adb1aced`

## What It Is

Trust quorum is Oxide's protocol for managing a shared "rack secret" distributed
across physical nodes in a rack using Shamir's Secret Sharing. No single node
holds the full secret — reconstructing it requires a threshold (K of N) of nodes
cooperating. The secret protects at-rest encryption keys and survives
reconfiguration (adding/removing nodes) with proper key rotation.

## Architecture

### Crate Layout

| Crate | Purpose |
|-------|---------|
| `trust-quorum-protocol` | Sans-IO protocol state machine (`Node`, `CoordinatorState`) |
| `trust-quorum-types` | Shared types, versioned via `trust-quorum-types-versions` |
| `gfss` | GF(2^8) Shamir secret sharing (split/reconstruct) |
| `tqdb` | Debugging REPL for replaying protocol state |
| `trust-quorum-test-utils` | Test harness and simulation utilities |
| `tla/` | TLA+ model checking spec |

### Sans-IO / No-IO Design

The protocol is a deterministic state machine with zero I/O:

```rust
pub struct Node {
    coordinator_state: Option<CoordinatorState>,
    key_share_computer: Option<KeyShareComputer>,
    rack_secret_loader: RackSecretLoader,
}
```

All side effects go through the `NodeHandlerCtx` trait:

```rust
pub trait NodeHandlerCtx {
    fn platform_id(&self) -> &BaseboardId;
    fn persistent_state(&self) -> &PersistentState;
    fn update_persistent_state(&mut self, f: impl FnOnce(&mut PersistentState) -> bool);
    fn send(&mut self, to: BaseboardId, msg: PeerMsgKind);
    fn connected(&self) -> &BTreeSet<BaseboardId>;
    fn add_connection(&mut self, peer: BaseboardId);
    fn remove_connection(&mut self, peer: &BaseboardId);
    fn raise_alarm(&mut self, alarm: Alarm);
}
```

Benefits:
- Fully deterministic for testing (swap in an in-memory `NodeCtx`)
- Protocol logic is pure — all networking and persistence is external
- Matches Aspen's "Functional Core, Imperative Shell" philosophy

### Protocol Messages

```rust
enum PeerMsgKind {
    Prepare { config: Configuration, share: Share },  // Coordinator → node
    PrepareAck(Epoch),                                 // Node → coordinator
    GetShare(Epoch),                                   // Any → any
    Share { epoch: Epoch, share: Share },              // Response to GetShare
    Expunged(Epoch),                                   // Node removed from group
    CommitAdvance(Configuration),                      // Catch up stale node
    GetLrtqShare,                                      // Legacy upgrade path
    LrtqShare(LrtqShare),                              // Legacy upgrade path
}
```

### Configuration

Each epoch has a `Configuration`:
- `rack_id: RackUuid` — physical rack identifier
- `epoch: Epoch(u64)` — monotonically increasing version
- `coordinator: BaseboardId` — which node runs this reconfiguration
- `members: BTreeMap<BaseboardId, Sha3_256Digest>` — member → share digest
- `threshold: Threshold(u8)` — K-of-N threshold
- `encrypted_rack_secrets: Option<EncryptedRackSecrets>` — old secrets encrypted with new

### Persistent State

Each node stores:
- `configs: IdOrdMap<Configuration>` — all configurations seen
- `shares: BTreeMap<Epoch, Share>` — own key shares per epoch
- `commits: BTreeSet<Epoch>` — which epochs are committed
- `expunged: Option<ExpungedMetadata>` — permanent removal marker
- `lrtq: Option<LrtqShareData>` — legacy rack trust quorum data

## Reconfiguration Flow

### Initial Configuration (No Prior State)

1. Nexus sends `ReconfigureMsg` to chosen coordinator
2. Coordinator creates new `RackSecret`, splits into shares via Shamir
3. Coordinator saves own config+share, sends `Prepare{config, share}` to each member
4. Members save config+share, reply with `PrepareAck(epoch)`
5. When coordinator collects K+Z acks (threshold + safety margin), Nexus commits
6. Nexus sends `Commit(epoch)` to each member

### Reconfiguration (Existing Configuration)

1. Coordinator receives `ReconfigureMsg` from Nexus
2. **Collect old shares**: Coordinator sends `GetShare(old_epoch)` to old members
3. When enough shares arrive, coordinator **reconstructs old rack secret**
4. If old config had encrypted secrets, **decrypt them** with old rack secret
5. Create new `RackSecret`, split into new shares
6. **Encrypt all old rack secrets** with the new rack secret → `EncryptedRackSecrets`
7. Send `Prepare{config_with_encrypted_secrets, new_share}` to each new member
8. Standard prepare/commit flow continues

This chain means any committed configuration can decrypt ALL prior rack secrets
by working backwards through the `encrypted_rack_secrets` chain.

### Crash Recovery

- Node reboots → enters "Unlock" state → sends `GetShare` to collect shares for last committed config
- If node missed a commit → receives `CommitAdvance(latest_config)` from peer
- If node needs to compute share for config it never got a Prepare for → "ComputeShare" state

### Expungement

When a node is removed from the trust group:
- Receives `Expunged(epoch)` message
- Records `ExpungedMetadata` in persistent state (permanent)
- Stops all coordination and key share computation
- Requires factory reset to rejoin

## Cryptography

| Primitive | Usage |
|-----------|-------|
| GF(2^8) Shamir (`gfss`) | Split rack secret into K-of-N shares |
| SHA3-256 | Share digest verification (config stores expected digests) |
| ChaCha20Poly1305 | Encrypt old rack secrets with new rack secret |
| HKDF-SHA3-256 | Derive encryption key from rack secret + rack_id + epoch |
| Zeroize | Memory safety — secrets are zeroed on drop |
| Constant-time comparison (`subtle`) | Prevent timing side channels on secret data |
| OsRng | Rack secret generation |

Key derivation context: `b"trust-quorum-v1-rack-secrets"` scoped to rack_id + epoch.

## TLA+ Model

The TLA+ spec (`tla/trust_quorum.tla`) models:
- 5 nodes, configurable max epochs and crashes
- Nexus operations: Idle → Preparing → Committing
- Node operations: Uncommitted, Idle, Unlock, CoordinateCollectShares, CoordinatePrepare, ComputeShare, Expunged
- All messages kept in a global set (not consumed on receipt)
- Crash/restart of nodes and Nexus
- Invariants: type safety, config consistency, committed nodes have shares

Key design choices in the spec:
- `CHOOSE` for message receipt (reduces state space vs `\E m \in msgs`)
- Fixed configuration sequences (avoids combinatorial explosion)
- Crash limits per node

## Testing

- Property-based tests via `proptest` for initial configuration, secret split/reconstruct
- `tqdb` REPL for replaying and inspecting protocol state
- `daft` crate for structural diffs of protocol state (diffable derives)
- `danger_partial_eq_ct_wrapper` feature for testing only (constant-time comparison bypass)

## Design Decisions Worth Noting

1. **No-IO is non-negotiable**: The protocol never touches the network or disk directly.
   The test suite runs the exact same `Node` code with an in-memory `NodeCtx`.

2. **Epoch linearization**: Epochs are strictly ordered. If a coordinator crashes
   during preparation, Nexus aborts and starts a new epoch. No Byzantine tolerance
   — the protocol trusts Nexus to linearize epochs (Nexus is the control plane).

3. **Encrypted secret chain**: Each config embeds the prior rack secrets encrypted
   with the current rack secret. A single reconstruction gives access to all
   historical secrets for key rotation / backward compatibility.

4. **Share validation via digest**: Configurations store SHA3 digests of each member's
   share. This prevents a corrupted or malicious share from poisoning reconstruction.

5. **Threshold + safety margin**: Nexus requires K+Z acks before committing (Z=1),
   providing extra safety margin beyond bare threshold.
