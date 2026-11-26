# Tasks

- [ ] [serial] Extract `crates/molten-raft-core` (`#![no_std]` + `alloc`) from `src/fabric_consistency/raft/{transition,ports,parts}` with typed core errors, standard module files (no `include!()` shards), and thin Tokio/Iroh/Redb shells remaining in `src/fabric_consistency/raft/`. r[molten.consensus.no_std_raft_and_reliability_kernels] r[molten.project.tier1_no_std_kernel_crates]
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
- [ ] [serial] Register all extracted crates in `Cargo.toml`, `[workspace.metadata.octet]`, `.config/nextest.toml`, `flake.nix` (`no_std` compile gates + tests), and `build-plan.json`. r[molten.project.tier1_no_std_kernel_crates]
  - Done for the four extracted crates (2026-11-26), and for `molten-fastpath-core` on the same date; left open only for `molten-raft-core`. The crates are workspace members with release overflow-check profiles, are in `moltenOwnedCrates`, have a `no-std-kernels` nextest profile, and are in `[workspace.metadata.octet]`. `molten-fastpath-core` also has a `noStdKernels` flake entry (`compileProbeModule = src/quorate.rs`, `sourceProbeModule = src/succession.rs`, empty linked allowlist), the check `molten-fastpath-core-no-std`, and a `scripts/no-std-kernel-plans.sh` entry.
  - Fast-path source paths were retargeted to the crate:
    - the `consensus-fastpath-model-profile` hygiene `rg` now scans `crates/molten-fastpath-core/src` and the shell;
    - `verification/tla/mutants/mixed-view-guard.patch` was regenerated against `crates/molten-fastpath-core/src/quorate.rs` (`fastpathMutantWs` rebuilds that path dependency);
    - `docs/` links and `ConsensusFastPath.tla` comments were updated.
  - Plans regenerated: `build-plan.json` (`unit2nix --workspace`) and `release-policy-build-plan.json` (`unit2nix -p molten-release-policy --bin molten-release-policy`). `Cargo.lock` gained only the four path packages (later `molten-fastpath-core`).
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
- [ ] [serial] Update `docs/architecture.md` and `docs/modularity-boundaries.md`, then run focused tests, Hegel properties, Clippy, workspace Octet, Nix checks, and strict Cairn validation. r[molten.consensus.no_std_raft_and_reliability_kernels] r[molten.wasm.no_std_wasm_policy_kernel] r[molten.project.tier1_no_std_kernel_crates]

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
