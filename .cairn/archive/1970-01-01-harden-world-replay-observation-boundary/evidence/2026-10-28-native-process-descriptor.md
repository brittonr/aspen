# ChaosControl native-process descriptor binding — 2026-10-28

## Producer

ChaosControl revision `bdb82e9f7a2d2ed5f5f81a9554cc88220de33f64` publishes
`chaoscontrol.native-process-replay-profile.v1`,
`chaoscontrol.native-process-descriptor.v1`, and
`chaoscontrol.native-process-record.v1` with canonical BLAKE3 identity and
fail-closed admission.

## Consumer

`crates/molten-core/src/world_replay/boundary/observation.rs` adds
`OpaqueReplayDiagnostic::native_process: Option<NativeProcessDiagnostic>`.
The binding retains only producer identity and declared shape: profile ref,
descriptor ref, producer cohort, replay class, record count, and non-claims.
Molten never parses records or derives observations from them.

`completeness.rs` rejects a binding whose descriptor ref differs from the
opaque diagnostic ref or is not a BLAKE3 reference, whose profile ref is not a
BLAKE3 reference, whose producer cohort differs from the admitted opaque
cohort, whose record count is zero or above 64, or whose non-claims are empty,
above eight, or text-unbounded. The descriptor still satisfies no observation
slot, contributes no semantic source, and cannot overclaim equivalence,
authority, repair, or voting.

## Executed

```sh
nix develop -c cargo test -p molten-core --lib world_replay::boundary -- --test-threads 1
nix develop -c cargo fmt --check
nix develop -c cargo clippy -p molten-core --all-targets -- -D warnings
```

Boundary suite: 15 passed including the positive binding and the swapped
descriptor, foreign cohort, zero-record, and missing-non-claim denials.

## Non-claims

The binding proves only exact producer and descriptor identity plus internal
shape. It does not reproduce a run, prove process behaviour, satisfy a missing
observation, become a logical world root, or establish semantic equivalence.
