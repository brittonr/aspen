# Molten modularity boundary inventory

This inventory records the first production-readiness modularity slice. It is evidence for the active Cairn modularity changes and is intentionally scoped: it defines ownership, pure cores, shell responsibilities, and focused checks without claiming full crate extraction or whole-system correctness.

## Core and dependency classes

`crates/molten-core/` is the minimal pure build surface. It has no dependencies beyond the Rust standard library and is validated with `cargo test -p molten-core`. The root crate depends on it and re-exports it through `molten::core_api` and `molten::prelude` so existing root-crate compatibility aliases keep working while new code has a stable preferred API.

Cargo dependency classes for the current root crate:

| Class | Dependencies | Boundary rule |
| --- | --- | --- |
| Core | `molten-core`, `blake3` for identity-only helpers | Pure functions over in-memory values; no filesystem, process, network, clock, Redb, Wasmtime, Steel execution, Nickel runtime, Iroh, or CLI shells. |
| Codec | `preserves`, `preserves-schema`, `serde`, `serde_json`, `toml`, `zstd`, `tar` | Encodes, parses, and hashes canonical artifacts; shared codec helpers must not import high-level domain modules. |
| Policy/evidence | `basalt`, `cairn`, `octet`, `trellis`, `nickel-lang` as authoring or evidence inputs | Constructs and verifies evidence-only values; does not grant authority or runtime trust by itself. Runtime consumes checked exports, refs, or receipts, not live policy tooling. |
| Runtime | `syndicate`, `tokio`, `n0-future`, `tracing` | Produces deterministic state transitions and planned effects after admission; shells execute effects. |
| Adapter | `iroh`, `iroh-blobs`, `iroh-docs`, `iroh-gossip`, `redb`, `wasmtime`, `wasmtime-wasi`, `wit-bindgen`, `wasmparser`, `steel-core` | Owns filesystem, transport, store, executor, and hostcall mechanics. Adapter availability is never trust. |
| CLI | `clap` | Parses flags and paths, reads/writes files, renders output, and exits. CLI code calls typed command cores. |
| Test/integration | `hegeltest`, `wat`, Nix VM checks, dogfood checks | Produces evidence for selected fixtures and integration profiles only. |

Reviewed boundary rules are authored in `docs/dependency-boundary-policy/valid.ncl`; negative fixtures cover duplicate rules, invalid layers, and empty allow/deny targets. The Rust pure validator in `molten_core::dependency` reports rule id, source file, forbidden target, and guidance for representative import facts.

## Policy authoring, export, runtime, and freshness

Policy source is authored in Nickel (`cairn-policy/default.ncl`, `cairn-policy/contracts.ncl`, and docs contracts). Checked generated JSON is a source-controlled export consumed by Cairn and runtime evidence checks. Runtime admission may consume checked exports, canonical refs, or policy-gate receipts; it must not invoke Nickel evaluation, Cairn policy export, or policy tooling availability as live authority.

The Cairn policy export now includes the current schema-owned fields `traceability_policy`, `stack_provenance_gate`, and `runtime_evidence_policy`. `molten_core::policy::validate_policy_freshness` is the pure freshness core for detecting missing schema fields or stale checked-export refs; the shell remains responsible for reading files and computing refs.

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

`molten_core::planning::plan_evidence_policy_runtime_flow` and `plan_adapter_effects` provide the in-memory core for this slice. Negative tests prove evidence-only data and adapter availability do not become trust.

## CLI imperative shell boundary

The CLI shell owns Clap parsing, path reads, stdout/stderr, process exit, and output files. The selected typed command-core boundary is the adapter effect planner exposed from `molten::prelude`; CLI handlers can pass parsed facts and receive `EffectPlan` without Clap, filesystem, stdout, stderr, network, or live adapter execution. Root tests prove the prelude exposes the positive and negative planner contracts.

Existing command names, flags, and documented output contracts are unchanged in this slice.

## Semantic module layout

The highest-pressure ordinal `include!` shard selected for the first semantic split is `src/chunk/store.rs` because it owns model, codec, verification, Redb index, pinning, GC, Iroh exchange, lineage, and receipt checks. The entry point now wraps all ordinal shards in a named `semantic_store` module and re-exports the public surface, preserving existing `molten::chunk_store::*` paths while giving reviewers a semantic boundary to expand into model/codec/verify/fs_store/index/exchange/retention/lineage/shell modules later.

Remaining ordinal shards are staged compatibility artifacts. They stay acceptable while generated or review-sized extractions still need path compatibility, and each future split must preserve public paths or provide an explicit compatibility package.

## Chunk store semantic boundary

Ownership split for the chunk store:

| Area | Owner | First boundary |
| --- | --- | --- |
| model | `ChunkRef`, `ChunkManifest`, receipt DTOs | In-memory DTOs and refs. |
| codec | Preserves constructors/parsers | Canonical bytes and BLAKE3 refs must stay byte-preserving. |
| verify | manifest/chunk/receipt checks | Pure validation over parsed values where possible. |
| fs_store | file read/write/remove/list | Shell-only. |
| index | Redb table definitions and transactions | Redb adapter shell. |
| exchange | Iroh publish/fetch | Transport adapter shell. |
| retention | pin/unpin/GC admission | Destructive plans before deletes. |
| lineage | manifest lineage and receipt chains | Evidence-only summaries. |
| shell | CLI and filesystem orchestration | Effect execution and receipts. |

`semantic_store` preserves the existing parser and identity tests. `molten_core::codec::validate_domain_artifact` is the first domain-owned codec façade used by chunk manifest parsing after canonical ref computation; it validates supported labels, schema identity, and BLAKE3 ref shape before callers consume the parsed manifest. `molten_core::planning::plan_retention_gc` and `plan_store_write` provide the pure destructive/store planning cores used by this slice.

## Preserves boundary adoption profile

The Preserves adoption profile lives under `docs/preserves-boundary-profile/`. It records artifact family, schema label, canonical-byte requirement, BLAKE3 identity field, adapter owner, core DTO boundary, allowed consumers, and non-claims for node control envelopes, tickets, workflow bundles, receipts, and evidence envelopes. `molten_core::preserves_profile::validate_preserves_boundary_profile` is the pure in-memory validator; shells remain responsible for measuring artifact bytes and refs.

Valid fixtures cover canonical node control, ticket, workflow bundle, receipt, and evidence envelope measurements. Negative fixtures fail closed for non-canonical bytes, missing schema labels, stale BLAKE3 refs, and raw-Preserves core coupling. Profile success proves canonical boundary identity and adapter placement only; it does not prove transport liveness, actor authority correctness, replay completeness, or Valence Evidence IR acceptance.

## Domain codec façade

The selected high-fan-in Preserves domain is the chunk store. It directly constructs and parses chunk manifests, chunk refs, lineage, receipts, and adapter tickets. The first façade is `molten_core::codec::validate_domain_artifact`, called from chunk manifest parsing to keep domain identity checks in a small in-memory core while preserving canonical Preserves bytes and refs. Positive core tests accept the current chunk manifest schema/ref contract; negative tests reject unsupported labels, schema drift, malformed refs, and missing domain identity.

Shared broad codec helpers remain in `preserves_rail`; they must not import high-level runtime, node, retention, job, plugin, CLI, or adapter domains.

## Redb store port boundary

Direct Redb usage remains in the chunk-store adapter shell. The selected first domain for port extraction is chunk-store write/delete planning. `plan_store_write` returns a store write and receipt plan only after authority, evidence freshness, resource admission, adapter support, and value well-formedness pass. `plan_retention_gc` returns no delete plan when authority, remote clearance, index completeness, or plan freshness fails.

## Retention core/store boundary

Retention ownership:

- admission: authority, policy, resource, and remote clearance inputs;
- plan: pure destructive `EffectPlan` values;
- apply: filesystem/store deletes in shell;
- audit: receipt-only evidence;
- store/bundle/live: adapter-specific mechanics;
- receipts: evidence-only canonical values.

The first pure destructive boundary is `plan_retention_gc`. Negative tests cover missing authority, missing remote clearance, stale plans, and incomplete indexes before any delete effect is returned.

## Job DAG execution boundary

Job DAG ownership:

- planning/admission/scheduling: pure DAG and lease checks;
- worker/blob IO/coordination/receipts/CLI: shells and adapters.

`plan_job_execution` returns worker execution only when admission passes, the DAG is acyclic, the manifest is present, the lease is fresh, and the executor is supported. Negative tests cover cycles, stale leases, missing manifests, and unsupported executors.

## Node daemon semantic boundary

The first node daemon decision boundary is duplicate enqueue planning. `plan_node_enqueue` turns duplicate operations into receipt-only replay plans without queue mutation while fresh admitted operations can produce store-write plans. State-root IO, live Iroh transport, and service-loop orchestration remain in node shell modules.

## Harness schema/gate modularity

Harness ownership:

- schema DTOs and report checks: pure in-memory validation;
- gate decision: `plan_harness_gate`;
- fixture IO and command orchestration: shell.

Positive tests accept supported, well-formed reports. Negative tests deny missing suites, malformed reports, unsupported schemas, and stale reports.

## Evidence ledger and registry/catalog boundary

Evidence parsers and verifiers should run over in-memory values before local ledger persistence. Ledger storage owns content-addressed persistence. Registry/catalog modules own classification, search, MCP read-only views, and operator discovery.

`plan_registry_discovery` preserves registry/catalog discovery as read-only evidence. Registry-only presence yields a deny decision with read/receipt effects and does not grant authority, provenance, policy, retention, source-gate, execution, or replay trust.

## Operator dogfood and integration boundary

Dogfood, prod-soak, and NixOS VM modules are integration shells. Runtime/node cores must not import operator dogfood, production soak, or VM modules. Their receipts remain evidence-only and do not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation, deployment, or release trust.

Focused evidence aggregation tests live in existing dogfood/prod modules; the new dependency-boundary policy names the integration boundary for future automated scans.

## Public API surface

Preferred stable API for new boundary work:

- `molten::core_api::*` for explicit core modules;
- `molten::prelude::*` for ergonomic planning, dependency, policy, and stack evidence cores;
- existing compatibility aliases such as `molten::chunk_store`, `molten::node_runtime`, and `molten::job_dag` remain preserved.

Current public root modules are classified as:

- stable API: `prelude`, `core_api`, `MoltenError`, `Result`;
- compatibility aliases: domain aliases created with `compat_module!`;
- internal implementation compatibility: `#[doc(hidden)]` root modules that back aliases;
- generated/test support: ordinal `parts/**` shards and `test_support`.

No implementation-only public module is removed in this slice because downstream compatibility aliases intentionally re-export the historical surface. The migration blocker is compatibility evidence: removing a hidden module requires a separate change that proves all documented aliases and downstream imports continue to compile.

## Validation paths

Focused checks for this boundary slice:

```sh
cargo test -p molten-core
cargo test --lib
cargo fmt --check
nickel export cairn-policy/default.ncl > /tmp/cairn-policy.json
nickel export docs/dependency-boundary-policy/valid.ncl
nickel export docs/stack-evidence-envelope/valid.ncl
nix run path:${ONIX_RESEARCH_ROOT:-$HOME/git/OnixResearch}/cairn#cairn -- validate --root .
```

Negative Nickel fixtures under `docs/dependency-boundary-policy/` and `docs/stack-evidence-envelope/` must fail to export. These checks are evidence for positive and negative contract coverage, not production deployment trust.
