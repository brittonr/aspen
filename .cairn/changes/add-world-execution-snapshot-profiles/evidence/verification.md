# World execution snapshot verification

## Baseline and pure-core stage

Recorded on 2026-08-26 before effectful snapshot adapters.

### Molten baseline

- `cargo test -p molten-core --all-features`: 235 unit tests and seven compile-fail documentation tests passed.
- Sealed reproduction-bundle filter: five hostile tests passed.
- Content-store adapter filter: five tests passed.
- Existing world-commit closure and restore tests passed in the complete core suite.
- Scheduler, virtual-time, entropy, effect-state, durability, simulation, and typed-root tests passed in the complete core suite.

The historical `replay_summary` filter selected no tests. Replay-summary code remains present, but this result is not counted as replay-summary test evidence.

### ChaosControl baseline

Exact local revision: `ad3923ed38d8f6da9038cd41a75495811c0015b4`.

`cargo test -p chaoscontrol-snapshot-descriptor` passed six positive identity and restore-plan tests plus eight negative tests.

This revision remains a local, unarchived dependency candidate. Molten does not treat it as a durable published dependency.

### VM Cohort baseline

Observed local revision: `ac279d14afa586521cfe2f5a2b229416d2de7ce9`.

The checkout contains no Rust package implementation. Clone realization remains blocked until VM Cohort publishes implementation and independent ChaosControl pilot evidence.

## Implemented pure boundary

The pure snapshot core now provides:

- closed logical and opaque profile classes;
- complete typed component and cohort inventories;
- explicit Molten and ChaosControl ownership facts;
- default denial for unknown profiles and undeclared synchronization claims;
- exact compatibility and deterministic restore ordering;
- live-handle and current-authority denial;
- divergent opaque-root merge denial;
- parent-bound overlay collision and isolation checks;
- canonical Preserves descriptors, inventories, compatibility reports, restore plans, clone plans, and bounded receipts;
- domain-separated BLAKE3 identities for each canonical artifact class;
- explicit non-claims for correctness, portability, authority, handle transfer, semantic equivalence, isolation, and release eligibility.

The logical shell now uses narrow materialization, current-admission, host-handle, restore, observation, and receipt ports. It validates the complete inventory before state effects, recreates handles, rechecks non-regressing current admission, activates last, and publishes the success receipt after activation.

Focused tests cover a complete logical restore, stale admission after restore, and unavailable component materialization. Stale or incomplete cases never activate or publish a success receipt.

Strict descriptor decoding round-trips normalized canonical bytes. It denies unsupported schemas and typed values through closed parsers before shell effects.

`docs/world-execution-snapshots.md` records ownership, component and cohort fields, restore order, handle recreation, authority checks, merge limits, VM Cohort blocking, and non-claims.

Focused core and shell tests, formatting, and Clippy with warnings denied passed.

## Remaining dependency gates

- The logical shell adapter can proceed without external snapshot ownership transfer.
- The ChaosControl adapter requires a reviewed immutable published descriptor cohort.
- Clone realization requires implemented VM Cohort mechanics and its independent ChaosControl pilot.
- No current evidence authorizes opaque fallback, cross-cohort restore, host-handle capture, or release claims.
