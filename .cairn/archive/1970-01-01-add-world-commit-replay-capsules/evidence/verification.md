# World replay capsule verification

## Baseline

Pueue task `3458` ran against an isolated `origin/molten` worktree before the replay implementation.

- `cargo test -p molten-core world_commit`: 9 passed.
- `cargo test --test worldcommit`: 4 passed.
- `cargo test --lib harness`: 9 passed.
- `cargo test --lib content_store_adapter`: 5 passed.
- `cargo test --test cliharness repro`: 2 passed.

The baseline had no failures.

## Implemented boundary

The pure core now owns typed transition traces, bounded capsule closure, deterministic plans, domain-separated BLAKE3 identities, and earliest-divergence meaning.

The shell now owns explicit ports for materialization, logical and exact opaque restore, current admission, transition execution, successor capture, exchange, import staging, availability publication, and receipt publication.

Capsule adapters reuse canonical world commits, world snapshot descriptors, content manifests, and sealed reproduction bundles. Locator hints and transport tickets remain detached.

## Focused tests

Pueue task `3606` passed the current focused suites.

- `cargo test -p molten-core world_replay`: 9 passed.
- `cargo test --lib world_replay`: 11 passed.

Positive coverage includes stable multi-step replay, deduplicated closure, complete export and import, exact opaque replay, and existing bundle and content-manifest adapters.

Negative coverage includes wrong parents, reordered steps, wrong successors, first divergence, missing and extra members, tampered identity, noncanonical input, unsupported profiles, stale schemas, plaintext secrets, bearer material, unavailable decryption, current-authority denial, opaque fallback, and import-as-authority denial.

## Evidence limits

These checks establish bounded source, fixture, identity, closure, and shell observations only.

They do not prove universal determinism, logical and opaque semantic equivalence, capability transfer, external effect completion, branch movement, runtime activation, or release eligibility.

## Final rails

Pueue task `3641` passed current focused replay tests, workspace Clippy across all targets and features with warnings denied, and the focused Nix Octet check.

Pueue task `3645` passed the broad post-change regression set.

- `cargo test -p molten-core`: 286 tests and 7 doctests passed.
- `cargo test --lib -- --test-threads=1`: 1,377 tests passed.
- `cargo test --test worldcommit`: 4 tests passed.
- `cargo test --test cliharness repro`: 2 tests passed.

Pueue task `3629` ran the isolated full-catalog Octet gate. It reported `Status: clean`, zero findings, zero warnings, and zero errors.

Pueue task `3643` ran the broad repository Octet scan. It remained `warning-only` with 6,507 inherited findings and zero errors. The result is not acceptance evidence. Its finding index contained no `src/world_replay` entry.

Pueue task `3638` passed `nix flake check --no-build`, the replay schema inventory, replay Octet, snapshot dependency, distribution dependency, and release dependency checks.

Pueue task `3644` passed strict Cairn repository validation and the proposal, design, and tasks gates with the canonical Cairn policy.

The inherited full-build `contract-export-drift-gate` remains outside this replay change. Focused replay checks and flake evaluation pass.
