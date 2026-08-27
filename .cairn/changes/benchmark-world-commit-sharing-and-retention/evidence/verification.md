# Verification evidence

Date: 2026-08-27

## Dependency and ownership evidence

Molten consumes only `chaoscontrol-snapshot-descriptor` from ChaosControl revision `7433557b85990f0f07a37ca44b97fef26c2a4c7e`.

- Cargo source: `ssh://git@github.com/brittonr/chaoscontrol.git`.
- Nix source: the same repository and revision through `chaoscontrol-src`.
- Crate: `chaoscontrol-snapshot-descriptor` version `0.1.0`.
- Crate license: `AGPL-3.0-or-later`.
- Nix asserts Cargo, lockfile, Nix input, package, revision, and license agreement.

ChaosControl retains exact snapshot descriptor meaning. Molten owns benchmark profiles, metrics, comparisons, receipts, and extraction policy. Snapshot validity does not grant access, replay, clone, activation, retention, deletion, or release authority.

## Baseline

Before implementation:

- world distribution core: 7 passed;
- world merge core: 7 passed;
- world distribution shell: 5 passed;
- world distribution CLI parsing: 2 passed;
- content replication focused shell set: 10 passed;
- no Molten world-snapshot module existed on `origin/molten`; the benchmark therefore binds the published ChaosControl descriptor directly and keeps opaque results separate.

## Focused results

- `cargo test -p molten-core world_benchmark`: 6 passed.
- `cargo test -p molten world_benchmark`: 4 passed.
- Nickel positive profiles: logical cold synthetic, logical declared-warm downstream-shaped, and opaque exact ChaosControl all exported.
- Nickel negative profiles: unknown preparation, hidden prepopulation, profile mixing, unnamed threshold, and timing-as-correctness all failed as required.
- Rust decoded and revalidated checked Nickel projections before execution.
- Repeated deterministic count-only runs produced equal plan and receipt identities.
- The exact ChaosControl fixture validated and bound complete copied and mapped page observations.
- Protected retention candidates failed before receipt publication.

## Broad Cargo and Octet results

- `cargo test --workspace --all-features`: all suites passed. Major suite counts included 1,342 root-library tests, 259 core tests, 61 CLI harness tests, 59 binary tests, and all integration and documentation tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `world-benchmark-octet-deny-all`: clean, 0 findings, 0 warnings, and 0 errors.

## Nix results

- `world-benchmark-profile`: passed.
- `world-benchmark-dependency-identity`: passed.
- `world-benchmark-octet-deny-all`: passed.
- `release-dependency-profile`: passed.
- `release-profile-validation`: passed its positive case and expected negative cases.
- `deterministic-drift-gate`: passed after unit2nix regenerated both plans.
- `nix flake check --builders '' --no-build`: all outputs evaluated.

`build-plan.json` contains 630 crates and the exact ChaosControl revision. `release-policy-build-plan.json` contains 53 crates and retains the `molten-release-policy` binary target.

## Repository-wide inherited failure

`nix flake check --builders '' -L` reached the inherited `contract-export-drift-gate` failure. The checked-in `cairn-policy/generated/cairn-policy.json` does not match an export of `cairn-policy/default.ncl`. This mismatch existed on `origin/molten` before this change. The benchmark change modifies neither file.

## Lifecycle dependency

Cairn sync and archive remain blocked because `add-world-execution-snapshot-profiles` is not archived. The benchmark implementation and its own tasks are complete, but this branch does not bypass that declared dependency. No accepted specification was synchronized, and the Cairn remains active with `status = "blocked"`.

## Bounded claims

The evidence proves only the checked profile, exact count, source-binding, validation, and receipt behavior. It does not prove asymptotic complexity, universal performance, storage correctness, benchmark representativeness, snapshot semantic equivalence, deletion safety outside supplied complete facts, dependency approval, component extraction, or release readiness.
