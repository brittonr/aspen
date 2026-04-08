# TLA+ Specifications

Formal specifications for Aspen's trust protocol using TLA+ model checking.

## Specs

| File | What | Status |
|------|------|--------|
| `trust_init.tla` | Cluster secret creation, share distribution | ✅ Verified |
| `trust_reconfig.tla` | Reconfiguration with share collection, epoch rotation | Planned |
| `trust_expunge.tla` | Expungement message delivery, peer enforcement | Planned |

## Running

```bash
# Verify all specs
nix run .#check-tla

# Verify a single spec
java -cp $(nix eval --raw nixpkgs#tlaplus)/share/java/tla2tools.jar tlc2.TLC trust_init.tla -config trust_init.cfg
```

## Conventions

- **Variables**: lowercase with underscores (`node_state`, `msg_set`)
- **Constants**: ALLCAPS (`N`, `K`, `NODES`)
- **Actions**: PascalCase (`LeaderDistributeShares`)
- **Invariants**: PascalCase prefixed with `Inv` (`InvAllNodesHaveShares`)
- **Module names**: snake_case matching file names

## Spec-to-Code Mapping

| TLA+ Variable | Rust Struct/Field |
|---------------|-------------------|
| `node_state[n].share` | `SharedRedbStorage::load_share(epoch)` |
| `node_state[n].expunged` | `SharedRedbStorage::is_expunged()` |
| `msgs` | Iroh QUIC message set (trust ALPN) |
| `leader` | Raft leader (from `openraft::Membership`) |
| `epoch` | Trust config epoch counter |

| TLA+ Action | Rust Method |
|-------------|-------------|
| `LeaderCreateSecret` | `RaftNode::init_trust()` |
| `LeaderDistributeShares` | Applied via Raft log entry |
| `NodeReceiveShare` | `SharedRedbStorage::store_share()` |
| `MarkExpunged` | `SharedRedbStorage::mark_expunged()` |
