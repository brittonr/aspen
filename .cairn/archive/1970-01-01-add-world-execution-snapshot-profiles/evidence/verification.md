# World execution snapshot verification

Date: 2026-08-27

## Baseline

Before adapter changes, `cargo test -p molten-core --all-features` passed 248 unit tests and seven negative compile tests.
Existing logical restore, scheduler, virtual-time, entropy, effect-state, durability, simulation, and typed-root tests passed in that suite.

The historical `replay_summary` filter selected no tests.
This result is not replay-summary evidence.

## Reviewed dependencies

Molten consumes `chaoscontrol-snapshot-descriptor` from revision `b8c440ea3b19df796542e58e8ee36200e1c3db85`.
That revision includes the archived VM Cohort adoption and portable descriptor boundary.

Molten optionally consumes `vm-cohort-core` from private RID `rad:z2QJLUqyAZnnHPiZQ1BFjLsX9ush3` at revision `31f1696ba9391bfda8577a58af84f72361d5573e`.
VM Cohort mechanism revision `ab123e3673b6dd616b3df5d044026b5e85755149` remains the consumed implementation identity.
The later revision adds consumer and lifecycle evidence.

Cargo, Cargo lock, Nix inputs, Nix lock, and the release dependency profile bind these revisions.
VM Cohort stays behind feature `world-snapshot-vm-cohort` because the pinned unit2nix Cargo parser cannot lower active `rad://` package IDs.

## Implemented boundary

The pure core provides closed logical and opaque profile classes.
It validates complete component and cohort inventories, exact compatibility, ownership, synchronization, restore ordering, clone isolation, and bounded receipts.
It rejects unknown profiles, live handles, stale authority, divergent opaque merges, incomplete state, and crossed cohorts.

The logical shell validates all observations before effects.
It recreates handles, rechecks current admission, activates last, and publishes a success receipt only after activation.

The ChaosControl adapter accepts only portable exact descriptors.
It verifies descriptor identity and maps one opaque machine root without importing ChaosControl policy or evidence authority.

The VM Cohort adapter requires an observed effective disk size.
It binds every child to one parent and private memory, device, disk, and endpoint overlays.
Realization rejects partial activation, crossed plans, malformed receipts, cleanup uncertainty, and product-authority overclaims.

The operator CLI provides `inspect`, `compatibility`, `restore-plan`, `clone-plan`, and `restore` commands.
Without an admitted runtime adapter, `restore` emits a canonical denial receipt and fails closed.

## Cargo verification

Passed:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p molten --lib --tests --features world-snapshot-vm-cohort -- -D warnings
cargo fmt --all -- --check
```

The workspace run passed 1,346 main Molten tests, 254 `molten-core` tests, seven negative compile tests, and all other workspace suites.

Focused default snapshot tests passed: eight of eight.
Focused feature-enabled snapshot tests passed: twelve of twelve.
The feature-enabled set includes complete and denied restore, exact descriptor drift, clone binding, disk drift, partial activation, cleanup uncertainty, and authority overclaim cases.

## Octet verification

The isolated `checks/world-snapshot-octet/` workspace passed fifteen core tests.
The strict pinned Octet run used `DYLINT_RUSTFLAGS=--deny warnings` and reported:

```text
Status: clean
Findings: 0
Warnings: 0
Errors: 0
```

The Nix `world-snapshot-octet-deny-all` consumer check also passed with zero findings.
The focused source split keeps every checked Rust file below the enforced file-length limit.

## Generated plans and Nix checks

Pinned unit2nix regeneration produced:

- `build-plan.json`: 640 crates, four workspace members, 891 build units, and 918 test units;
- `release-policy-build-plan.json`: 53 crates and two workspace members.

The Cargo lock SHA-256 reported by unit2nix is `429040a160ea8f37ca1050145ecc631b8199f95337b90bdf3dcc0b7c0884b634`.
SHA-256 is used here because unit2nix defines this interoperability field.

Passed Nix checks:

```text
world-snapshot-dependency-identity
world-snapshot-octet-deny-all
release-dependency-profile
```

`nix flake check . --no-build` evaluated every flake output and passed.
The repository-wide `contract-export-drift-gate` build failure is inherited from the canonical Molten baseline.
It remains separate from these focused snapshot checks.

## Claim limits

Logical and opaque snapshots are not semantically equivalent.
Descriptor possession does not prove disk materialization, guest correctness, host portability, current authority, isolation, or release eligibility.
VM Cohort plans and receipts do not transfer ChaosControl fault, replay, assertion, evidence, or release meaning.
Unknown restore, clone, cleanup, and publication outcomes never imply success.
