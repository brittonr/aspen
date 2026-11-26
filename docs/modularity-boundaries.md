# Aspen modularity boundary inventory

<a id="molten-modularity-boundary-inventory"></a>

This inventory records the modularity slices that are in place, as evidence for the active Cairn modularity changes. It is intentionally scoped: it defines ownership, pure cores, shell responsibilities, and focused checks without claiming full crate extraction or whole-system correctness.

## Core and dependency classes

`crates/molten-core/` holds in-memory decision cores, transitions, typed identities, and plans. It performs no filesystem, process, network, or clock access and links no execution or backend runtime. It is not dependency-free. Its [manifest](../crates/molten-core/Cargo.toml) declares these external dependencies:

| Dependency | Source |
| --- | --- |
| `blake3`, `serde` | crates.io |
| `artifact-auth-core`, `artifact-binding-core` | Git, `OnixResearch/onix-artifact` |
| `basalt` | Git, `OnixResearch/basalt` |
| `choregraph-history` | Git, `OnixResearch/choregraph` |
| `kamacite-core` | Git, `OnixResearch/kamacite` |
| `content-identity-core`, `schema-migration-core`, `transactional-reconciliation-core` | Git, three `seed.radicle.garden` repositories |
| `executable-extent-core` (optional, `executable-extents` feature) | Git, `seed.radicle.garden` |

Every Git dependency is pinned to a reviewed revision. `cargo test -p molten-core` validates the crate. The root crate depends on it and re-exports it through `molten::core_api` and `molten::prelude`.

<a id="molten-core-no-std-scope"></a>

r[impl molten.project.molten_core_no_std] `molten-core` is `#![no_std]` + `alloc` whenever its default `std` feature is off (`#![cfg_attr(not(any(feature = "std", test)), no_std)]`). In that build, `src/lib.rs` replaces the standard prelude with `core`'s prelude plus the `alloc` items that the standard prelude would otherwise provide (`Vec`, `String`, `Box`, `ToString`, `ToOwned`, `format!`, `vec!`), through a crate-private `#[prelude_import]`. As a result, module sources look the same under both profiles and need no per-file imports that would trip Tigerstyle's `non_trait_imports` lint. Other items are named through `alloc` (collections, `alloc::string`, `alloc::vec`) or `core` (`fmt`, `cmp`, `mem`, `net`, `hint`, `error`, `iter`, `slice`, `str`, `ops`), never through `std`. `blake3` and `serde` are declared with `default-features = false` (`serde` keeps `alloc` and `derive`); the `std` feature turns their `std` features back on. Exactly three modules need the `std` feature, because their upstream dependencies still require the standard library:

| `std`-gated module | Upstream dependency |
| --- | --- |
| `live_binding` | `kamacite-core` |
| `world_merge` (migration identity via `kamacite_core::encode_typed_preserves`) | `kamacite-core` |
| `world_branch_authority` | `basalt` |

Every other module, including `executable_extent` behind `executable-extents`, compiles under `no_std`. `std` is a default feature so the root crate, `molten-node-host`, `molten-release-policy`, and the focused Octet workspaces keep their current API; a consumer that wants only the pure cores depends on `molten-core` with `default-features = false`. Crates that mount `molten-core` module sources directly (the `checks/*-octet` roots and `pilots/orchestration/adapters`) declare `extern crate alloc;` whenever a mounted module names an `alloc::` path. Unit tests always link the standard library, so test modules may still use `std`, and the same tests run under both feature profiles.

r[impl molten.testing.molten_core_no_std_ci_gate] The `molten-core-no-std` flake check builds `molten-core` from its own unit graph (`molten-core-no-std-build-plan.json`, generated with `unit2nix -p molten-core --no-default-features --features executable-extents`) and checks, at evaluation time, that the plan enables no `std` feature on `molten-core` or on any crate linked into it and does not contain `basalt` or `kamacite-core`. The check then:

- **Plan freshness.** It runs the pinned `unit2nix` with the same flags in a copy of the source and requires "Build plan is up to date". That message appears only when the recorded `inputsHash` matches the current fingerprint (CLI flags, `Cargo.lock`, and every `Cargo.toml` in the tree). No cargo is available inside the check, so a stale plan fails, and the plan must be regenerated after any manifest change. As a negative, it flips `serde` back to its default (`std`) features in a copy of `crates/molten-core/Cargo.toml` and requires the freshness test to fail.
- **Source guard.** It runs `scripts/check-molten-core-no-std-sources.sh`. The guard rejects any `extern crate` except `extern crate alloc;` in `lib.rs`. It also rejects any `std` path or `use std` outside the `std`-gated modules and test files (`tests.rs`, `**/tests/**`), including imports hidden behind `#[cfg(feature = "std")]`. Compilation alone misses an explicit `extern crate std;` in some module, because the library still builds. So the check plants `extern crate std; use std::fs::read;` in `planning.rs` and a feature-gated `use ::std::time::SystemTime;` in `codec.rs`, and requires the guard to reject both.
- **Compilation.** It compiles the library with `--no-default-features` and with `--no-default-features --features executable-extents`, and runs the unit tests for the no_std feature set.
- **Compile probes.** It appends `use std::fs::read;`, `use std::time::SystemTime;`, and `use std::env::var;` in turn to `src/codec.rs`. Each of those builds must fail with rustc's unresolved-`std` diagnostic for that file.

The `molten-core-tests` flake check runs the unit tests under the default `std` feature set. Compiling under `no_std` rules out ambient OS input/output at compile time. It does not prove that the logic is correct, it does not replace runtime authority gates, and it says nothing about embedded hardware targets.

Cargo dependency classes for the current root crate:

| Class | Dependencies | Boundary rule |
| --- | --- | --- |
| Core | `molten-core`, `blake3` for identity-only helpers | Pure functions over in-memory values; no filesystem, process, network, clock, Redb, Wasmtime, Steel execution, Nickel runtime, Iroh, or CLI shells. |
| Codec | `preserves`, `preserves-schema`, `serde`, `serde_json`, `toml`, `zstd`, `tar` | Encodes, parses, and hashes canonical artifacts; shared codec helpers must not import high-level domain modules. |
| Policy/evidence | `basalt`, `cairn`, `octet`, `trellis`, `nickel-lang` as authoring or evidence inputs | Constructs and verifies evidence-only values; does not grant authority or runtime trust by itself. |
| Runtime | `syndicate`, `tokio`, `n0-future`, `tracing` | Produces deterministic state transitions and planned effects after admission; shells execute effects. |
| Adapter | `iroh`, `iroh-blobs`, `iroh-docs`, `iroh-gossip`, `redb`, `wasmtime`, `wasmtime-wasi`, `wit-bindgen`, `wasmparser`, `steel-core` | Owns filesystem, transport, store, executor, and hostcall mechanics. Adapter availability is never trust. |
| CLI | `clap` | Parses flags and paths, reads/writes files, renders output, and exits. |
| Test/integration | `hegeltest`, `wat`, Nix VM checks, dogfood checks | Produces evidence for selected fixtures and integration profiles only. |

Dependency-direction rules are authored as Nickel data in `docs/dependency-boundary-policy/valid.ncl`; negative fixtures cover duplicate rules, invalid layers, and empty allow/deny targets. That file is policy data only: no Rust code consumes it, and it does not by itself block an import. Import direction is enforced by the ast-grep [runtime-authority audits](ast-grep-runtime-authority-audits.md) and the Octet source gate. An earlier `molten_core::dependency` import-fact validator had no production caller and was removed.

## Filesystem and Nickel boundaries

`molten-core` contains no `std::fs` calls. The root crate is not uniformly pure. `std::fs::` appears at 781 call sites across 236 files under `src/`. They concentrate in CLI shells (`src/cli/`), node state and daemon shells (`src/node/`), the cluster harness, component RPC, Wasm tooling, and Raft durability. Ongoing node I/O is meant to go through `NodeStateRoot` and typed store roots. See [node-state filesystem authority](node-state-filesystem-authority.md). A `std::fs` call outside those roots is a review finding, not an approved pattern.

Only the harness schema evaluates Nickel in-process: its policy, budget, and capability source checks call `nickel_export_json` in `src/harness/parts/schema/p020/body.rs`, which uses `nickel-lang`. Job-graph construction lowering under the `job-dag-construction` feature (`src/job/authoring/construction*.rs`) does not evaluate in-process. It admits captured modules through `nickel-eval` and runs them in the separate `molten-nickel-eval-worker` process, launched only through `nickel_eval::captured::evaluate_isolated` with an explicitly selected path.

Fixture tests may also run the `nickel` CLI. Policy source (`cairn-policy/default.ncl`, `cairn-policy/contracts.ncl`, and docs contracts) is authored in Nickel and exported to checked JSON. The `contract-export-drift-gate` flake check re-exports `cairn-policy/default.ncl` and fails on any difference from `cairn-policy/generated/cairn-policy.json`. Runtime admission may consume checked exports, canonical refs, or policy-gate receipts; it must not treat Nickel evaluation, Cairn policy export, or policy tooling availability as live authority. An earlier `molten_core::policy` freshness validator duplicated that drift gate without a caller and was removed.

## Stack evidence adapter boundary

Stack inputs from Basalt, UCAN, Trellis, Octet, Valence, Cairn, and Mantle are normalized into evidence-only envelope members. The contract and fixtures live under `docs/stack-evidence-envelope/`:

- `valid.ncl` covers complete Basalt, UCAN, Trellis, Octet, Valence, Cairn, and Mantle refs.
- `missing-role.ncl` denies incomplete envelopes.
- `stale-ref.ncl` denies unsupported/stale ref forms.
- `overbroad-claim.ncl` denies claims that grant runtime authority.

Approved upstream-specific crate usage is limited to adapter, policy/evidence, CLI, or release-evidence shells. Pure runtime cores consume only parsed facts, canonical refs, verification roles, and non-claim summaries.

## Evidence, policy, runtime, and adapter ownership

Representative workflow: a runtime operation with evidence-backed admission.

1. Evidence modules parse or verify receipt values and produce evidence summaries. Evidence-only receipts do not grant authority, provenance trust, transport trust, retention authority, execution permission, or replay trust by themselves.
2. Policy modules make deterministic admission decisions over explicit authority, policy, resource, provenance, retention, and evidence inputs.
3. Runtime modules consume admitted inputs and return planned effects.
4. Adapter shells execute store, transport, executor, clock, or receipt writes after the plan is admitted.

Each domain owns its own admission decision; there is no shared boolean planner. Retention's `admit_destructive_evidence_with_root` is a representative example: it resolves referenced authority, policy, supporting-evidence, and clearance records before any delete. `molten_core::planning` now carries only the `EffectKind` port vocabulary that the world-replay boundary classifies. The earlier generic `plan_*` planners took caller-asserted booleans, had no production caller, and were removed.

## CLI imperative shell boundary

The CLI shell owns Clap parsing, path reads, stdout/stderr, process exit, and output files. Command handlers call the domain entry points (`molten::chunk_store`, `molten::retention`, `molten::job_dag`, `molten::node_daemon`, and others) with parsed inputs.

## Semantic module layout

The chunk store (`src/chunk/store.rs`), retention (`src/retention/mod.rs`), job DAG (`src/job/dag.rs`), harness schema (`src/harness/schema.rs`), and node daemon (`src/node/daemon.rs`) are still assembled from ordinal `parts/**/body.rs` shards with `include!`. They share one module scope. Because `include!`d spans count as macro expansions, Octet's item-level lints (`path_segment_repetition`, `non_trait_imports`, and others) do not inspect those shards. Converting them into real submodules is staged work under the `align-formal-models-architecture-and-governance-toolchains` change and must:

- re-export every public item explicitly from the entrypoint so `molten::chunk_store::*`, `molten::retention::*`, `molten::job_dag::*`, and `molten::node_daemon::*` keep their paths;
- reference other modules by qualified owner path, because Octet's `non_trait_imports` rejects non-trait `use`, and replace shared-scope aliases (`type Result<T>`, forwarding wrappers around `preserves_rail`) with their owner paths;
- name modules so that no contained item repeats a module word (`path_segment_repetition`), and keep each file within 300 lines and each directory within seven Rust files;
- start every item private and raise its visibility only where another module uses it;
- leave `src/job/selected/` and `src/job/authoring/` textually included and unedited. Their source text is hashed into selected-stage signatures (`selected_stage_signature`) and job-lowering provenance refs. Editing them would change canonical job identities, so the job DAG conversion must keep the names they use resolvable in the `workload` scope.

The internal root names (`blocks`, `custody`, `workload`, `daemon_core`, and `testbed::format`) and their `compat_module!` aliases stay: callers use the aliases, and making an alias the real module would repeat words such as `chunk` or `retention` in many public item names.

Physical source groups also keep domain directories below eight sibling Rust files without adding lexical modules. Coordination-delivery profile and canonical-record tests live in `src/coordination_delivery/tests/contracts/`. Wasm performance profile and optimization code and their tests live in `src/wasm/performance/policy/` and `src/wasm/performance/tests/policy/`. World-benchmark port and canonical-record contracts live in `src/world_benchmark/contracts/`. World-distribution catalog loading and synchronization execution live in `src/world_distribution/sync/`. Existing `mod.rs` loaders use `#[path]` to retain the original module names, visibility, and `super` relationships.

## Chunk store semantic boundary

| Area | Owner | Boundary |
| --- | --- | --- |
| model and codec | `ChunkRef`, `ChunkManifest`, receipt DTOs, Preserves constructors/parsers | In-memory DTOs, canonical bytes, and BLAKE3 refs. |
| verify | manifest/chunk/receipt checks | Validation over parsed values. |
| fs_store and index | file writes, Redb tables | Shell and Redb adapter. |
| exchange | peer sync, Iroh publish/fetch, remote range readback | Transport adapter shell. |
| retention | pin/unpin/GC admission | Decisions before deletes. |
| lineage | manifest lineage and receipt chains | Evidence-only summaries. |

`molten_core::codec::validate_domain_artifact` is the first domain-owned codec façade used by chunk manifest parsing after canonical ref computation; it validates supported labels, schema identity, and BLAKE3 ref shape before callers consume the parsed manifest. `gc_with_root` deletes only targets whose retention evidence and apply refs pass, and writes a receipt either way.

## Preserves boundary adoption profile

<!-- r[impl molten.runtime_spine.preserves_boundary_profile.docs] -->
<!-- r[impl molten.runtime_spine.preserves_boundary_profile.docs.non_claims] -->

The Preserves adoption profile lives under `docs/preserves-boundary-profile/`. It records artifact family, schema label, canonical-byte requirement, BLAKE3 identity field, adapter owner, core DTO boundary, allowed consumers, and non-claims for node control envelopes, tickets, workflow bundles, receipts, and evidence envelopes. The Nickel contract in `contracts.ncl` is the validator: `valid.ncl` applies it to the shared `fixture.ncl`, and the `contract-export-drift-gate` flake check requires every negative fixture to fail on that contract. A duplicate Rust validator (`molten_core::preserves_profile`) had no production caller and was removed.

Valid fixtures cover canonical node control, ticket, workflow bundle, receipt, and evidence envelope measurements. Negative fixtures fail closed for non-canonical bytes, missing schema labels, stale or uppercase BLAKE3 refs, unsupported consumers, missing non-claims, and raw-Preserves core coupling. Profile success proves canonical boundary identity and adapter placement only; it does not prove transport liveness, actor authority correctness, replay completeness, or Valence Evidence IR acceptance.

## Domain codec façade

The selected high-fan-in Preserves domain is the chunk store. It directly constructs and parses chunk manifests, chunk refs, lineage, receipts, and adapter tickets. The first façade is `molten_core::codec::validate_domain_artifact`, called from chunk manifest parsing to keep domain identity checks in a small in-memory core while preserving canonical Preserves bytes and refs. Positive core tests accept the current chunk manifest schema/ref contract; negative tests reject unsupported labels, schema drift, malformed refs, and missing domain identity.

Shared broad codec helpers remain in `preserves_rail`; they must not import high-level runtime, node, retention, job, plugin, CLI, or adapter domains.

## Retention core/store boundary

Retention ownership: admission (plan gates, destructive-evidence admission, remote clearance refs); plan, apply, execution gate, and audit records; lifecycle and candidate bundles; remote clearance exchange; and store/receipt persistence. `apply_gc_plan_with_root` recomputes the stored plan and re-runs destructive admission before writing an apply receipt or tombstone. Negative tests cover missing authority, missing remote clearance, stale plans, and incomplete indexes.

## Job DAG and node daemon boundaries

Job DAG planning, admission, and scheduling decisions are separate from worker, blob I/O, and remote execution shells. Node daemon state-root I/O, live Iroh transport, and service-loop orchestration are separate from workflow bundle, grant, and ticket decisions and their receipt encoders.

## Harness schema/gate modularity

Harness schema DTOs and report checks are in-memory validation. Repro bundles, redaction, and policy, capability, and UCAN gate evidence are separate concerns. Fixture I/O and command orchestration stay in the harness shell.

## Evidence ledger and registry/catalog boundary

Evidence parsers and verifiers should run over in-memory values before local ledger persistence. Ledger storage owns content-addressed persistence. Registry/catalog modules own classification, search, MCP read-only views, and operator discovery. Registry or catalog presence is evidence only and does not grant authority, provenance, policy, retention, source-gate, execution, or replay trust.

## Operator dogfood and integration boundary

Dogfood, prod-soak, and NixOS VM modules are integration shells. Runtime/node cores must not import operator dogfood, production soak, or VM modules. Their receipts remain evidence-only and do not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation, deployment, or release trust.

## Public API surface

Preferred stable API for new boundary work:

- `molten::core_api::*` for explicit core modules;
- `molten::prelude::*` for ergonomic core and stack evidence imports;
- existing compatibility aliases such as `molten::chunk_store`, `molten::node_runtime`, and `molten::job_dag` remain preserved.

Current public root modules are classified as:

- stable API: `prelude`, `core_api`, `Failure`, `Result`;
- compatibility aliases: domain aliases created with `compat_module!`;
- internal implementation compatibility: `#[doc(hidden)]` root modules that back aliases;
- generated/test support: remaining ordinal `parts/**` shards and `test_support`.

No hidden module is removed in this slice because the compatibility aliases re-export the historical surface and callers use them. Removing a hidden module requires a separate change that proves all documented aliases and downstream imports continue to compile.

## Validation paths

Focused checks for this boundary slice:

```sh
cargo test -p molten-core
cargo test -p molten-core --no-default-features --features executable-extents
cargo check -p molten-core --no-default-features
nix build .#checks.x86_64-linux.molten-core-no-std
cargo test --lib
cargo test -p molten --lib chunk_store
cargo test -p molten --lib retention
cargo fmt --check
cargo octet check --artifact-dir target/octet
nix build .#checks.x86_64-linux.contract-export-drift-gate
nickel export docs/dependency-boundary-policy/valid.ncl
nickel export docs/stack-evidence-envelope/valid.ncl
```

Negative Nickel fixtures under `docs/dependency-boundary-policy/`, `docs/preserves-boundary-profile/`, and `docs/stack-evidence-envelope/` must fail to export. These checks are evidence for positive and negative contract coverage, not production deployment trust.
