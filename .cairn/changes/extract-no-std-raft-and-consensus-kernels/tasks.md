# Tasks

- [x] [serial] Extract `crates/molten-raft-core` (`#![no_std]` + `alloc`) from `src/fabric_consistency/raft/{transition,ports,parts}` with typed core errors, standard module files (no `include!()` shards), and thin Tokio/Iroh/Redb shells remaining in `src/fabric_consistency/raft/`. r[molten.consensus.no_std_raft_and_reliability_kernels] r[molten.project.tier1_no_std_kernel_crates]
  - Done (2026-11-26). The crate owns the model (`model`), the transition function with election, client, replication, read index, snapshot, rejoin and recovery repair (`transition`), envelope shape validation (`transition::wire`), start admission (`admission`), runtime port identity checks (`ports`), and typed errors (`denial::ReplicaError`, classes `Invalid`/`MalformedObservation`/`Encoding`, all `core::error::Error`; ref and identifier checks in `denial::refs`). It depends only on `molten-core` without default features (for `consensus_recovery`).
  - Design decision: canonical identities (election timer, snapshot, recovery barrier, entry, recovery corpus refs) enter through the `identity::ReplicaIdentity` port. The shell binds `CanonicalReplicaIdentity` (BLAKE3 over the unchanged canonical Preserves records) in `src/fabric_consistency/raft/kernel.rs`, which also maps every `ReplicaError` to the same `Failure` text and class as before (`replica_failure`). The async effect traits and executor stay in the shell because their results carry classified I/O failures.
  - Shell paths are unchanged: `raft/mod.rs` re-exports the kernel types; `apply_replica_event`, `validate_retention_bounds` and the wire codecs keep their `crate::error::Result` signatures. `fabric_consistency::ReadMode` re-exports the kernel's. Unused `SnapshotPlan` was removed.
  - Regression tests moved with the code into `crates/molten-raft-core/src/tests/` (bounds, liveness, steps, steps/quorum, rejoin, rejoin/fencing) and `src/transition/rejoin/repair/tests/` (copies, contradiction), plus two Hegel properties (determinism; one leader per term and agreeing committed prefixes) and a kernel-level commit-chunk test. Tests that need I/O stay in the shell: `executor/chunks.rs` (effect-bounded commit through the executor), `rejoin/crash.rs` (module `rejoin_restart_tests`: Redb crash between repair and snapshot, driven through recovery reports), and `kernel/tests.rs` (class-preserving mapping, canonical identities). Octet `path_segment_repetition` renamed two repair regressions: `a_lost_peer_before_the_repair_defers_…` → `a_lost_peer_before_the_rewrite_defers_and_keeps_the_committed_tail`, and `a_committed_tail_after_replaced_entries_is_rewritten_with_the_repair_…` → `…_is_rewritten_in_the_same_mutation_and_never_dropped`. `verification/tla/AspenRaft.tla` now names `crates/molten-raft-core/src/transition*`.
- [x] [parallel] Extract `crates/molten-reliability-model-core` (`#![no_std]` + `alloc`) from `src/reliability/model/` and `src/reliability/service/profiles/`, keeping `serde_json` file I/O in `src/reliability/`. r[molten.consensus.no_std_raft_and_reliability_kernels] r[molten.project.tier1_no_std_kernel_crates]
  - Audit correction: `service/profiles/` is not a pure fault-profile model. Its DTOs convert into `molten-core` fabric transport/time/durability types and `experimental()` hashes definitions over canonical Preserves, so the DTOs stay in the shell; only the bounded serde text/collection deserializers and closed-vocabulary `choice`/`choices` (typed `VocabularyIssue`) moved (`mechanism` module). The fault-profile timing/partition validation lives in `model/fault.rs` and moved with a typed `FaultIssue`. Canonical identity and the history publication size enter the core as explicit ports (`profile::admit(profile, canonical_ref)`, `history::validate(.., encoded_len)`); the shell binds Preserves hashing and `serde_json`.
- [x] [parallel] Extract `crates/molten-wasm-policy-core` (`#![no_std]` + `alloc`) from `src/wasm/component/{admission,profile}` and `src/wasm/performance/policy/` with zero `wasmtime` dependency. r[molten.wasm.no_std_wasm_policy_kernel] r[molten.project.tier1_no_std_kernel_crates]
  - The core owns the component model/denial, profile cohort validation (`admit_export`), execution planning (declared import surface supplied as a port, since the manifest identity is canonical Preserves), the materialization `Admission` facts, and the performance profile/optimization policy (`validate_conformance` takes the component profile as input). Byte inspection (`admission/inspection.rs`, `wasmparser`) and JSON embedding/decoding stay in `src/wasm/`; the embedded exports now come from `config/`.
- [x] [parallel] Extract `crates/molten-fastpath-core` (from `src/fabric_consistency/fastpath/`), `crates/molten-composition-core` (from `src/harness/composition/`), and `crates/molten-runtime-admission-core` (from `src/runtime/admission/mod.rs` and `src/authority/nominal.rs`) as `#![no_std]` + `alloc` crates. r[molten.consensus.no_std_raft_and_reliability_kernels] r[molten.project.tier1_no_std_kernel_crates]
  - Done (2026-11-26) for all three crates. `molten-composition-core` and `molten-runtime-admission-core` landed first; `molten-fastpath-core` followed once the Raft repairs were in.
  - `molten-fastpath-core` modules: `commute` (conflict contract), `envelope` (profile, quorums), `hazard` (fault corpus), `quorate` (stable-view admission, ledger), `succession` (recovery) and `witness` (evidence). They are renamed from `conflict`/`profile`/`fault`/`stable`/`recovery`/`evidence` for `path_segment_repetition`. `src/fabric_consistency/fastpath/mod.rs` is now a re-export shell at the old facade path.
  - The issue enums (`RecoveryIssue`, `LedgerIssue`, `ProfileIssue`, `ConflictContractIssue`, `FaultScenarioIssue`) implement `Display` and `core::error::Error`. The crate-root `Rejection` wraps each one. The shell's `fastpath::failure` maps every `Rejection` to `Failure::Classified(MalformedObservation)` and keeps the typed reason; the Quint harness's ledger rejection goes through it.
  - Fixed-width public signatures for `usize_in_public_api`: `derive_quorums(u32)`, `explore_fault_corpus(.., u32)`, `minimize_counterexample(.., u64)`, `ModelStep::sequence: u64` and `ModelRunEvidence::first_divergence: Option<u64>`. `ApplicationLedger::applied_count()` became `applied_operations()`, an iterator over the applied identities.
  - The unit tests moved to `crates/molten-fastpath-core/src/tests/`. The root keeps only the shell-boundary tests in `src/fabric_consistency/tests/fastpath/mod.rs`.
  - Audit correction: `src/harness/composition/` is not free of external crates. Its scenario drivers run `molten-core` system-extension/fabric-time/fabric-durability cores, the Wasm `SliceLedger`, and Preserves checkpoint hashing, so only the scenario/variant/fact vocabulary, property evaluation, and saturation slice contract moved.
  - `src/runtime/admission/` matched on the Preserves-backed `RuntimeValue`, so the core `ingress` policy is generic over the value and the shell instantiates it (`AdmissionStep` projects `RuntimeStep`).
  - Review decision: `src/authority/nominal.rs` stays in the shell and `molten_core::nominal` stays one file at its catalogued path. Octet's nominal-domain catalog and `checks.nominal-reference-domains` pin `crates/molten-core/src/nominal.rs`, and the wire DTOs are the declared compatibility scope of the `molten.authority.nominal_references.octet` requirement family. An earlier move of both into the crate was reverted, so `molten-runtime-admission-core` contains only `ingress`.
- [x] [serial] Register all extracted crates in `Cargo.toml`, `[workspace.metadata.octet]`, `.config/nextest.toml`, `flake.nix` (`no_std` compile gates + tests), and `build-plan.json`. r[molten.project.tier1_no_std_kernel_crates]
  - Done for all six extracted crates (2026-11-26): the four Tier-1 kernels first, then `molten-raft-core` and `molten-fastpath-core`. The crates are workspace members with release overflow-check profiles, are in `moltenOwnedCrates`, have a `no-std-kernels` nextest profile, and are in `[workspace.metadata.octet]`.
  - Done for `molten-raft-core` (2026-11-26): workspace member, release overflow profile, `[workspace.metadata.octet]`, `no-std-kernels` nextest filter, `moltenOwnedCrates`, `noStdKernels` entry (allowlist = the `molten-core` no_std closure; probes `src/transition.rs`/`src/transition/wire.rs`), `checks.molten-raft-core-no-std`, `scripts/no-std-kernel-plans.sh` entry, and `license-policy.tsv` package rule.
  - `molten-fastpath-core` also has a `noStdKernels` flake entry (`compileProbeModule = src/quorate.rs`, `sourceProbeModule = src/succession.rs`, empty linked allowlist), the check `molten-fastpath-core-no-std`, and a `scripts/no-std-kernel-plans.sh` entry.
  - Fast-path source paths were retargeted to the crate:
    - the `consensus-fastpath-model-profile` hygiene `rg` now scans `crates/molten-fastpath-core/src` and the shell;
    - `verification/tla/mutants/mixed-view-guard.patch` was regenerated against `crates/molten-fastpath-core/src/quorate.rs` (`fastpathMutantWs` rebuilds that path dependency);
    - `docs/` links and `ConsensusFastPath.tla` comments were updated.
  - Plans regenerated at integration: `build-plan.json` (`unit2nix --workspace`), `release-policy-build-plan.json` (`unit2nix -p molten-release-policy --bin molten-release-policy`), and every per-kernel plan via `scripts/no-std-kernel-plans.sh`. `Cargo.lock` gained only the six path packages.
  - Integration verification (2026-11-26, merge of both late kernels): `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test -p molten-fastpath-core -p molten-raft-core` 26 + 52 passed; `cargo test -p molten --lib -- fastpath fabric_consistency raft` 147 passed (1 ignored); `--test fastpathquint` 5 passed; `anix build` of `molten-fastpath-core-no-std`, `molten-raft-core-no-std`, `consensus-fastpath-quint`, `consensus-fastpath-model-profile`, `production-raft-model`, `license-boundary`, `inherited-tracey-debt`, and `git-source-hash-binding` all built; `scripts/no-std-kernel-plans.sh --check` and `scripts/git-source-hashes.sh --check` exit 0.
  - Octet: the first run found 472 findings. They were repaired rather than allowed:
    - `scripts/octet-qualify-imports.rs` plus manual fixes for the import findings;
    - module renames for `path_segment_repetition`: composition `property`/`scenario`/`saturation` → `judgment`/`vocabulary`/`quantum`; runtime-admission `admission` → `ingress`; `fault::FaultIssue` → `fault::Issue`; `history::Issue::Value` → `Issue::Output`;
    - a `refs::VecSink` sink for `borrowed_argument_types`;
    - splits into `component/verdict.rs` and `performance/identity.rs`.
  - Flake checks `molten-{composition,reliability-model,runtime-admission,wasm-policy}-core-no-std` cover the no_std build, tests, plan freshness, a linked-crate allowlist, and compile-time and source-scan probes. Each crate has its own plan, `nix/no-std-kernels/<crate>.build-plan.json` (regenerate with `scripts/no-std-kernel-plans.sh`). Each check:
    - builds the library from that plan;
    - builds and runs the crate's lib unit and Hegel tests through `ws.test.check.<crate>`. A planted failing test makes the check fail; see Verification.
    - asserts at eval time that the plan matches the crate manifest (default-features choice, requested features, root features), with a negative that flips a manifest feature;
    - asserts that every linked crate is on a per-kernel allowlist and that none enables `std`, with a negative that adds a std-only `tokio` crate without a `std` feature;
    - asserts that `<crate>.inputs.b3` records exactly the resolution inputs: workspace `Cargo.toml` and `Cargo.lock`, `.cargo/config.toml`, `rust-toolchain.toml`, and the manifests of every local package in the plan, i.e. the `cargo metadata` path-dependency closure. A negative drops one recorded input. The check then verifies those inputs' bytes with `b3sum --check`;
    - runs three `testBuildFailure` probes that inject `std::fs`/`std::time`/`std::env` imports into one module;
    - runs `scripts/check-no-std-sources.py`, which rejects `extern crate std` and `std::` paths in production sources, with negative probes in a different module.
- [x] [serial] Update `docs/architecture.md` and `docs/modularity-boundaries.md`, then run focused tests, Hegel properties, Clippy, workspace Octet, Nix checks, and strict Cairn validation. r[molten.consensus.no_std_raft_and_reliability_kernels] r[molten.wasm.no_std_wasm_policy_kernel] r[molten.project.tier1_no_std_kernel_crates]
  - Done (2026-11-26). `docs/modularity-boundaries.md` has a new section, "No_std kernel crates" (`#no-std-kernel-crates`). For each kernel it records what the crate owns, its linked dependencies, the `std` shell in `src/`, and how refusals cross the boundary. Raft maps `ReplicaErrorClass` to `Failure::InvalidHarness`, `Classified(MalformedObservation)` or `Preserves`. Fast path maps every `Rejection` to `Classified(MalformedObservation)`. The reliability issues become `InvalidHarness` at the callers. The Wasm `Denial` is re-exported unchanged. Composition and runtime admission return outcomes and decisions rather than errors. The section also lists the `<kernel>-no-std` enforcement (plan freshness, linked-crate allowlist, build and tests, compile probes, source scan) with each kernel's probe modules.
  - The same doc now lists the kernels in the Core dependency class, adds the kernel checks to the validation paths, and corrects the Wasm performance policy location.
  - `docs/architecture.md`: the layer map has a kernel box, and a summary paragraph follows the map. The consensus table now names `molten-raft-core`. The stale "lives in the root crate, not in `molten-core`" sentence is replaced with the kernel, identity port, shell and failure mapping. `consensus_recovery` is attributed to `molten-core` and called from `molten-raft-core`. The runtime admission gate is attributed to `molten-runtime-admission-core`.
  - Octet fix within this change: workspace Octet reported `no_unwrap` at `tests/fastpathquint.rs:91`. The finding came from this change's proposer-acknowledgement commit (`derive_quorums(5).expect(..)`). `fast_commit` now returns `quint_connect::Result` and propagates a missing quorum derivation as an error.

## Verification (2026-11-26, reliability/wasm-policy/composition/runtime-admission kernels)

Final state, after the review fixes:

- `aremote 'cargo check --workspace --all-targets'`: pass, no warnings. Crates are `#![cfg_attr(not(test), no_std)]` with `extern crate alloc`.
- `aremote 'cargo test -p molten-reliability-model-core -p molten-wasm-policy-core -p molten-composition-core -p molten-runtime-admission-core -p molten-core --lib'`: composition 4, reliability model 18, runtime admission 4, wasm policy 16, and molten-core 793 tests pass, including Hegel properties.
- `aremote 'cargo test -p molten --lib -- live_reliability wasm_component wasm_performance harness::composition harness::tests runtime::ingress authority::nominal plugin::host reliability::service'`: 179 passed, 0 failed.
- `aremote 'cargo clippy -p molten-reliability-model-core -p molten-wasm-policy-core -p molten-composition-core -p molten-runtime-admission-core -p molten-core -p molten --all-targets -- -D warnings'`: clean.
- Octet: the first `aremote 'cargo octet check -p <four crates> --artifact-dir target/octet-kernels'` run found 472 findings, all repaired (see task 5). The full `aremote 'cargo octet check --artifact-dir target/octet'`, whose default scope now includes the four crates, is `Status: clean` with 0 findings.
- Plans regenerated: `aremote 'unit2nix --workspace -o build-plan.json'`, `aremote 'unit2nix -p molten-release-policy --bin molten-release-policy -o release-policy-build-plan.json'`, and `MOLTEN_ROOT=. scripts/no-std-kernel-plans.sh`. `scripts/no-std-kernel-plans.sh --check` and `scripts/git-source-hashes.sh --check` both exit 0.
- `anix build --print-out-paths` built all of:
  - checks `molten-composition-core-no-std`, `molten-reliability-model-core-no-std`, `molten-runtime-admission-core-no-std`, `molten-wasm-policy-core-no-std`;
  - checks `nominal-reference-domains`, `release-dependency-profile`, `dependency-audit-scanner-fixture`, `docs-embed-hygiene`, `wasm-component-profile`, `wasm-component-performance-profile`, `git-source-hash-binding`, `release-overflow-checks`;
  - `.#packages.x86_64-linux.molten-release-policy`.
- Planted-failure proof: a temporary `#[test] fn planted_failure_must_fail_the_no_std_check() { assert_eq!(1 + 1, 3) }` in `crates/molten-composition-core/src/tests/mod.rs` made `anix build .#checks.x86_64-linux.molten-composition-core-no-std` fail ("test result: FAILED. 4 passed; 1 failed", exit 1). The test was then removed and the check rebuilds.
- `python3 scripts/check-doc-links.py --root .`: the four `crates/molten-core/src/nominal.rs` links resolve again. The 2 remaining findings are pre-existing `docs/linux-ebpf-observability.md` anchors into another change.
- `cairn validate --root . --policy <sibling cairn-policy.json> --strict`: `valid: true`.

## Verification (2026-11-26, molten-raft-core)

- `aremote 'cargo check -p molten-raft-core --no-default-features'`: pass.
- `aremote 'cargo test -p molten-raft-core'`: 52 passed, 0 failed (incl. the two Hegel properties and the moved liveness/bounds/rejoin/repair regressions).
- `aremote 'cargo test -p molten --lib -- fabric_consistency raft reliability live_reliability'`: 222 passed, 0 failed, 1 ignored.
- `aremote 'cargo check --workspace --all-targets'`: pass (rerun exactly after the Octet fixes).
- `aremote 'cargo clippy -p molten-raft-core -p molten --all-targets -- -D warnings'`: clean.
- `aremote 'cargo test -p molten --test collaborationnative'` (the only `tests/` suite naming Raft): 10 passed, 4 ignored. `Cargo.lock` gained only the `molten-raft-core` path package (pulled with `apull`).
- Plans (local): `unit2nix --workspace -o build-plan.json`, `bash scripts/no-std-kernel-plans.sh`; `bash scripts/no-std-kernel-plans.sh --check` and `bash scripts/git-source-hashes.sh --check` exit 0.
- Octet: `aremote 'cargo octet check --artifact-dir target/octet'` first reported 20 findings in touched files (`module_file_count` for `src/`, `unchecked_division` in the property harness, `path_segment_repetition` test names, one `function_length`). All were repaired (`refs` and `wire` moved under `denial`/`transition`, test modules renamed, the crash test split). The rerun has no finding in any touched file. The remaining workspace findings are in untouched files. `aremote 'cargo octet check -p molten-raft-core --artifact-dir target/octet-raft'`: `Status: clean`, 0 findings.
- `anix build -L .#checks.x86_64-linux.molten-raft-core-no-std`: pass (plan admission, 52 lib/Hegel tests, three std compile probes, source-scan probes). `.#checks.x86_64-linux.production-raft-model`: pass. `.#checks.x86_64-linux.license-boundary`: valid (27 rules) with the four other kernels' rules the integration branch already carries temporarily added; without them it fails only on `molten-composition-core`.
- `cairn validate --root . --policy …/cairn-policy.json --strict`: the only error is the known unknown `molten.node_host.crate_boundary` requirement reference in `fix-authority-bypasses-and-sandbox-isolation/tasks.md`, fixed on the integration branch.

## Verification (2026-11-26, FastpathCore: molten-fastpath-core and the proposer acknowledgement)

- `aremote 'cargo check -p molten-fastpath-core --no-default-features'`: pass. `aremote 'cargo test -p molten-fastpath-core'`: 22 passed, 0 failed, including the Hegel property (superseded by the review-fix run below).
- `aremote 'cargo check --workspace --all-targets'`: pass.
- `aremote 'cargo test -p molten --lib -- fastpath fabric_consistency'`: 182 passed, 0 failed, 1 ignored. This includes the two shell-boundary tests, which map each `Rejection` family to `MalformedObservation`.
- `aremote 'cargo test -p molten --features test-fixtures --test fastpathquint'`: 5 passed, including `proposer_acknowledgement_replay`.
- `aremote 'cargo clippy -p molten-fastpath-core -p molten --all-targets -- -D warnings'`: clean.
- Octet:
  - `aremote 'cargo octet check -p molten-fastpath-core --artifact-dir target/octet-fastpath'`: `Status: clean`, 0 findings. The first run had 16 findings; all were repaired with no allows:
    - `&mut Vec` profile helpers → `Option`-returning issue helpers;
    - test module renames;
    - `Rejection` moved to the crate root for `module_file_count`.
  - Workspace `aremote 'cargo octet check --artifact-dir target/octet'`: 1051 findings, none in this slice's files: the crate, `src/fabric_consistency/fastpath`, `src/fabric_consistency/tests/fastpath`, and `tests/{fastpathquint.rs,parts/fastpathquint}`. All 1051 are in other files of the integration base.
- Plans: `aremote 'unit2nix --workspace -o build-plan.json'`, `aremote 'unit2nix -p molten-release-policy --bin molten-release-policy -o release-policy-build-plan.json'` and `aremote 'MOLTEN_ROOT=. scripts/no-std-kernel-plans.sh'`. The `--check` run exits 0. `Cargo.lock` gained only the `molten-fastpath-core` path package.
- `anix build -L --print-out-paths .#checks.x86_64-linux.<name>`, all built:
  - `molten-fastpath-core-no-std`: 22 Nix-built tests passed; the `std::fs`, `std::time` and `std::env` probes failed as required in `src/quorate.rs`.
  - `consensus-fastpath-model-profile`: passed.
  - `consensus-fastpath-quint`: `campaign.json` status `passed`. Normal replay gave `5 passed; 0 failed`. The mutated-guard replay exited 101 with `State invariant failed`, from the regenerated patch on `crates/molten-fastpath-core/src/quorate.rs`.
  - `consensus-fastpath-model-parity`: passed, with 6/6 reader tests and campaign status `passed`.
  - `license-boundary`: `27 rules` valid. `license-policy.tsv` now has a `package` rule for every workspace crate.
- `python3 scripts/check-doc-links.py --root .`: 320 files, 0 findings. `python3 scripts/check-no-std-sources.py crates/molten-fastpath-core`: exit 0.
- Task 5 stays open for `molten-raft-core` only; task 6 stays open (architecture docs and repository-wide validation).
- `cairn validate --root . --policy <sibling cairn-policy.json> --strict` reports only the pre-existing unknown-requirement error in `fix-authority-bypasses-and-sandbox-isolation/tasks.md`, which is unchanged since `24ddd0479` and already fixed on the integration branch. `cairn gate tasks` is PASS for both this change and `align-formal-models-architecture-and-governance-toolchains`.

### Review fixes (2026-11-26, FastpathCore)

- `derive_quorums` computes in `u64`; `quorums_derive_past_the_u32_superquorum_product_boundary` covers 1,431,655,765, 1,431,655,766 and `u32::MAX`.
- Admission subsets:
  - `every_five_replica_acknowledger_and_proposer_subset_admits_exactly_when_proposers_acknowledged` (deterministic; 32 × 31 subset pairs);
  - `two_active_proposers_both_must_be_among_the_acknowledgers`;
  - `hegel_seven_replica_admission_requires_every_proposer_in_the_superquorum`.
  - The Quint replay driver now runs five admitted replicas with `derive_quorums(5)`, so `unacknowledged` omits the admitted proposer from a 4-of-5 superquorum.
- Simulation regression for the repair change's fast-path defect: `tests::cluster::simulated_replicas_converge_on_session_keyed_ledgers_and_ignore_non_member_acknowledgements`.
  - Setup: five replicas, each with its own `ApplicationLedger`, over 48 seeded interleavings. Seed 0 delivers non-member acknowledgements first.
  - It checks that equal commands from two sessions apply exactly once and in commit order on every replica, that a conflicting session retry is rejected everywhere, that non-member acknowledgements never complete a stable view, and that the applied sequences converge across replicas.
  - Guard check, by temporarily mutating `quorate.rs` on leviathan:
    - keying the ledger by `command_ref` fails the test (panic at the applied-sequence assertion);
    - dropping the member filter in `same_view_acknowledgers` fails the test (panic at the admission assertion);
    - restored source passes.
- Re-verification after the review fixes:
  - `aremote 'cargo test -p molten-fastpath-core'`: 26 passed.
  - `aremote 'cargo octet check -p molten-fastpath-core --artifact-dir target/octet-fastpath'`: clean, 0 findings.
  - `aremote 'cargo clippy -p molten-fastpath-core -p molten --all-targets -- -D warnings'`: clean.
  - `aremote 'cargo test -p molten --features test-fixtures --test fastpathquint'`: 5 passed.
  - `anix build -L .#checks.x86_64-linux.consensus-fastpath-quint`: campaign `passed`, normal replay `5 passed`, and the mutated guard failed with `State invariant failed`.
  - `anix build -L .#checks.x86_64-linux.molten-fastpath-core-no-std`: 26 Nix-built tests passed and the std probes were rejected.

## Verification (2026-11-26, task 6: docs and final validation)

- `aremote 'cargo test -p molten-composition-core -p molten-reliability-model-core -p molten-runtime-admission-core -p molten-wasm-policy-core -p molten-raft-core -p molten-fastpath-core'`: 122 passed, 0 failed. By crate: composition 4, fast path 26, raft 52, reliability model 18, runtime admission 6, Wasm policy 16. The total includes all 10 Hegel properties (1, 1, 2, 2, 2, 2).
- `aremote 'cargo test -p molten --lib -- fabric_consistency raft fastpath reliability wasm_component harness::composition runtime::admission'`: 311 passed, 0 failed, 1 ignored.
- `aremote 'cargo test -p molten --features test-fixtures --test fastpathquint'` (after the `no_unwrap` fix): 5 passed.
- `aremote 'cargo clippy --workspace --all-targets -- -D warnings'`: clean. Rerun after the fix: clean.
- `aremote 'cargo octet check --artifact-dir target/octet'`: `Status: warning-only`, 1030 findings before the fix, 1029 after, 0 errors.
  - None of the 1029 is in the six kernel crates, in `src/fabric_consistency/fastpath`, in `src/fabric_consistency/raft/kernel*`, in `tests/fastpathquint.rs`, or in the edited docs.
  - The remaining findings are in files this change does not touch. 1016 are in `molten`, 9 in `molten_node_host`, 2 in `nativesystemextension`, 1 in `executable_extent` and 1 in `cliharness`. Raft examples: `src/fabric_consistency/raft/live_cluster/contexts.rs:19,43`, from `a0db999d9` (Raft peer authentication).
- `anix build -L --print-out-paths` built all of:
  - `.#checks.x86_64-linux.{molten-raft-core,molten-fastpath-core,molten-reliability-model-core,molten-wasm-policy-core,molten-composition-core,molten-runtime-admission-core}-no-std`. The Nix-built tests reported raft 52, reliability model 18, Wasm policy 16, runtime admission 6 and composition 4 passed. The fast-path test derivation printed no new output because it was already built. Its `std` compile probe failed with `E0433` as required.
  - `.#checks.x86_64-linux.production-raft-model`.
  - `.#checks.x86_64-linux.consensus-fastpath-quint`, rebuilt with the fixed `tests/fastpathquint.rs`. The mixed-view-guard mutant was patched into `crates/molten-fastpath-core/src/quorate.rs`.
- Local checks:
  - `python3 scripts/check-doc-links.py --root .`: 320 files, 0 findings.
  - `python3 scripts/check-no-std-sources.py crates/molten-{composition,reliability-model,runtime-admission,wasm-policy,raft,fastpath}-core`: exit 0.
  - `bash scripts/no-std-kernel-plans.sh --check`: exit 0.
  - `cargo fmt --all`: no changes beyond the fix.
- `cairn validate --root . --policy <sibling cairn-policy.json> --strict`: `valid: true`. `cairn gate tasks extract-no-std-raft-and-consensus-kernels`: verdict `PASS`.
