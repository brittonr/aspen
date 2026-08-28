# Verification

Date: 2026-08-28

## Completed boundary

Molten now has one bounded crash, restart, uncertainty, and concurrency conformance rail for world mutations.

The pure core owns these facts:

- a closed nine-row mutation inventory;
- eight named semantic fault phases;
- typed operation observations and durable read-back;
- eight conservative recovery classes;
- explicit fenced schedules;
- domain-separated BLAKE3 inventory, profile, and schedule identities;
- bounded comparison and receipt meaning;
- retained unsupported rows and required non-claims.

The shell owns interruption, restart, local durable read-back, schedule execution, and receipt publication.
It calls one owner decision port for each supported case.
It does not create success, retry, compensation, conflict, or cleanup decisions.

## Source and profile identities

The profile binds implementation revision `06cd7ca465550d2a35e0511cbbd7989e434d8f51`.

The pure identities are:

- mutation inventory: `blake3:d2d36a4685dbfcee16075e508a70592e51e6df12f86b58d039d5b961b985d367`;
- fault profile: `blake3:c7dc3a3c40e1d2cc1618bac585ecbeaa5178d94b4290607018c74cf8a94aaa3c`.

The generated Nickel profile has BLAKE3 `faf8a9beee1c290e4077feb45113f91b918a02230ca2f3cfdd349e3fb0cf1c1e`.
Two independent exports produced identical bytes.

## Mutation inventory

The closed inventory covers:

1. capture;
2. head transition;
3. promotion transaction;
4. witness finalization;
5. outbox effect attempt;
6. replication update;
7. capsule import;
8. retention update;
9. garbage-collection planning.

Each row binds its owner, operation domain, expected pre-state, effects, linearization point, durable record, uncertain window, and recovery entry.

An unknown product mutation fails inventory validation.

## Fault phases and schedules

Each row retains these named phases:

1. uninterrupted;
2. before submit;
3. after possible submit;
4. after durable write;
5. before response;
6. lost response;
7. process restart;
8. recovery read-back.

The standard profile has 72 cases, 8 concurrent schedules, and 64 schedule steps.

Schedules cover head, promotion, witness, outbox, import, replication, retention, and garbage collection.
They bind operation identities, expected generations, pre-state identities, nodes, and declared interleaving points.

The core converts schedule steps into existing Fabric Simulation scheduler choices.
Wall-clock timing does not select a winner.

## Recovery behavior

Positive fixtures cover:

- already complete;
- safe to retry;
- superseded;
- conflict;
- uncertain;
- corrupt;
- manual review.

Negative fixtures cover:

- torn records;
- lost responses;
- duplicate submission risk;
- stale plans;
- missing objects;
- corrupt records;
- generation races;
- effect uncertainty;
- rollback without an independent witness;
- unsafe cleanup;
- contradictory observations;
- fault-coverage overclaims.

An already-complete decision requires exact durable state and record read-back.
A possible submit cannot become a blind safe retry.
Missing or contradictory facts never become success.

Transactional Reconciliation Core supplies promotion persistence decisions.
Unknown commit outcomes remain quarantined until exact read-back resolves them.

## Restart evidence

The shell restart test writes through the capability-rooted node-state adapter.
It then reopens the node-state root before durable read-back.

The standard run performs 16 bounded restart operations across the supported rows.
The receipt is published after every supported case and schedule completes.
A crossed receipt identity fails closed.

## Witness boundary

The local profile does not have an independent witness owner.
All witness phase rows and the witness concurrency schedule remain explicit and unsupported.

A restored local image can roll back the head and generation together.
The receipt does not claim strong rollback detection without independent state.

## Rust verification

Before implementation, the baseline core and serial library commands passed.
The recorded baseline counts were 300 core tests and 1,397 library tests.

Focused final tests passed:

- 10 world-fault core tests;
- 4 world-fault shell and projection tests.

This full command passed:

```sh
cargo test --workspace --all-targets --all-features -- --test-threads=1
```

The full run included these results:

- 1,405 `molten` library tests;
- 74 `molten` binary tests;
- 61 CLI harness tests;
- 324 all-feature `molten-core` tests;
- 12 content-replication integration tests;
- 8 native-system-extension integration tests;
- 6 executable-extent integration tests;
- 5 node-host boundary tests;
- 5 release-policy binary tests;
- every other listed integration and feature-gated suite.

Formatting passed with `cargo fmt --all -- --check`.

Clippy passed with this command:

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Octet

`checks.x86_64-linux.world-faults-octet-deny-all` passed with zero findings, warnings, and errors.
It checks the complete pure core with the pinned full catalog.

The broad supported scan completed as warning-only:

- findings: 6,534;
- warnings: 6,534;
- errors: 0.

No broad finding names `crates/molten-core/src/world_faults` or `src/world_faults`.
The broad warnings are inherited outside this change.

## Nickel, schemas, and Nix

The positive Nickel profile exported successfully.
Every negative Nickel fixture failed contract evaluation.
Rust tests compare every generated case, adapter, limit, and schedule with the Rust projection.

These focused Nix checks passed:

- `world-faults-profile`;
- `world-faults-schema-inventory`;
- `world-faults-octet-deny-all`.

`nix flake check --no-build -L --option builders '' --option secret-key-files ''` evaluated every compatible output and passed.

The generated build plans remain unchanged because this change adds no product dependency.

Main plan:

- 713 crates;
- 4 workspace members;
- 989 build units;
- 1,019 test units;
- BLAKE3 `40ae6e5cf5e59c5ac0246b712a770008784b77c73e65e9ddb3504dce54c33d29`.

Release-policy plan:

- 254 crates;
- 2 workspace members;
- 323 build units;
- 2 roots;
- BLAKE3 `9ec27b4dcc8ff82dba13d96ffa5fe8fa626493fabd0ae5fbfdb0ed4986fdb0d5`.

The release plan still contains the `molten-release-policy` binary target.
Cargo lock SHA-256 is `4831149e509872705a47b22aeb5e48e06a2eb7b15524fc09ee7cbf766ffcf935`.
SHA-256 appears only because unit2nix requires that interoperability field.

## Cairn

Current Cairn validation passed with no issues.
The proposal, design, and tasks gates returned `PASS`.

Gate receipt hashes:

- proposal: `bac9c2417869c11a56496c6781f3da269173fee2ec4757d0fd599511e6531746`;
- design: `fad3bab1186aa262788ac414705defac76c42b1185308f20ba8f63d0ed6cdf82`;
- tasks: `003babf33865dfc7b151522b246264693601464e3d7dfead8e4820f399873183`.

## Inherited repository rail

The focused requirement traceability gate passed after sync.
It reported receipt `blake3:661efe0e9f7aa853f529f438ecdfd94a5904266b726b876961b98eb54d64ac98`.

A full built `nix flake check -L` still fails in the inherited `inherited-tracey-debt` check.
That guard still reads legacy `cairn/specs`, but this repository uses `.cairn/specs` and has no legacy directory.
It therefore reports existing repository references as dangling.

This path mismatch predates this change and is outside the world-fault boundary.
A complete built flake pass is not claimed.

## Non-claims

A passing conformance receipt proves bounded agreement for the exact exercised cohort and semantic fault points.
It does not prove universal crash safety.
It does not prove physical power-loss behavior or storage correctness.
It does not establish release eligibility.
It does not authorize mutation, cleanup, activation, effect dispatch, or deletion.
