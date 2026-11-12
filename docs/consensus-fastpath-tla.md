# Consensus fast-path TLA+ model

`verification/tla/ConsensusFastPath.tla` is a bounded TLA+ model of the
fast-path hazard contract. TLC checks the finite three-replica, two-command
configuration in `ConsensusFastPath.cfg`.

It is a model, not a live-engine proof. It does not establish external Jetpack
behavior, transport security, durability, timing, production linearizability,
Byzantine tolerance, throughput, or release readiness.

## Modeled boundary

- One canonical command is accepted, committed, applied, and replied at most
  once (set variables; apply and reply require prior commitment/application).
- A fast commit requires a same-view superquorum, promises from every replica,
  and a conflict-free classification.
- The conflict relation is the conservative `d # c`: every distinct command is
  treated as potentially conflicting, so fast commit cannot commit a second
  distinct command after the first.
- A view change resets acknowledgements and promises. A new view requires a
  completed recovery marker.
- Recovery targets `accepted \ committed`, commits those commands through the
  original path, records an explicit marker, and cannot lose accepted commands.
- Mixed-view acknowledgements cannot satisfy a fast commit because every
  superquorum acknowledgement must name the current acceleration view.

## Invariants

- `CommittedAccepted`: committed commands were accepted.
- `AppliedCommitted`: applied commands were committed.
- `RepliedApplied`: replies follow application.
- `RecoveryPreserves`: recovery preserves every accepted-uncommitted command.
- `RecoveryBeforeNewView`: a normal view after the first requires a marker.
- `NoMixedViewCommit`: no acknowledgement names a future view.
- `NoFastConflict`: fast-committed commands are pairwise conflict-free.

## Run

```sh
tlc -metadir /tmp/tlc-fastpath -config verification/tla/ConsensusFastPath.cfg \
  verification/tla/ConsensusFastPath.tla
```

The flake check is `checks.x86_64-linux.consensus-fastpath-tla` and uses
TLA+ 1.7.4 / TLC 2.19.

TLA-ts is not integrated in this repository yet; the check is TLC-only. A
TLA-ts parser check should be added when a pinned package is available.
