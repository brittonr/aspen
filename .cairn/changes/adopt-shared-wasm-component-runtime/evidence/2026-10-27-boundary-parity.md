# Immutable boundary-parity continuation — 2026-10-27

## Outcome and claim boundary

The published v4 prerequisite corrects the original frozen start-work and memory-admission counterexamples without changing Molten's admitted synchronous/time-sliced profiles or WIT. The original component bytes and full comparison oracle remain: whole decisions, canonical output, typed receipt inputs, payloads, BLAKE3 identities, stage parents, successful fuel and callback ledgers.

The candidate remains `cfg(test)` only. Production synchronous execution, time-sliced callbacks and existing actor/extension paths still use the complete local runtime. No product effect world, shared-runtime consumer installation, matching positive production Mantle bundle, operated product deadline, cutover or rollback is claimed. Do not sync or archive `adopt-shared-wasm-component-runtime`.

Immediate outcome: a compatible immutable producer and executed unchanged-cohort parity for the two former blockers. Durable contribution: explicit initialization draining, checked independent memory translation and positive/negative real-component regressions. Molten's component-runtime maintainers own this adapter, product admission, receipt/ledger mapping and repeatable acceptance. Hardened-runtime maintainers own execution mechanics; Mantle maintainers own their materialization schema/verifier.

## Immutable source and completed producer evidence

- Both hardened runtime packages now resolve canonical delegate-qualified Radicle revision `cf4a1913f7f2ca7eb7f655663e87b8b72d0c506c`, contract `hwr.shell.v4`, Wasmtime 45.0.0.
- The producer's native `preserve-component-execution-boundaries` package was reviewed, synced, archived and published to canonical Radicle main. Push explicitly reported no seeds; remote replication is not claimed.
- Cargo fetched that published revision without a source-path override. Targeted Nix locking fetched the same revision; metadata-free NAR hash is `sha256-i+N+OVTdZk7t68IkVYj2EzS/8UfgN3YQ1DjFdGg3+Xg=`.
- Cargo manifest/lock, Nix input/lock, Nickel dependency policy, source hashes, both regenerated unit2nix plans and license notice select the same source. Mantle remains `c99556e5c91dfd13fcc261dc451f471359d47c38`.
- The existing stable archive-receipt identity now points at `.cairn/archive/1970-01-01-preserve-component-execution-boundaries`; it binds completed tasks, actual shell contract bytes and producer verification:

```text
d4a6554a7a61162d339bf9dc515be4e5725847ef8cb7c5b3167e1543fb8aea87  tasks.md
510b2cf96efd92c096d8d780a844e272c7811317aee3dc94fed891df89f59fa0  crates/hardened-wasmtime-shell/src/lib.rs
ad9639e51a32ab13ff4aaae009c96155cd7d11d23e0c79c3864fbe467f127e04  verification.md
```

Producer checks passed 8 core, 30 admission and 44 actual execution cases, strict Clippy, formatting, deny-all Octet, strict native Cairn and all three host Nix rails. A real public-API executable outside the test runner observed seven typed initialization yields, exactly one checkpoint with 9517 remaining fuel, then `Completed` with exact `OK` bytes and 9501 fuel matching synchronous execution. Actual module growth independently denied at `(per-memory, aggregate)=(2,1)` and `(1,2)` and succeeded at `(2,2)`. Its temporary source was removed. These are producer mechanism observations, not Molten production execution.

## Reviewed mapping and exercised behavior

`runtime/shared/sliced.rs` drains initialization through the public successful checkpoint under the same finite `total_budget_fuel / quantum_fuel + 1` poll bound as the local shell. Typed initialization yields never enter the callback scheduler or ledger. A failed start returns Inspection → Denial before exposing a callback handle. Invocation then resumes the same future/store, with no guest restart, fuel refill, quantum reset or second local engine. Terminal measured fuel retains the local final-slice convention, including initialization work.

`runtime/shared/configuration.rs` preserves the per-memory byte/page ceiling. Its explicit aggregate bound is the checked per-memory pages times the already admitted memory count. This is a redundant upper bound over the existing local policy, not multiplication of a product grant: every individual memory and the total memory count still have the unchanged product ceilings. Zero, misalignment, multiplication overflow and aggregate `u32` overflow deny construction rather than truncating or saturating.

Actual fixtures exercised:

- Original 3000-iteration start work: equal callback boundaries, complete ledgers, canonical outputs and full receipts for Actor and SystemExtension labels.
- Original two fixed 129-page memories: equal successful output, Inspection → Instantiation → Execution, whole canonical receipts and successful fuel.
- Actual 256/257-page per-memory and 4/5-memory count boundary pairs: byte reinspection agrees with supplied facts before local/shared admission comparison. Over-bound cases deny rather than hiding behind stale-fact errors.
- Trapping start: both constructors deny before returning a callback, with the same complete Inspection → Denial records and `component-instantiation-denied` class.
- Existing retained execution, cancellation, cleanup, malformed output, pointer/transfer precedence, fuel, stale evidence, wrong world, undeclared imports, loose production bytes, fallback and receipt-overclaim cases remain in the corpus.

The first integrated focused run passed 60 cases and exposed one obsolete v3 test that expected a trapping start to return a lazy shared handle. That incidental deferral test was deleted, not re-pinned. The stronger constructor-denial regression compares both paths and complete parented records; no duplicate same-path case remains. Initial Octet warning-only evidence named two ancestor-word test names. Both names were corrected without an allowance. The unsuccessful workspace evidence remains in the source-gate directory; warning-only is not accepted evidence.

## Unchanged product inputs

These independently remeasured BLAKE3s match the preflight:

```text
5bea351924145de8946669680125e99c837fc61ac6737e114ba3b9edf7ea0b89  docs/wasm-component-runtime/generated/profile.json
48b088f86827c4fa31ffe6c2151dd32a8db53b3f38afef2b2e1b394d43d20852  docs/wasm-component-runtime/generated/time-sliced-profile.json
83be349bb27975ada30dbe60817c5404df7862babdf02ae229b399886e76d5e8  wit/molten-component-runtime/runtime.wit
```

## Executed continuation checks

Commands ran from the implementation worktree with the repository's pinned Nix development shell, executable task-owned `/dev/shm` temporary and Cargo target directories, two Cargo build jobs, and debug information/incremental compilation disabled. The canonical checkout supplied tools, not a dependency source override. Cargo resolved the published immutable producer.

| Executed surface | Observed result |
|---|---|
| `cargo test --locked -p molten --lib -- wasm_component` | 60 passed, zero failed or ignored. The candidate is test-only; this is not a production backend installation. |
| `cargo test --locked --workspace --lib --quiet` | Molten 1760 and Molten Core 629 passed; the other two library targets had zero cases. Zero failed or ignored cases. Not the integration-target or optional-feature suite. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed without allowances or policy relaxation. |
| `cargo fmt --all --check`; `nixfmt --check flake.nix` | Passed. The initial combined verification reached an unformatted new Nix assertion; canonical `nixfmt` corrected it before the final checks. |
| Pinned workspace and `-p molten -- --lib` Octet producers | Both final statuses are `clean`, exit zero, with zero findings, warnings or errors. Producer toolchain remains `nightly-2026-03-21-x86_64-unknown-linux-gnu`. |
| Real CLI artifact import, `strict-ci` gate and remediation plan | Import decision `pass`, six artifacts imported; gate `pass`; canonical remediation plan emitted. |
| Inherited Tracey guard | `pass`: 2966 requirements, 1032 referenced, 1934 unchanged baseline entries, zero dangling references. Five pending references confer identity resolution only, not accepted coverage. |
| Strict native Cairn validation with the explicit sibling-provider policy | Passed with empty findings/issues. Policy BLAKE3 `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`; lifecycle structure is not behavioral acceptance. |
| `scripts/git-source-hashes.sh --check`; Nix `release-dependency-profile` and `git-source-hash-binding` | Passed. Nix realized `/nix/store/18a5ffcm8wrz6h23s0jj5bg5saxpvar5-molten-release-dependency-profile` and `/nix/store/n0n8xr7d27ah13y4axk1pcvd1qca776n-molten-git-source-hash-binding`. Full flake checks, VM integration and release promotion were not run. |

The final required source sequence was both clean Octet producers, object-corpus receipt, actual CLI import/gate/remediation, workspace library tests, strict Clippy, and strict Cairn validation. Final emitted command/status/summary/provenance files and canonical CLI receipts are retained under [`2026-10-26-boundaries-source-gate/`](2026-10-26-boundaries-source-gate/), alongside the unsuccessful producer attempt. The full corpus command selects recursive `src` plus `src/job/dag.rs`, `src/main.rs`, `src/node/daemon.rs`, `src/node/runtime.rs`, `src/octet/gate.rs` and `src/upgrades/mod.rs`; the gate was not weakened.

Canonical CLI identities, not raw rendered-file hashes:

```text
artifact import  blake3:030bd7f098097c01cd4cccb65f8cd95bff80e440d50a9035609ace42663369fd
strict-ci gate   blake3:bfe1fc01a0e2d5acc2b90e5072167b5851231f30a8dcefd3555c8ce6a09130a8
remediation plan blake3:dd229a20c15b9bca925187813fba550fb684072a2367e4ff2a5da751ec19f723
corpus bytes     3a9e86c7a229c0866a81dfb16a61a5dd2a6129578c9f16c7ad7b7710b23621a6
```

The host kache index reported a disk I/O error and explicitly disabled caching; compilation, tests and source checks still completed. No unrelated cache or storage configuration was changed.

## Remaining adoption gates

Operated product deadlines and the new closed effect world/admitted host adapter still need product-owned evidence. Non-test ordinary actor and system-extension composition need complete matching Mantle member trees and actual production-bound admission inputs. Consumer enum labels, loose-byte fixtures and successful producer calls are not installations. Dependency, adapter, profile/WIT, consumer, materialization and receipt identities must cut over and roll back together only after those gates pass. The full local mechanism must not be removed before then.

Historical counterexamples and previous source receipts remain in the [earlier frozen record](2026-10-26-runtime-adoption-parity.md); they describe its old immutable producer, not current v4 failures. This continuation proves neither arbitrary guest correctness, host isolation, source trust, authority, release readiness nor full adoption.
