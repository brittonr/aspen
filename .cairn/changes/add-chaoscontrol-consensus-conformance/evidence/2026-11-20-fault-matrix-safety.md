# Local fault-matrix and operation-safety slice (2026-11-20)

## Scope

Pure source controls only: declared campaign profiles, operation-identity
safety evaluation, and deliberately faulty local adapters over the pinned
`chaoscontrol-smr` producer contract. This record is not guest, KVM, replay,
linearizability, production or release evidence.

## What changed

- `conformance/profiles.rs` declares the producer no-fault control and the
  bounded message-loss, reorder, partition, quorum-loss, leader-crash,
  follower-crash, restart, and snapshot-catch-up campaigns. Disk corruption,
  clock anomalies, and Byzantine requests are typed denials.
- `conformance/safety.rs` binds committed chain observations to one logical
  operation, one command index, one command identity, and one application
  receipt. An exact restart re-observation is accepted; a second application,
  identity drift, or receipt substitution is a failure. The fail-closed
  importer now consumes this evaluator instead of keeping a second copy of the
  operation/receipt checks.
- `conformance/faulttests.rs` exercises divergent, duplicate-application,
  rollback, and stalled-recovery adapters. Stalled recovery stays
  not-evaluated under an unavailable quorum and is not reclassified as a
  consensus-safety failure.

## Checks

```
cargo fmt --check
cargo test --locked --lib --features chaoscontrol-conformance conformance:: -- --nocapture
cargo clippy --locked --all-targets --features chaoscontrol-conformance -- -D warnings
cairn validate --root . --policy cairn-policy/generated/cairn-policy.json --strict
cairn gate proposal|design|tasks add-chaoscontrol-consensus-conformance --root . --policy cairn-policy/generated/cairn-policy.json
cargo octet check -p molten --artifact-dir target/octet-lib -- --lib
cargo octet check --artifact-dir target/octet
```

Result: 43 conformance tests passed, 0 failed (2202 filtered); Clippy passed
with warnings denied; Cairn validation and the three artifact gates passed.
Both Octet checks were clean (0 findings, 0 warnings) after clearing the
pre-existing example findings. The selection includes the existing wire,
admission, and offline importer tests.

## Open obligations

The live Rust consumer adapter over admitted Molten consensus/application
paths, the measured multi-node guest cohort, actual no-fault and fault
campaigns, replay, and the Cairn validation/gate closeout remain unchecked. No
execution or qualification claim is made here.
