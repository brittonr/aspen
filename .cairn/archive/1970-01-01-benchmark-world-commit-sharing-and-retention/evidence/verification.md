# Verification evidence

Date: 2026-08-27

## Dependency and ownership evidence

Molten consumes `chaoscontrol-snapshot-descriptor` from revision `b8c440ea3b19df796542e58e8ee36200e1c3db85`.

- Cargo source: `https://github.com/brittonr/chaoscontrol.git`.
- Nix source: the same repository and revision through `chaoscontrol-src`.
- Crate: `chaoscontrol-snapshot-descriptor` version `0.1.0`.
- Crate license: `AGPL-3.0-or-later`.
- Nix asserts Cargo, lockfile, Nix input, package, revision, and license agreement.

The merged baseline also contains the optional VM Cohort dependency at `31f1696ba9391bfda8577a58af84f72361d5573e`.
The benchmark adapter does not import VM Cohort mechanics.
It observes only exact snapshot descriptor facts.

ChaosControl retains snapshot meaning.
Molten owns benchmark profiles, metrics, comparisons, receipts, and extraction policy.
Snapshot validity does not grant access, replay, clone, activation, retention, deletion, or release authority.

## Baseline and dependency closeout

Before implementation:

- world distribution core: seven passed;
- world merge core: seven passed;
- world distribution shell: five passed;
- world distribution CLI parsing: two passed;
- content replication focused shell set: ten passed.

The benchmark was initially blocked on `add-world-execution-snapshot-profiles`.
That change is now archived at Molten revision `2217e2910e2998621bee9acf73f8a4174b7b8267`.
The benchmark branch merged that exact canonical baseline before final verification.

## Focused results

- `cargo test -p molten-core world_benchmark`: six passed.
- `cargo test -p molten world_benchmark`: four passed.
- Nickel positive profiles exported for logical cold, logical declared-warm, and opaque exact cohorts.
- Nickel negative profiles rejected unknown preparation, hidden prepopulation, profile mixing, unnamed thresholds, and timing-as-correctness.
- Rust decoded and revalidated checked Nickel projections before execution.
- Repeated deterministic count-only runs produced equal plan and receipt identities.
- The exact ChaosControl fixture bound complete copied and mapped page observations.
- Protected retention candidates failed before receipt publication.

## Broad Cargo and Octet results

Passed:

```text
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Major suite counts included 1,354 root-library tests, 265 core tests, two 61-test CLI suites, and seven negative compile tests.

The Nix `world-benchmark-octet-deny-all` check reported a clean result with zero findings, warnings, and errors.

## Nix and generated-plan results

Passed:

```text
world-benchmark-profile
world-benchmark-dependency-identity
world-benchmark-octet-deny-all
release-dependency-profile
nix flake check . --no-build
```

`build-plan.json` contains 640 crates, four workspace members, 891 build units, and 918 test units.
`release-policy-build-plan.json` contains 53 crates and two workspace members.
The unit2nix Cargo lock SHA-256 is `429040a160ea8f37ca1050145ecc631b8199f95337b90bdf3dcc0b7c0884b634`.
SHA-256 is used because unit2nix defines this interoperability field.

## Repository-wide inherited failure

A full `nix flake check --builders '' -L` retains the inherited `contract-export-drift-gate` failure.
The checked-in Cairn policy export does not match its Nickel source.
This mismatch existed on `origin/molten` before the benchmark work.
The benchmark does not modify either policy file.

## Bounded claims

The evidence proves only the checked profile, exact count, source binding, validation, and receipt behavior.
It does not prove asymptotic complexity, universal performance, storage correctness, benchmark representativeness, snapshot equivalence, deletion safety, component extraction, or release readiness.
