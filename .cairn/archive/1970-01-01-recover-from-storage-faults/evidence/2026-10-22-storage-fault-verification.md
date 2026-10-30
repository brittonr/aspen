# Recover from storage faults: baseline and verification

## Baseline before the change

Focused suites before any edit:

- `cargo test -p molten-core --lib -- fabric_durability`: 21 passed.
- `cargo test -p molten --lib -- fabric_durability`: 9 passed.
- `cargo test -p molten --lib -- fabric_consistency::raft`: 48 passed.

Recorded behavior:

- **Durability labels.** `MutationOutcome::Durable` was asserted after any
  `Ok` Redb commit (`persist_log`, `replace_log`, `persist_ordered_batch`,
  `persist_snapshot`, `persist_effect`). A commit error returned
  `Failure::invalid_harness` and the adapter kept serving, with its
  in-memory state possibly disagreeing with storage.
- **Snapshot ordering.** `persist_snapshot` wrote the payload with
  `root.write` (no file or parent synchronization), then committed metadata.
- **Repair behavior.** No Redb repair callback. Nothing recorded that a store
  was repaired at open.
- **Consensus caveats.** `plan_replica` replays local durable records at
  startup with no protocol-aware repair. Voting is gated only by the external
  `production_admitted` flag.

## After the change

- Core: 42 passed (`fabric_durability` 28, `consensus_recovery` 14), covering:
  - every persistence observation and level;
  - sticky quarantine;
  - status- and reopen-only admission;
  - reconciliation by reference, by pending state, and by self-consistency;
  - snapshot metadata ordering;
  - entry repair: local-only, fetch, replace, quorum removal, committed
    protection, unknown wait, conflicting history, outsider reports;
  - effect plans;
  - hard-state fencing;
  - snapshot repair, including the conflicting-content negative;
  - recovery progress: local sufficiency, an unnecessary remote repair, a
    missed available item, horizon, an unstable window, global absence, and
    an incomplete observation or drain;
  - repair admission, including a non-voting role after vote loss;
  - receipt cohort binding: missing, changed, overclaim, and tamper.
- Shell: 119 passed (`fabric_consistency` and `fabric_durability`). New tests
  run over a simulated medium that separates synchronized bytes from
  process-visible bytes:
  - a synchronized commit survives power loss;
  - a failed synchronization quarantines, refuses reads, and is not cleared by
    a successful retry;
  - a reconciling reopen while synchronization still fails never serves;
  - reconciliation after a retained-cache read-back rewrites and re-syncs, and
    the result survives power loss;
  - a write the medium lost reconciles to the acknowledged state;
  - a repaired store opens quarantined, needs a reference or self-consistency,
    and stays recorded as repaired;
  - a corrupt store never serves;
  - snapshot publication: payload then parent synchronization then metadata;
  - a partial publication commits nothing;
  - a failed parent synchronization withholds metadata until reconciliation,
    and the retry verifies the existing destination;
  - a mismatched payload quarantines, at creation and at startup;
  - a repaired replica store reports no rejoin role, even after storage
    reconciliation.

  The all-nodes crash restart test (`three_elects_commits_reads_and_catches_up`)
  passes: its recover-mode startup reconciles each repaired store explicitly by
  self-consistency before replaying it.

- Dependency: `durable-file-publication` 0.1.0 is pinned at
  `8e05e74e24b45f752d77145c4455385daaf6d6ab` from
  `https://git.onix.computer/z3tAR4For7qw8ZirkJzoDw1VNDDLM.git`
  (MIT OR Apache-2.0) in `Cargo.toml`, `Cargo.lock`, the release dependency
  profile, `flake.nix` (input, source identity assertion, local git source),
  `flake.lock`, `crate-hashes.json`, and both unit2nix plans. The NAR hash
  `sha256-kd74vui4byAC32OnH7Wn1sw9IW4PWjSw/VThvdnd380=` was taken from the
  flake input locked at the same revision, the script's own fallback. It
  matches an independent `nix-prefetch-git`. A full
  `scripts/git-source-hashes.sh --write` failed on an unrelated pinned
  source: `seed.radicle.garden` returned 504. `scripts/git-source-hashes.sh
  --check` passes. The regenerated plans also change `workspaceRoot` and the
  path IDs, because they were last generated in a worktree.

## Open

- **Voting-readiness wiring.** The user chose to defer it. Gating
  `plan_recovery` on repair admission would leave a cluster whose every node
  crashed unable to re-elect, because no live peer-query recovery path exists
  to reconcile from. `RedbReplicaPort::storage_admission` reads the admission
  back; startup recovery does not consult it. The pure repair decisions and
  effect plans are implemented; their execution over the replica transport is
  the remaining work.
- **ChaosControl campaigns.** These need ChaosControl's
  `model-guest-storage-flush-failures`: guest flush outcomes, cache and
  reboot profiles, and persistent-Raft workloads. That change has every task
  open.

## Non-claims

These results bind the simulated medium, the Linux publication adapter, and
the pinned revisions above. They do not prove that Redb, a filesystem, or
hardware cannot lose data. They do not make direct I/O a universal
requirement, treat checksums as proof of an authoritative replica, or treat
local repair as Raft repair. They establish no whole-cluster safety,
liveness, or release readiness.

## Gates run

- `cargo test --workspace --lib`: 1721 (`molten`) and 572 (`molten-core`)
  passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo octet
  check`: clean.
- `nix build .#checks.x86_64-linux.release-dependency-profile
  .#checks.x86_64-linux.git-source-hash-binding`: built.
  `scripts/git-source-hashes.sh --check`: passed.
- Cairn strict validation: no spec or substance findings. Tracey guard: pass
  (2938 requirements, 1004 referenced, 1934 inherited uncovered, 0 dangling).
