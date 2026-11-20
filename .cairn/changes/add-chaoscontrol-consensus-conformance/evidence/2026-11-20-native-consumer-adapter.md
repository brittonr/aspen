# Native consumer adapter over the real Raft and application paths (2026-11-20)

## Scope

A Rust adapter in `fabric_consistency::raft::live_cluster` drives the actual
three-replica in-process Raft service and application handler, then projects
committed applies through `chaoscontrol::conformance::wire`. This is native
in-process evidence; it is not guest, KVM, campaign, replay, or production
qualification.

## Implementation

- `ApplicationHandler::state_ref` derives a deterministic application-state ref
  from the actual applied request sequence.
- `workflow::replicate_request_observed` performs the real propose, append,
  commit-notice and apply path and retains each committed replica's command
  index, application receipt, and application-state ref.
- `live_cluster::conformance_adapter` builds an admitted SMR workload plan and
  protocol profile, binds one chain projector per participant, and encodes the
  committed observations with `wire::project`.
- The committed quorum (leader and one follower) is observed. The third
  replica's commit message is not applied in this bounded adapter, so it is
  retained as a lagging replica rather than fabricated as an observation.

## Behavior verified

For two commands:

- four committed observations were produced (leader and follower per index);
- both replicas had equal chain digests and equal application-state refs at
  each index;
- the Raft application port returned one deterministic batch receipt per
  command, shared by the applying replicas;
- `safety::evaluate` returned `Pass` with no divergence, rollback, duplicate
  application, or receipt-reuse violation.

The safety rule now treats the same receipt at the same command index and
logical operation as a legitimate deterministic batch identity shared by
replicas. Reuse across a different index or operation remains a violation.

## Checks

```
cargo test --locked --lib --features chaoscontrol-conformance native_cluster_adapter
  -> 1 passed
cargo test --locked --lib --features chaoscontrol-conformance conformance::
  -> 48 passed
```

## Non-claims

- in-process three-replica native execution only;
- the lagging third replica is not observed in this adapter test;
- no guest image, KVM, multi-VM, campaign, saved replay, production admission,
  cluster availability, security, or release eligibility is claimed.
