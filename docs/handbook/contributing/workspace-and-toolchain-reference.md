# Workspace and toolchain reference

Mode: Reference

Use this page to locate the owner of a development setting and understand what an observed artifact means. It is a source snapshot, not a claim that dependencies are currently fetchable or that a build succeeded. No commands were executed for this page. Return to the [Handbook](../README.md).

## Workspace targets

The [root manifest](../../../Cargo.toml) declares four members and Cargo resolver `3`. Workspace package defaults are edition `2024` and license `AGPL-3.0-or-later`. These are package metadata; license metadata alone does not establish distribution compliance.

| Package | Manifest-owned entry or role | Selection caution |
| --- | --- | --- |
| `molten` | Library `src/lib.rs`; default binary `src/main.rs` named `molten` | Library tests and application subcommands are different surfaces |
| `molten-core` | Library `crates/molten-core/src/lib.rs` | Root-package nextest filters do not include it by name |
| `molten-node-host` | Internal capability-rooted node host package | Has its own manifest and development dependencies |
| `molten-release-policy` | Binary `crates/molten-release-policy/src/main.rs`; `publish = false` | Observes release inputs; not the application binary |

The root also declares `molten-native-extension-fixture`, an additional binary, and a `doltlite_oracle` integration test requiring the `doltlite-oracle` feature. Do not infer that a default run selects the fixture binary or that default tests enable the oracle.

## Toolchain and development environment

| Setting or tool | Actual owner | Review use |
| --- | --- | --- |
| `nightly-2026-05-26` | `rust-toolchain.toml` | Main Rust toolchain identity |
| `rust-src`, `rustfmt`, `clippy` | Toolchain components | Available components, not evidence they ran |
| Nix Rust toolchain | `flake.nix`, using `fromRustupToolchainFile` | Keeps shell/build toolchain tied to the file |
| `git-fetch-with-cli = true` | `.cargo/config.toml` | Cargo Git fetching uses the CLI path |
| Nextest `0.9.136` required/recommended | `.config/nextest.toml` | Runner compatibility requirement |
| Steel, Nickel, Wasmtime | Default Nix development shell | Language/runtime tooling, not an installed application |
| `cargo-nextest`, `cargo-watch`, `rust-analyzer` | Default development shell | Development tools |
| unit2nix, `nix-prefetch-git`, `jq` | Default development shell | Build-plan and source-hash workflow tools |
| `cargo-octet` | Catalogued `octet-toolchain` flake input | An unrelated ambient executable is not equivalent evidence |

The shell also supplies `pkg-config`, Clang, and Mold. Its Flux profiler CLI addition is conditional on `x86_64-linux`. Installing a similarly named system package does not demonstrate identity with the flake's selected tool.

## Root feature switches

| Feature | Manifest effect or boundary |
| --- | --- |
| `doltlite-oracle` | Enables the optional oracle surface and required-feature test selection |
| `executable-extents` | Enables data encoding, executable-extent dependencies, and the core feature |
| `world-snapshot-vm-cohort` | Enables optional `vm-cohort-core` dependency |
| `profiler` | Enables optional Flux dependency |
| `profiler-alloc` | Adds allocation profiling through `profiler` |
| `profiler-perf` | Adds Flux perf support through `profiler` |
| `profiler-disabled` | Enables Flux's disabled-profiling configuration |

These are compile-time selections, not runtime admission grants. Optional adapters still have their own source, platform, and policy prerequisites. A feature name does not establish that its live composition is available in a particular environment.

## Nextest selection and artifacts

All named semantic profiles below restrict selection to `package(molten)`. Inspect their full filter expressions in [the configuration](../../../.config/nextest.toml) when changing names or moving tests.

| Profile | Main selection intent | Important distinction |
| --- | --- | --- |
| `ci` | Root package | Four test threads; 45-minute global timeout |
| `deterministic` | Root package excluding live/VM/dogfood/soak/exploratory names | Four threads; 20-minute global timeout |
| `fast-core` | Hardening/bounded/Preserves/profile/receipt names | Inherits deterministic policy, not all core-crate tests |
| `harness` | Harness/replay/repro/gate/receipt names | Name filter, not every test helper |
| `cli` | CLI/cliharness/command/receipt names | Excludes the non-deterministic name partitions |
| `distributed-simulation` | Distributed/simulation/fault/two-peer/remote names | Does not imply live transport coverage |
| `vm-platform` | VM/NixOS/platform names | Platform-scoped; not itself a VM launcher |
| `dogfood-soak` | Dogfood/soak/release names | Sixty-minute global timeout |
| `exploratory` | Root package | One retry; flaky result passes; diagnostic distinction matters |

Named profiles configure JUnit at `target/nextest/<profile>/junit.xml`. JUnit is a rendered report. The [development reference](#development) below describes the hermetic nextest check preserving `cargo-metadata.json`, `binaries-metadata.json`, `junit.xml`, and `ci-test-run-receipt.preserves`; a local Cargo test run does not promise those outputs.

## Worked ownership lookup

Suppose `cargo nextest run --profile fast-core` succeeds but a change touched `molten-node-host`. The profile's name is not evidence of package coverage: its filter is explicitly `package(molten)`. The correct review question is which selected tests exercise the changed boundary and whether the owning member's tests were separately covered. Record the actual selection rather than relabeling the pass as a workspace-wide result.

Dependency changes have another multi-owner boundary. The Nickel release profile, Cargo manifests and lock, Nix input and lock, metadata-free source hashes, and both generated build plans must agree. The [governing update sequence](../../reproducible-dependencies.md) owns that process; this reference intentionally does not prescribe ad hoc lockfile edits or override-based release evidence.

## Sources

- [Handbook](../README.md)
- [Reproducible dependencies](../../reproducible-dependencies.md)
- [Dependency cohort theory](../../technical/engineering/dependency-cohorts-and-reproducible-builds.md)
- [Root manifest](../../../Cargo.toml)
- [Core manifest](../../../crates/molten-core/Cargo.toml)
- [Node host manifest](../../../crates/molten-node-host/Cargo.toml)
- [Release-policy manifest](../../../crates/molten-release-policy/Cargo.toml)
- [Toolchain](../../../rust-toolchain.toml), [Cargo configuration](../../../.cargo/config.toml), [flake](../../../flake.nix), and [nextest configuration](../../../.config/nextest.toml)


## Repository-root command convention

The command sequences retained below were moved from the root README. Run them
from the **repository root**; their relative paths are not relative to this
handbook page. They are reference instructions, not a record of execution for
the current source. Use fresh task-owned state/output locations when running
fixtures. Private or local sibling overrides are development choices, never
clean public bootstrap evidence. Clean reproduction is still gated on
[#23](https://github.com/brittonr/aspen/issues/23) and retained execution by the
integration owner. Historical pilot outputs and authored check definitions do
not qualify a new candidate.

## Dependency inventory

The dependency inventory includes Steel Scheme (`steel-core`), iroh (`iroh`,
`iroh-blobs`, `iroh-docs`, `iroh-gossip`), Syndicate (`syndicate`), Preserves
(`preserves`, `preserves-schema`), Nickel (`nickel-lang`), Wasmtime and
WASI/component tooling (`wasmtime`, `wasmtime-wasi`, `wit-bindgen`,
`wasmparser`), Redb (`redb`), Blake3 (`blake3`), Snafu (`snafu`), Serde
(`serde`), Tracing (`tracing`), Clap (`clap`), and revision-pinned OnixResearch
Git dependencies for Basalt (`basalt`), Cairn (`cairn-core` as `cairn`),
standalone Valence (`valence-core` as `valence`), and Trellis
(`verified-logic` as `trellis`). Dev dependencies include Hegel (`hegeltest`,
imported as `hegel`) for property-based testing. The Nix dev shell also exposes
the `steel`, `nickel`, and `wasmtime` tools.

## Architecture direction

For advanced implementation notes, see the [technical companion](../../technical/README.md): 100 source-linked articles covering runtime semantics, authority, fabric mechanisms, world lifecycle, evidence, and operations. Its topic index and reading routes complement the governing architecture and subsystem documents below.

For task-oriented material, see the [workflow handbook](../README.md): 100 walkthroughs, how-to guides, references, troubleshooting articles, and review checklists. The [documentation map](../../README.md) connects both collections to the governing subsystem documents.

See the [system atlas](../../atlas.md) for a chaptered map of the runtime, storage, and networking boundaries. See [`docs/architecture.md`](../../architecture.md) for the fuller architecture. See [`docs/nominal-authority-references.md`](../../nominal-authority-references.md) for typed authority and artifact reference domains, Preserves wire admission, compile-time separation, and authority non-claims. See [`docs/fabric-port-ownership.md`](../../fabric-port-ownership.md) for application-owned fabric ports, typed infrastructure failures, shell effect order, adapter roles, and source enforcement. See [`docs/nickel-toolchain.md`](../../nickel-toolchain.md) for the pinned Nickel evaluator cohort and product-policy boundary. See [`docs/dataspace-access-cache.md`](../../dataspace-access-cache.md) for bounded advisory access caching, deterministic retention, deferred release, and authority non-claims. See [`docs/live-artifact-binding-and-semantic-effects.md`](../../live-artifact-binding-and-semantic-effects.md) for one-snapshot live binding, conservative generation retirement, and exact Kamacite semantic operation adoption. See [`docs/fabric-whole-system-simulation.md`](../../fabric-whole-system-simulation.md) for bounded same-core simulation, replay, shrinking, reference services, and claim profiles. See [`docs/dag-sync.md`](../../dag-sync.md) for bounded receiver-driven graph synchronization, resume fencing, and live/simulation conformance. See [`docs/distributed-system-fabric.md`](../../distributed-system-fabric.md) for the primitive, adapter, system-extension, and workload ownership boundary. See [`docs/system-extension-runtime.md`](../../system-extension-runtime.md) for executable system-extension manifests, lifecycle, generation fencing, supervision, typed fabric effects, and operator evidence. See [`docs/fabric-time-scheduler-runtime.md`](../../fabric-time-scheduler-runtime.md) for typed time domains, generation-fenced timers, bounded runnable scheduling, purpose-bound entropy, deadlines, local lease decisions, and live/virtual adapter conformance. See [`docs/fabric-transport-session-runtime.md`](../../fabric-transport-session-runtime.md) for extension-owned protocol registration, capability-bound cross-process endpoint handoff, generation-fenced listeners/sessions/streams, typed effect-port routing, receipt-first two-process conformance, bounded framing and flow control, identity separation, live Iroh/simulation parity, and explicit connectivity/delivery non-claims. See [`docs/fabric-membership-placement.md`](../../fabric-membership-placement.md) for source-scoped membership views, bounded failure observations, deterministic placement, role recruitment, profile-scoped fencing, drain/replacement, and live/simulation provider parity. See [`docs/fabric-cryptographic-identity.md`](../../fabric-cryptographic-identity.md) for opaque purpose-scoped keys, Ed25519/Iroh production adapters, canonical signatures, generation-fenced rotation, redacted readback, and fixture-only BLAKE3 boundaries. See [`docs/fabric-observability.md`](../../fabric-observability.md) for bounded canonical metrics and events, scoped health/readiness, exporter failure semantics, redaction, and read-only integrity scans. See [`docs/content-store-adapter.md`](../../content-store-adapter.md) for primitive-owned content identity, bounded local/Redb/live-Iroh/simulation adapters, verified partial state, and protection non-authority. See [`docs/content-replication.md`](../../content-replication.md) for explicit replica policy, fenced repair, retention, and same-core conformance. See [`docs/syndicate-reference-harness.md`](../../syndicate-reference-harness.md) for the Syndicate reference-semantics boundary. See [`docs/local-filesystem-authority.md`](../../local-filesystem-authority.md) for the local capability-root filesystem boundary. See [`docs/choreography-typed-facade.md`](../../choreography-typed-facade.md) for the ChoRus-inspired typed-facade boundary. See [`docs/effect-manifest-profiles.md`](../../effect-manifest-profiles.md) for the Unison-inspired effect manifest/profile boundary. See [`docs/unison-reference-execution.md`](../../unison-reference-execution.md) for the artifact-ref remote execution boundary. See [`docs/plugin-lifecycle-fsm.md`](../../plugin-lifecycle-fsm.md) for the explicit plugin lifecycle FSM boundary. See [`docs/iroh-alpn-routing-registry.md`](../../iroh-alpn-routing-registry.md) for the canonical Iroh ALPN routing registry boundary. See [`docs/rkyv-derived-cache-boundary.md`](../../rkyv-derived-cache-boundary.md) for the rkyv-derived cache and typed-storage sidecar boundary. See [`docs/peer-session-transition-relation.md`](../../peer-session-transition-relation.md) for the reviewed peer-session FSM boundary. See [`docs/replay-coverage-readiness.md`](../../replay-coverage-readiness.md) for replay coverage/readiness matrices. See [`docs/replay-effect-log-hardening.md`](../../replay-effect-log-hardening.md) for replay effect-log ordering and binding validation. See [`docs/replay-identity-freshness.md`](../../replay-identity-freshness.md) for release-bound replay identity freshness checks. See [`docs/replay-multiturn-explain.md`](../../replay-multiturn-explain.md) for multi-turn replay compare/explain diagnostics. See [`docs/runtime-limit-profiles.md`](../../runtime-limit-profiles.md) for reviewed runtime budget admission under hard caps. See [`docs/ast-grep-runtime-authority-audits.md`](../../ast-grep-runtime-authority-audits.md) for ast-grep structural authority audits and blocking regression gates. See [`docs/verified-node-replication-pilot.md`](../../verified-node-replication-pilot.md) for the blocked local NUMA node-replication compatibility pilot, revision-pinned Octet verifier-package provenance, and trusted-boundary audit.

The product-owned world-commit protocol is documented in [`docs/world-commit.md`](../../world-commit.md). It defines typed roots, canonical identity, fenced local capture, closure checks, restore planning, detached evidence, and explicit non-claims.

The detached branch-head protocol is documented in [`docs/world-branch-heads.md`](../../world-branch-heads.md). It defines generation-fenced claims, Choregraph mapping, Artifact Auth statements, atomic local updates, explicit conflicts, and rollback limits.

World branch authority is documented in [`docs/world-branch-authority.md`](../../world-branch-authority.md). It maps portable Basalt policy into explicit derivation, simulation, linear-transfer, replacement, promotion, and activation obligations without copying live authority.

Typed comparison and merge are documented in [`docs/world-state-diff-and-merge.md`](../../world-state-diff-and-merge.md). The profile uses an exact base, closed merge modes, schema gates, pure handlers, durable conflicts, and publish-last output.

World replication and retention are documented in [`docs/world-distribution.md`](../../world-distribution.md). The adapter composes bounded DAG sync, protected content replication, detached claim exchange, complete retention roots, and observation-only reachability without granting activation or deletion authority.

World promotion and effect release are documented in [`docs/world-promotion-and-effect-release.md`](../../world-promotion-and-effect-release.md). The flow makes active-head movement and the complete local reservation set atomic while keeping dispatch, observations, and external completion separate.

Logical and opaque restore boundaries are documented in [`docs/world-execution-snapshots.md`](../../world-execution-snapshots.md). The profile uses closed inventories, exact cohorts, handle recreation, current-admission rechecks, opaque merge denial, and optional fail-closed VM Cohort clone realization.

Bounded sharing, mutation, replication, snapshot, and retention metrics are documented in [`docs/world-benchmark-sharing-and-retention.md`](../../world-benchmark-sharing-and-retention.md). The rail records exact structural facts and finite-run limits without granting deletion, extraction, dependency, or release authority.

Exact transition replay and complete bounded capsules are documented in [`docs/world-replay-capsules.md`](../../world-replay-capsules.md). The profile stops at the first complete-world divergence and keeps import, activation, authority, effects, and release claims separate.

Preview-first operator composition is documented in [`docs/world-operator-workflows.md`](../../world-operator-workflows.md). The typed workflow orders component handlers, rechecks every mutation, reports the first blocker, and links receipts without replacing their owners.

Bounded crash, restart, and concurrency conformance is documented in [`docs/world-fault-conformance.md`](../../world-fault-conformance.md). The rail keeps a closed mutation inventory, uses semantic fault phases, reopens durable state, and retains unsupported witness rows without granting recovery or cleanup authority.

The optional DoltLite semantic-state oracle is documented in [`docs/doltlite-semantic-state-oracle.md`](../../doltlite-semantic-state-oracle.md). It compares normalized, bounded test observations from one exact remotes-disabled cohort without granting product authority or proving correctness.

The product-local Prolly semantic-state map is documented in [`docs/prolly-semantic-state-map.md`](../../prolly-semantic-state-map.md). It provides bounded, history-independent ordered state while storage, branch policy, merge authority, retention, and deletion remain outside the core.

The coordination delivery extension is documented in [`docs/coordination-delivery.md`](../../coordination-delivery.md). It adds fenced claims, acknowledgements, logical-time retry, dead-letter handling, redrive, restart, and bounded receipts without changing base FIFO semantics or claiming exactly-once effects.

The addressable actor profile is documented in [`docs/addressable-actor-runtime.md`](../../addressable-actor-runtime.md). It composes keyed sleep, wake, restore, drain, and recovery over existing fabric owners with explicit survival and uncertainty boundaries.

The optional `executable-extents` profile is documented in [`docs/executable-extent-consumer.md`](../../executable-extent-consumer.md). It admits one exact Mantle bundle, remeasures each member, checks current authority, maps without W+X, and emits a detached consumer receipt.

The node daemon's capability-rooted lifecycle, identity, queue, ingress, nested-store, and async authority boundary is documented in [`docs/node-state-filesystem-authority.md`](../../node-state-filesystem-authority.md). The internal `molten-node-host` crate owns the shared error type, capability-rooted node state, and typed local stores. The root crate keeps `molten::error`, `molten::node_state`, and `molten::local_store` as compatibility re-exports. CLI parsing, daemon orchestration, service semantics, workload semantics, NixOS validation, and release policy remain outside the node-host crate. The explicit Wasm Component Model cohort, Mantle materialization boundary, deny-by-default imports, deterministic resource profile, migration split, and staged evidence model are documented in [`docs/wasm-component-runtime.md`](../../wasm-component-runtime.md). The Sightglass-backed, host-scoped component benchmark suites, exact AOT/Wizer admission, optimization profiles, and recorded-only performance receipts are documented in [`docs/wasm-component-performance.md`](../../wasm-component-performance.md).

Molten is a workload-neutral, policy-gated distributed-systems fabric built around a canonical Preserves envelope spine:

- Pure primitives own deterministic laws and plans; capability-rooted adapters perform external effects without defining authority; optional manifest-installed system extensions own distributed-service semantics.
- Executable system extensions use a distinct system-tier manifest, exact fabric-port bindings, active-generation callback fencing, finite resources, bounded supervision, and canonical status/evidence; ordinary plugin metadata or artifact possession cannot activate that tier.
- The native system-extension pilot runs one process per callback through Bounded Exec, persists callback and effect intent, recovers canonical instance state, and exposes exact acknowledged ingress. See [`docs/native-system-extension-host.md`](../../native-system-extension-host.md).
- Fabric time keeps wall, monotonic, logical, and virtual values non-interchangeable; timers and runnable scheduling are bounded and generation-fenced, while entropy is purpose/capability-bound and secret-free in evidence.
- Fabric membership keeps connectivity, suspicion, source-scoped membership, advisory placement, committed assignment, capability authority, and consistency ordering separate; role effects require generation/epoch/token fencing.
- Fabric cryptographic identity uses opaque purpose-scoped handles, canonical signature domains, admitted Ed25519 entropy and storage, generation-fenced rotation, and public-only status; identity and verification do not grant membership or authority.
- Fabric observability keeps metrics, traces, scoped health, readiness, and read-only integrity findings bounded and non-authoritative; exporter or scan failures are explicit terminal observations.
- Content-store adapters preserve canonical Molten manifests across capability-local, Redb, actual Iroh Blobs, and deterministic simulation profiles; transport and protection hints never replace identity or authority.
- Fabric ports resolve exact versions and reviewed profiles without silent substitution. Rust DTO layout, backend handles, transport identity, clocks, randomness, and ambient state are not canonical authority.
- OpenRaft is not selected, adapted, or used; consistency remains an explicit extension and port boundary. The bounded, model-only fast-path hazard contract is documented in [`docs/consensus-fastpath-hazard-model.md`](../../consensus-fastpath-hazard-model.md).
- Deterministic playback is a central law: the same artifacts, dependency closure, initial state, policy/schema refs, handler profile, and seed or recorded effect log must reproduce the same canonical traces, receipts, outputs, and final state hash.
- Preserves + Blake3 define stable communication, storage, policy, and evidence boundaries.
- Synit/SAM-inspired dataspaces provide assertions, retractions, `Observe` patterns, service dependency assertions, and turn tracing.
- Spritely Goblins-inspired vats are optional actor internals for near/far object references, transactional actormaps, promises/vows, revocable proxies, safe object serialization, and authority-graph debugging. The first local fixtures are available with `molten test vat run-fixture --out target/vat.preserves`, `molten test vat snapshot-fixture --out target/vat-snapshot.preserves`, `molten test vat restore-fixture --out target/vat-restore.preserves`, `molten test vat promise-fixture --out target/vat-promise.preserves`, `molten test vat ambient-authority-fixture --out target/vat-authority.preserves`, `molten test vat rights-fixture --out target/vat-rights.preserves`, `molten test vat distributed-ref-fixture --out target/vat-distributed-ref.preserves`, `molten test vat replay-fixture --out target/vat-replay.preserves`, and `molten test vat show <artifact>`.
- Trellis choreographies define finite multi-party protocol shape and project to dataspace-backed local endpoints.
- Trellis Raft primitives define strongly consistent replicated control-plane state, not normal actor traffic.
- Basalt/UCAN, Nickel contracts, reviewed Steel predicates, Trellis predicates, Cairn receipts, and Octet/Valence evidence gate side effects. Molten-to-Valence stack evidence identity compatibility is documented in [`docs/valence-stack-evidence-adapter.md`](../../valence-stack-evidence-adapter.md).
- Iroh bridges envelopes, blobs, and docs across peers; Wasmtime actors run behind deny-by-default hostcalls; Redb stores local durable metadata and indexes.

The executable system-extension fixture invokes initialization, start, request, checkpoint, retryable failure, restart/recovery, drain, and shutdown callbacks under native and no-WASI Wasmtime profiles, then writes canonical evidence and bounded operator status:

```sh
cargo run -- system-extension run-fixture \
  --profile sandboxed-component \
  --out target/system-extension-fixture
cargo run -- system-extension show \
  --status target/system-extension-fixture/status.preserves
```

The fixture proves callback execution and host-contract conformance only. It does not prove consensus, durability, protocol compatibility, extension semantic correctness, or production readiness.


## Archived Cairn roadmap

Cairn roadmap status: active production-readiness changes live under `.cairn/changes/` when present; use `cairn change list --root .` for the current active set. Accepted requirements live under `.cairn/specs/`, and completed roadmap slices are archived under `.cairn/archive/` (older slices under `cairn/archive/`). The drained/archived roadmap includes:

- `runtime-spine`
- `synit-sam-runtime`
- `goblins-vat-runtime`
- `trellis-choreography`
- `trellis-raft-consensus`
- `trellis-runtime-predicates`
- `unison-artifact-registry`
- `unison-effect-handlers`
- `unison-remote-artifact-sync`
- `unison-typed-storage`
- `unison-upgrade-sessions`
- `unison-schema-identity`
- `unison-evaluation-cache`
- `unison-executable-transcripts`
- `unison-structured-rewrite`
- `unison-artifact-catalog-mcp`
- `unison-distributed-job-dag`
- `deterministic-test-playback`
- `first-class-testing-harness`
- `admission-evidence-validation`
- `policy-boundary-preflight`
- `nickel-basalt-policy-preflight`
- `capability-context-admission`
- `basalt-ucan-capability-preflight`
- `mandatory-capability-fixtures`
- `mandatory-actor-registry-fixtures`
- `mandatory-budget-fixtures`
- `nickel-basalt-budget-preflight`
- `sealed-repro-bundles`
- `sealed-repro-verify-unpack`
- `sealed-repro-redaction-preflight`
- `redacted-repro-export-profiles`
- `signed-evidence-receipts`
- `chain-hashed-evidence-ledger`
- `local-evidence-ledger-store`
- `executor-hostcall-boundary`
- `iroh-sealed-repro-exchange`
- `haskell-runtime-patterns`
- `octet-enforcement-gates`
- `authority-identity-revocation`
- `resource-governance-backpressure`
- `delivery-idempotency-replay`
- `failure-supervision-lifecycle`
- `retention-gc-pinning`
- `secrets-redaction-confidentiality`
- `supply-chain-provenance-builds`
- `supply-chain-provenance-ux`
- `peer-bootstrap-negotiation`
- `content-addressed-chunk-store`
- `persistent-node-identity`
- `operator-receipts-dogfood`
- `plugin-host-abi`
- `federated-pull-sync`
- `blob-ref-job-submission`
- `coordination-primitives`
- `iroh-sam-dataspace`
- `remote-dataspace-harness-cli`
- `molten-node-runtime-daemon`
- `node-control-socket-runtime`
- `node-control-operation-dispatch`
- `node-control-daemon-loop`
- `node-control-provenance-gates`
- `node-control-iroh-ingress`
- `node-control-supervised-runner`
- `node-control-live-iroh-transport`
- `node-control-live-serve-listener`
- `node-control-authority-delegation`
- `node-control-live-peer-tickets`
- `node-control-supervisor-policy`
- `node-control-live-send-ux`
- `node-control-live-import-ux`
- `node-control-live-send-diagnostics`
- `node-control-live-workflow-bundle-import-export`
- `node-control-live-workflow-bundle-verify`
- `node-control-live-workflow-bundle-gate`
- `node-control-live-workflow-bundle-apply`
- `node-control-live-workflow-bundle-reconcile`
- `node-control-live-workflow-bundle-ack`
- `node-control-live-trellis-workflow`
- `sam-service-supervision-runtime`
- `sam-service-records-ledger`
- `sam-service-demand-runtime`
- `sam-service-supervision-cleanup`
- `trellis-protocol-session-runtime`
- `raft-control-plane-registry`
- `job-dag-iroh-worker-execution`
- `job-dag-iroh-worker-cli-ux`
- `job-worker-coordination-scheduling-ux`
- `dataspace-delivery-idempotency`
- `secrets-redaction-encrypted-refs`
- `plugin-host-lifecycle-runtime`
- `system-extension-service-runtime`
- `fabric-durable-state-ports`
- `fabric-transport-session-runtime`
- `fabric-cross-process-transport-shell`
- `coordination-services-control-plane`
- `coordination-control-plane-ux`
- `operator-dogfood-node-workflow`
- `operator-dogfood-retention-gc-workflow`
- `operator-dogfood-release-evidence-bundle`

## Development

Optional cross-process function profiling is documented in [`docs/development-function-profiling.md`](../../development-function-profiling.md). Traces are bounded development observations and never release evidence.

Proof-affecting changes should follow [`docs/proof-workflow.md`](../../proof-workflow.md) for checklists, receipt-backed traceability, deny-path evidence, layered proof boundaries, and readbacks.

```sh
nix develop
cargo nextest run
cargo test # fallback
```

Nextest profiles:

```sh
cargo nextest run --profile fast-core
cargo nextest run --profile harness
cargo nextest run --profile cli
cargo nextest run --profile distributed-simulation
cargo nextest run --profile vm-platform
cargo nextest run --profile dogfood-soak
cargo nextest run --profile ci
cargo nextest run --profile deterministic
cargo nextest run --profile exploratory
```

The profiles fail flaky tests in CI/deterministic mode, bound hangs with explicit global/slow/leak timeouts, and write JUnit evidence under `target/nextest/<profile>/junit.xml` for release readback. `.config/nextest.toml` now gives each semantic profile an explicit `default-filter`, and `nix build .#checks.x86_64-linux.nextest-config` emits `nextest-profile-matrix-v1` readback that binds filters, command surfaces, JUnit paths, retry policy, cost class, evidence scope, platform availability, and diagnostics. `fast-core`, `harness`, `cli`, and `distributed-simulation` are deterministic evidence profiles only when their filters exclude live-only, VM-only, exploratory, retry-only, and diagnostic-only partitions. `vm-platform` is platform-scoped VM evidence, `dogfood-soak` is operator-readiness evidence, and `exploratory` retry success remains diagnostic-only; none of these profiles replace canonical subsystem receipts, source gates, policy, provenance, authority, transport, resource, retention, deployment, or release gates.

Nix exposes the CI command and runs nextest through flake checks. The hermetic `nextest` check preserves `cargo-metadata.json`, `binaries-metadata.json`, `junit.xml`, and `ci-test-run-receipt.preserves`; JUnit is a rendered view and cannot replace the canonical CI receipt.

```sh
nix run .#nextest-ci
nix build .#checks.x86_64-linux.nextest
nix build .#checks.x86_64-linux.dogfood-local-node
nix build .#checks.x86_64-linux.nixos-vm-multinode
nix build .#checks.x86_64-linux.nextest-config
```

When running the VM check from a sibling checkout workspace, pass explicit local sibling overrides so unit2nix receives cleaned checkout trees instead of falling back to remote fetches:

```sh
nix build .#checks.x86_64-linux.nixos-vm-multinode -L \
  --override-input basalt-src path:../basalt \
  --override-input cairn-src path:../cairn \
  --override-input valence-src path:../valence \
  --override-input onixpkgs path:../onixpkgs
```

**Historical/intended positive-rail description, not current qualification:**
the paragraph below retains the old README's VM/soak artifact contract. A green
unsigned `nixos-vm-multinode` denial check must not be read as successful signed
startup. [#16](https://github.com/brittonr/aspen/issues/16) separates that
negative result from the positive `nix run .#nixos-vm-multinode-signed` rail,
which requires independently selected source trust and actual retained
execution. Consult the selected rail definition and
[exact capability evidence](../../capability-qualification-report.md), not
this historical description, before claiming any positive integration result.

`nixos-vm-multinode` is an explicit `testers.runNixOSTest` platform integration check. It starts two headless NixOS VMs with the current Molten package, runs the real node init/start/status/control-loop/stop service path under systemd with isolated `/var/lib/molten` roots, checks VM networking, exercises queued control-request recovery across a service restart, and emits `nixos-vm-topology-v1`, `nixos-vm-node-evidence-v1`, and `nixos-vm-test-run-v1` receipts inside the VM evidence directories. It also emits `prod-soak-evidence-export-v1`, `prod-soak-durability-v1`, `prod-soak-fault-case-v1`, `prod-soak-fault-matrix-v1`, `prod-soak-resource-envelope-v1`, and `prod-soak-run-v1` receipts that bind the production-shaped live workflow child evidence: peer tickets, node-control bundle lifecycle, remote dataspace/service exchange, blob-ref job execution, coordination apply, restart durability, network/transport fault coverage, resource envelope bounds, per-node evidence exports, replay status, logs, and pilot-scope caveats. For the historical internal pilot, retained matching soak evidence was scoped to show that the VM topology can coordinate live child receipts across peers, survive the bounded restart path, preserve referenced artifacts, and report pilot resource bounds. It remains out of scope for broad production claims about real WAN transport, sustained SLOs, adversarial security, authority delegation, retention policy, destructive operations, source-gate trust, or fleet-scale resource pressure. It requires working NixOS VM execution support; missing KVM/QEMU support must fail or report unavailable rather than minting pass evidence.

`dogfood-local-node` depends on the hermetic nextest check, runs `molten dogfood local-node` in a temporary state root, preserves `dogfood-report.preserves`, `release-gate.preserves`, `replay-verify.preserves`, `replay-evidence-index.preserves`, `dogfood-summary.txt`, and an `after-nextest.txt` marker, then emits `nix-dogfood-evidence.preserves` plus `nix-dogfood-verify.preserves` to bind the Nix output path, report ref, release-gate ref, replay verify ref, replay evidence index ref, and nextest dependency marker for release review. It also emits `release-evidence-bundle.preserves`, `release-evidence-bundle-verify.preserves`, `release-promotion-gate.preserves`, `release-promotion-gate.signed.preserves`, `release-promotion-summary.preserves`, `release-export-manifest.preserves`, `release-evidence.tar.zst`, and `release-export-verify.preserves`, which bind the complete release review member set, signed keyring currentness, source/Octet/Cairn evidence markers, a final evidence-only promotion decision, a keyring-verified signature over that exact decision subject ref, a compact readback summary, and a deterministic portable evidence archive plus verification receipt. The check imports a local fixture signing key into a ledger-backed signed receipt keyring, signs the Preserves release members, verifies the bundle with `--require-signed-members` plus `--signed-key-ledger`, runs `release-promote` over the resulting graph, signs the promotion receipt with purpose `release-promotion`, and verifies that signed promotion receipt through the same keyring. These artifacts remain evidence-only and do not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation, or deployment trust.

Release bundle commands can be run manually against a realized check output: `molten dogfood release-bundle-export --output-path OUT --out release-evidence-bundle.preserves`, `molten dogfood release-bundle-verify --output-path OUT --bundle release-evidence-bundle.preserves --receipt-out release-evidence-bundle-verify.preserves`, `molten dogfood release-promote --output-path OUT --bundle-verify release-evidence-bundle-verify.preserves --receipt-out release-promotion-gate.preserves --signed-key-ledger KEYRING --source-evidence ... --octet-evidence ... --cairn-evidence ...`, `molten dogfood release-promotion-summary --output-path OUT --out release-promotion-summary.preserves --signed-key-ledger KEYRING`, `molten dogfood release-export --output-path OUT --out release-evidence.tar.zst --manifest-out release-export-manifest.preserves`, and `molten dogfood release-export-verify --bundle release-evidence.tar.zst --receipt-out release-export-verify.preserves`; export verification emits deny receipts for missing manifests, duplicate paths, extra members, stale refs, or tampered member bytes. Signed member and promotion checks use `molten receipts key import --ledger KEYRING --key-id release-key-v1 ...`, `molten receipts sign RECEIPT --out RECEIPT.signed.preserves --purpose release-evidence|release-promotion ...`, and `molten receipts verify-signed --key-ledger KEYRING --key-id release-key-v1 ...`; `release-bundle-verify --require-signed-members --signed-key-ledger KEYRING --signed-member RECEIPT.signed.preserves ...` emits deny receipts for missing, wrong-signer, wrong-purpose, revoked-key, stale, or tampered members instead of treating logs as primary evidence. Keyring management also supports `molten receipts key list/show/revoke/rotate`; currentness and revocation are verification evidence, not authority.

Operator receipt readback is available for local dogfood ledgers: `molten receipts list --ledger STATE/ledger`, `molten receipts show REF --ledger STATE/ledger`, `molten receipts validate REF --ledger STATE/ledger`, and `molten receipts export REF --ledger STATE/ledger --out receipt.preserves`. The commands read canonical Preserves artifacts from the content-addressed ledger, validate supported dogfood/operator receipt kinds, render only non-normative summaries by default, and keep logs auxiliary rather than primary evidence.

For revision-pinned OnixResearch Git dependencies, the flake uses reviewed immutable HTTPS source inputs. Authentication and binding use one unified Artifact input. The default source graph requires no GitHub SSH credentials; private Radicle sources remain explicit public-seed or local-profile prerequisites. Sibling checkouts can be substituted only through explicit development-time `--override-input` paths. The substitution filters generated and VCS-heavy directories such as `target/` and `.git/` before unit2nix sees the source. Release validation compares Cargo manifest, lockfile, Nix, release-policy, and generated-plan identities and rejects local paths as release evidence. See [`docs/reproducible-dependencies.md`](../../reproducible-dependencies.md). Historical limited-pilot nextest output (not a current clean result): `/nix/store/d50qlba4am1pb4f59l71djjg1bl6adhi-molten-nextest`.

The historical Molten `0.1.0` limited-pilot candidate is commit `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0`. Its source reference is `blake3:80e3ceb18784504c7573191fce72e121d0789613c6c5f7bdcecbdd9ae0e4cdb7`.

The candidate gate passed as `blake3:9d15e7f99a34ba5f1a979d07cd7023025347144349db50caacca6ddbb90ab1e4`. The pilot decision is `blake3:ea03ddbada910b33a09fe3156e9908b2b7c1f1b0c6b14b39570ae05abdaf27cb`.

See [`docs/releases/molten-0.1.0.md`](../../releases/molten-0.1.0.md) and [`release/molten-0.1.0-pilot.ncl`](../../../release/molten-0.1.0-pilot.ncl) for scope and evidence. Preserved artifacts live with the archived release Cairn. They remain evidence-only and grant no runtime or release authority.

### Strict Octet source-gate sequence

```sh
cargo octet check --artifact-dir target/octet
cargo octet check -p molten --artifact-dir target/octet-lib -- --lib
cargo octet object corpus receipt \
  --output target/octet/object-corpus-receipt.json \
  $(find src -name "*.rs" | sort)
cargo run -- test octet artifacts import \
  --artifacts target/octet \
  --ledger target/octet-ledger \
  --receipt-out target/octet/artifact-ledger-receipt.preserves
cargo run -- test octet gate \
  --artifacts target/octet \
  --profile strict-ci \
  --receipt-out target/octet/gate-receipt.preserves
cargo run -- test octet remediation plan \
  --artifacts target/octet \
  --lib-artifacts target/octet-lib \
  --focused-object-corpus target/octet/object-corpus-receipt.json \
  --receipt-out target/octet/remediation-plan.preserves
cargo test
cargo clippy --all-targets -- -D warnings
CAIRN_ROOT="${CAIRN_ROOT:-../cairn}"
nix run "path:$CAIRN_ROOT#cairn" -- validate --root . \
  --policy "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" \
  --strict
```

The strict gate is fail-closed: `warning-only` denies even when `cargo-octet` exits `0`, and `command.txt`, `status.json`, `summary.txt`, structured finding keys, object corpus receipts, and fingerprint evidence are bound by canonical refs in the Octet receipt. The dev shell provides `cargo-octet` from the catalogued `octet-toolchain` flake input (`OnixResearch/octet` `3f7abb641ee604f54629da415bc7686f89662065`, toolchain `nightly-2026-03-21`); an ambient `cargo-octet` elsewhere on `PATH` is not Octet evidence. The [nominal adoption record](../../nominal-authority-references.md#result-and-adoption-status) separates the approved focused acceptance from retained whole-core warning debt. The frozen verifier pilot independently retains its original Octet cohort through `verified-node-replication-octet`; see its [exact inputs](../../verified-node-replication-pilot.md#exact-inputs). The quarantine commands below are historical burn-down tooling: their example expiry dates have passed, and no baseline or review receipt admits current findings.

The [2026-10-15 local source checkpoint](../../octet-burndown-handoff.md#current-source-checkpoint)
records clean configured workspace/library producers and a passing unchanged
`strict-ci` gate. It is unintegrated source evidence, not acceptance of every
active Cairn change.

```sh
cargo run -- test octet baseline write \
  --artifacts target/octet \
  --out target/octet/warning-baseline.preserves \
  --created-at 2026-05-31T00:00:00Z \
  --expires-at 2026-06-30T00:00:00Z
cargo run -- test octet review write \
  --out target/octet/critical-review.preserves \
  --profile quarantine-ci \
  --expires-at 2026-06-30T00:00:00Z \
  --finding-key b3:...
cargo run -- test octet baseline check \
  --artifacts target/octet \
  --baseline target/octet/warning-baseline.preserves \
  --profile quarantine-ci \
  --as-of 2026-05-31T00:00:00Z \
  --review target/octet/critical-review.preserves \
  --receipt-out target/octet/baseline-receipt.preserves
```

## Nix

```sh
nix build
nix flake check
nix build .#checks.x86_64-linux.verified-node-replication-pilot --no-link -L
```

## Public release checklist

A new external candidate is not ready until the following are true for the exact
source revision and dependency closure under review:

- [ ] A fresh checkout builds through the public HTTPS source graph without SSH
  credentials or sibling overrides, and the release dependency profile check
  passes against exact Nix source inputs.
- [ ] The selected NixOS or local deployment path has a positive executed
  workflow; negative startup-denial checks remain evidence of denial, not
  positive startup.
- [ ] VM fault receipts distinguish executed observations from unavailable or
  unexecuted fixtures; caller-authored logs cannot qualify an observed denial.
- [ ] Cross-host claims are backed by actual authenticated peer evidence for the
  selected profile; loopback evidence is labelled as loopback.
- [ ] Capability qualification distinguishes implementation status from recorded
  execution evidence for the exact profile and environment.
- [ ] Third-party notices match the selected dependency revisions, and the
  dependency and license audit records its advisory snapshot and dispositions.
- [ ] [SECURITY.md](../../../SECURITY.md) names a verified private reporting channel and
  [CONTRIBUTING.md](../../../CONTRIBUTING.md) matches the current no-PR integration route.
- [ ] Release notes, source acquisition or archive, checksums, supported
  commands, and retained evidence are reviewed before a new tag is published.

## References

- [Dolt Prolly Tree architecture](https://www.dolthub.com/docs/architecture/storage-engine/prolly-tree/) — design reference for history-independent ordered structure, content-addressed nodes, structural sharing, and changed-region diff. Molten keeps its own canonical format, BLAKE3 domains, bounds, proofs, authority, and evidence.
- [`dolthub/doltlite` at `10170ed82c1b12414db8d1b29d2fe9ea2a72fd88`](https://github.com/dolthub/doltlite/tree/10170ed82c1b12414db8d1b29d2fe9ea2a72fd88) — pinned optional semantic-state oracle for branch, concurrency, format, GC, and compatibility cases. DoltLite does not own Molten world identity, durable conflicts, authority, effects, or release claims.
- [Engineering Record And Replay For Deployability](https://www.usenix.org/system/files/conference/atc17/atc17-o_callahan.pdf) — replay-boundary reference for explicit nondeterministic inputs and sealed observations. Molten adapts the principle at typed hostcalls and effect ports rather than adopting native syscall replay as world semantics.
- [`rr-debugger/rr` `5.9.0` at `aaed29dc0324444e36503a26dcec96731d1942b0`](https://github.com/rr-debugger/rr/tree/aaed29dc0324444e36503a26dcec96731d1942b0) — optional native-process diagnostic mechanism owned through the ChaosControl adapter. It does not replace logical world replay or deterministic VM simulation.
- [PsychoLlama/ambient](https://github.com/PsychoLlama/ambient) — design reference for immutable content-addressed implementations, explicit one-snapshot late binding, runtime-owned state roots, semantic handler keys, and generation retirement tracing. Molten retains its own Preserves identity, receiver-side authority and policy admission, resource limits, evidence, retention, and distributed-runtime semantics; it does not adopt Ambient's language, VM, mobile-code model, or unauthenticated deployment posture.
- [`onixpkgs` kache Nix Rust helpers](https://github.com/OnixResearch/onixpkgs/blob/56a93169f4ace2394dc4c474c6d28911c24c35e7/lib/kache-nix-rust.nix) — pinned `onixpkgs` flake input that supplies the kache package (`onixpkgs.overlays.default`) and `lib.kacheNixRust`, the helpers behind the opt-in Nix-owned kache wrappers around sandboxed Rust builds.
- [`verus-lang/verified-node-replication`](https://github.com/verus-lang/verified-node-replication) — pinned local multicore/NUMA compatibility pilot only; it is not network replication or consensus, current verifier compatibility is blocked, and upstream trusted boundaries do not transfer proof authority.
- [`verus-lang/verus`](https://github.com/verus-lang/verus) — exact current and historical verifier source identities for the pilot; verifier execution is bounded evidence, not a tool-soundness or Molten-correctness claim.
- [`n0-computer/iroh-experiments`](https://github.com/n0-computer/iroh-experiments) — design reference for hint-only Iroh content discovery, deterministic DAG traversal, optional pkarr pointers, HTTP3-over-Iroh readback, and remote byte-source ideas; Molten keeps Preserves receipts, BLAKE3 identity, and capability/policy/resource gates normative.
- [`brittonr/aspen` `main`](https://github.com/brittonr/aspen/tree/main) — AGPL-licensed behavior and architecture reference for clean-room requirements and independently expressed conformance tests only; Molten does not copy Aspen implementation code, comments, or fixtures without compatible relicensing.
- [`n0-computer/iroh-examples`](https://github.com/n0-computer/iroh-examples) — reference patterns for dynamic Iroh protocol routing, framed bidirectional streams, and read-only blob gateway UX.
- [`n0-computer/iroh-experiments`](https://github.com/n0-computer/iroh-experiments) — reference patterns for Iroh content discovery, deterministic DAG sync, pkarr latest pointers, HTTP3-over-Iroh readback, and remote byte-source verification experiments.
- [`n0-computer/irpc`](https://github.com/n0-computer/irpc) — reference patterns for local/remote request-response and streaming service sessions over Iroh-style transports.
- [`n0-computer/iroh-doctor`](https://github.com/n0-computer/iroh-doctor) — reference patterns for NAT, UDP, relay latency, connect/accept, port-map, and network diagnostic reports.
- [`n0-computer/net-tools`](https://github.com/n0-computer/net-tools) — reference patterns for route/interface watching and opt-in port mapping via UPnP, PCP, and NAT-PMP.
- [`n0-computer/iroh-metrics`](https://github.com/n0-computer/iroh-metrics) — reference patterns for bounded counters, gauges, histograms, and OpenMetrics export.
- [`n0-computer/iroh-services`](https://github.com/n0-computer/iroh-services) — reference patterns for optional external metrics/diagnostics service integration and remote diagnostics capabilities.
- [`n0-computer/n0-watcher`](https://github.com/n0-computer/n0-watcher) — reference patterns for latest-state watchers without unbounded intermediate-value buffering.
- [`rkyv`](https://github.com/rkyv/rkyv) — reference zero-copy Rust archive format for optional derived local cache materializations; Preserves remains Molten's canonical evidence and storage boundary.
- [`lsd-ucsc/ChoRus`](https://github.com/lsd-ucsc/ChoRus) — reference Rust choreography ergonomics for typed locations, located values, EPP-as-DI, and runner/projector APIs; Trellis, Preserves, and Molten receipts remain the normative semantics.
- [`bytecodealliance/cap-std`](https://github.com/bytecodealliance/cap-std) — reference for capability-rooted filesystem and temporary-workspace boundaries; Molten retains its own policy, evidence, durability, and authority semantics and does not treat `cap-std` as a sandbox for untrusted Rust code.
- [`bytecodealliance/sightglass` at `c18bbe75803a6a610f7ff3b15549c927c6e02667`](https://github.com/bytecodealliance/sightglass/tree/c18bbe75803a6a610f7ff3b15549c927c6e02667) — pinned CLI and raw-measurement-schema reference for recorded-only Wasm component performance experiments; its results do not transfer correctness, security, authority, or release claims.
- [`stonysystems/jetpack` at `c03e318ec355b11edd42aac56c68d0765f88d1d2`](https://github.com/stonysystems/jetpack/tree/c03e318ec355b11edd42aac56c68d0765f88d1d2) and the [OSDI 2026 paper](https://www.usenix.org/system/files/osdi26-tang.pdf) — pinned MIT-licensed design, TLA+, fault-scenario, and benchmark references for optional crash-fault consensus acceleration; external proofs, code behavior, and performance claims do not transfer to Molten.
- [Schema Migration Core](https://seed.radicle.garden/z1C4YVMgDGyVdQa72uPNj3UDS5cY.git) at `4fe90e130f2871cf69a6febcdc70785adca98aea` — bounded migration planning pilot without runtime or storage authority.
- [WalTier](https://github.com/danthegoodman1/waltier) — MIT reference for an object-store CAS-arbitrated ordered log. Molten keeps its store, consistency, authority, and evidence semantics.
- [`adanil-code/LRUHashTable`](https://github.com/adanil-code/LRUHashTable) — Apache-2.0 design reference for lazy promotion and deferred release. Molten keeps explicit bounds, BLAKE3 keys, dataspace authority, and policy semantics.
- [`executable-extent`](rad://z37R1bP1kHcELs89RNbQRaqbCVKxB) — private product-neutral source for layout admission, W^X transitions, Linux mapping, and bounded conformance. Molten retains code meaning, runtime authority, world roots, receipts, storage, and release decisions. See [`docs/executable-extent-consumer.md`](../../executable-extent-consumer.md).
- [Mantle executable-extent producer](rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb) at `2c636b1b25353a1b0befa5af48dc68615cd686dd` — private producer reference for the exact flat-page bundle and detached producer receipt. Mantle does not grant Molten mapping or execution authority.
- [`choregraph-history`](rad://zL2ncTUeASVYwcoGkEXv9JKgGbAF) at `b3e08e19750f53bdbcae970cdf58a47a791ed20b` — reviewed pure branch-plan dependency. Choregraph does not own Molten branch policy, head storage, authority, conflict selection, or release decisions.
- [`transactional-reconciliation-core`](rad://z4Tky6zvC8w4Y6c4YBzNxVbq5n752) at `eb2bd3441753af97bfcb247cef7cc22d72675b62` — reviewed pure publication and uncertain-outcome classification dependency. It does not own Molten mutation, dispatch, authority, observations, or release decisions.
- [`bounded-exec`](https://github.com/OnixResearch/bounded-exec.git) at `29dac88ecded94457572db3fdfaaaab95fa91525` — reviewed bounded process mechanism for explicit input, output, deadlines, cancellation, and owned teardown. Aspen retains executable authority, content publication, lifecycle, evidence, and release decisions. See [`docs/fabric-execution.md`](../../fabric-execution.md).
- [`chaoscontrol-snapshot-descriptor`](https://github.com/brittonr/chaoscontrol/tree/b8c440ea3b19df796542e58e8ee36200e1c3db85/crates/chaoscontrol-snapshot-descriptor) at `b8c440ea3b19df796542e58e8ee36200e1c3db85` — reviewed portable opaque-snapshot descriptor source. ChaosControl keeps fault, scheduler, replay, guest, evidence, and release meaning.
- [`vm-cohort`](rad://z2QJLUqyAZnnHPiZQ1BFjLsX9ush3) at `31f1696ba9391bfda8577a58af84f72361d5573e` — optional product-neutral clone-planning source. Molten keeps world meaning, admission, runtime effects, evidence, and release decisions.
- [`rivet-dev/actors`](https://github.com/rivet-dev/actors/tree/71f371ba4eab1234d8b6b6c419e6748cc6fc9911) at `71f371ba4eab1234d8b6b6c419e6748cc6fc9911` — Apache-2.0 design reference for keyed addressability, generation fencing, sleep and rewake separation, and explicit persisted classes. Molten keeps Preserves identity, fabric ownership, Basalt/UCAN authority, logical time, survival rules, effect uncertainty, and evidence semantics; Rivet APIs, formats, benchmarks, and service claims are not compatibility targets.
- [*The Synit Manual*](https://synit.org/book/) — CC BY 4.0 design reference for reactive dataspaces, assertion lifetimes, facets, capability attenuation, declared service state, and causal turn traces. Molten keeps its own Preserves envelopes, receipts, Basalt/UCAN authority, lifecycle states, and evidence boundaries; Synit wire protocol, relays, sturdyrefs, and configuration scripting are not compatibility targets. See [`docs/synit-incorporation-review.md`](../../synit-incorporation-review.md).
- [`komora-io/marble` `16.0.2`](https://github.com/komora-io/marble) and [`spacejam/art` `1.0.0`](https://github.com/spacejam/art) — optional, feature-gated physical store and digest index measured against the Redb Prolly store. The recorded decision keeps the current path; marble does not own content identity, durability beyond recovered batches, or any runtime semantics.
