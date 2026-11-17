# Aspen

<a id="molten"></a>

**A Rust runtime for distributed systems with explicit authority, bounded effects, and deterministic replay.**

[Getting started](#getting-started) · [Architecture](#architecture) · [Documentation](#documentation) · [Contributing](CONTRIBUTING.md)

Aspen provides building blocks for stateful distributed services: supervised execution, capability-checked operations, content-addressed state, and reproducible execution records. It separates decisions from filesystem, network, and process operations so you can inspect what a service was allowed to do, what it attempted, and what happened.

It is built for developers working on distributed runtimes, service coordination, and reproducible systems experiments. Pure transition cores drive both live adapters and bounded simulations; canonical records make decisions and outcomes available for inspection and replay.

> **Development preview.** Aspen has working local fixtures and scoped runtime implementations. External deployment qualification is still in progress, and there is no production-supported release. See [release status](docs/release-status.md) for the current scope.

## What Aspen provides

- **Explicit authority.** Capabilities, policy, and resource limits govern operations. Loading an artifact or connecting to a peer does not grant permission to execute its effects.
- **Controlled effects.** Typed adapters handle storage, transport, time, and process execution. The runtime records intent and outcomes at the defined boundaries, including operations whose completion is uncertain.
- **Deterministic replay.** The same artifacts, initial state, policy, and recorded nondeterministic inputs reproduce canonical transitions and outputs within the selected replay profile.
- **Supervised services.** System extensions declare lifecycle callbacks, exact runtime interfaces, and finite budgets for initialization, requests, checkpoints, restart, upgrade, drain, and shutdown.
- **Inspectable state.** Preserves values and BLAKE3 references identify artifacts, snapshots, and execution receipts. World commits organize typed state roots for comparison, restore planning, and replay.

The [architecture](docs/architecture.md) explains these boundaries; each subsystem's documentation records its implementation and qualification limits.

## Getting started

The first example runs a local system-extension lifecycle and writes inspectable evidence. It requires a development environment that can resolve the repository's pinned dependencies.

**Prerequisites:** Git, an x86_64 Linux development host, and Nix with flakes enabled. No credentials, SSH keys, or Radicle node are needed: sources are fetched anonymously over HTTPS, and two exact sources without a public route ship as reviewed snapshots under `vendor/`. The Nix shell supplies the Rust toolchain and development tools.

One pinned development tool, the Octet source-gate checker, still names a public GitHub dependency by its SSH URL. In the shell you build from, export this rewrite so Git fetches the same commits over anonymous HTTPS. It changes no global configuration and does not affect `git@github.com:` remotes:

```sh
export GIT_CONFIG_COUNT=1 \
  GIT_CONFIG_KEY_0=url.https://github.com/.insteadOf \
  GIT_CONFIG_VALUE_0=ssh://git@github.com/
git clone --branch molten https://github.com/brittonr/aspen.git
cd aspen
nix develop
```

The hosted [Reproducible dependencies](.github/workflows/reproducible-dependencies.yml) workflow exercises this path on a clean runner; [public-build work](https://github.com/brittonr/aspen/issues/23) and the [dependency policy](docs/reproducible-dependencies.md) record its evidence. If a fetch still fails, see [environment troubleshooting](docs/handbook/contributing/diagnosing-development-environment-failures.md#cargo-cannot-fetch-a-pinned-git-dependency) and preserve the error rather than changing pinned inputs.

From the repository root inside that shell:

```sh
mkdir -p target
ASPEN_DEMO_DIR="$(mktemp -d target/aspen-demo.XXXXXX)"

cargo run --locked -- system-extension run-fixture \
  --profile sandboxed-component \
  --out "$ASPEN_DEMO_DIR"

cargo run --locked -- system-extension show \
  --status "$ASPEN_DEMO_DIR/status.preserves"
```

The first command reports `system-extension fixture ok profile=sandboxed-component`. The status readback includes `phase=stopped` and `health=stopped`: the fixture deliberately completes its shutdown lifecycle.

Inspect `manifest.preserves`, `status.preserves`, the upgrade/rollback/recovery status files, and the indexed receipts under `evidence/` in the generated directory. After inspection, remove only that run's output:

```sh
rm -rf -- "${ASPEN_DEMO_DIR:?Set by the demo above}"
```

**Scope of this example:** lifecycle callbacks, bounded restart, checkpoint, upgrade/rollback, and canonical evidence. The `sandboxed-component` fixture invokes an import-free Wasmtime core-module probe; it is not the installed Component Model deployment path. It does not qualify production isolation or durability. The [fixture implementation](src/cli/runtime/system_extension/ops.rs) owns its commands and canonical outputs.

**Not covered yet, with owning issues:**

- Live cross-host transport: [qualify a cross-host Iroh streaming profile #15](https://github.com/brittonr/aspen/issues/15).
- A selected runtime composition beyond this local fixture: [executable world promotion #17](https://github.com/brittonr/aspen/issues/17) and [a hosted addressable actor #19](https://github.com/brittonr/aspen/issues/19).
- A two-host variant of this workflow is added only after #15 and the selected runtime path are qualified.

**Retained execution observation (2026-11-13):** these fixture and status operations completed in **3m34.165s** with a fresh task-owned Cargo target, state and output directory in the pinned development shell (`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`). Existing Cargo/Nix source caches were used. The fixture retained 68 evidence entries; canonical status readback reported generation 3, stopped, one restart and 13 invocations. This is not a cold anonymous bootstrap, an installed-component deployment or a Nix-built-binary result. [Public-source qualification #23](https://github.com/brittonr/aspen/issues/23) and [exact capability qualification #22](https://github.com/brittonr/aspen/issues/22) remain separate.

A clean hosted [Reproducible dependencies run](https://github.com/brittonr/aspen/actions/runs/43986122156)
at `1e81020e3` passed both acceptance jobs and their required aggregate
([exact source, commands and results](.cairn/archive/2026-11-16-admit-anonymous-runtime-sources/evidence/hosted-ci-20261116/full-43986122156-observation.json)).

| Hosted stage | Observed command duration |
| --- | ---: |
| Locked Cargo build through `nix develop` | 38m30s |
| Following README Cargo fixture and cleanup, in the same job | 8s |
| Default `nix build --no-link`, on a separate clean runner | 47m46s |
| Following full `nix flake check` | 1h51m23s |

These are provider step timings for that candidate, not a promised setup time.
The Nix job requested single-build/two-core scheduling. The README stage follows
the completed Cargo build; its eight seconds are not a cold bootstrap duration.
This establishes the recorded default build/workflow, not optional-feature,
external-runtime or release qualification. Earlier failed runs remain separate
in the [dependency policy](docs/reproducible-dependencies.md).

For the next step, follow the [checkout-to-first-check walkthrough](docs/handbook/contributing/checkout-to-first-check.md), [system-extension guide](docs/system-extension-runtime.md), or [deterministic harness and replay examples](docs/development.md#deterministic-harness-and-replay).

<a id="architecture-direction"></a>

## Architecture

Aspen uses a functional core with explicit effect boundaries:

| Layer | Responsibility |
| --- | --- |
| Deterministic core | Validate inputs, plan transitions, enforce bounds, and define canonical identities. |
| Runtime hosts | Admit work, order effects, supervise lifecycle, and record outcomes. |
| Adapters | Perform capability-rooted I/O through storage, transport, clock, process, and execution interfaces. |
| System extensions | Define service protocols and policies through admitted manifests and versioned runtime interfaces. |

The implementation is written in Rust. Iroh provides peer transport and content mechanisms; Redb stores local durable metadata; Wasmtime hosts admitted Wasm execution. Preserves and BLAKE3 define canonical values and identities. Nickel, Basalt/UCAN, and the repository's evidence tools supply policy and admission inputs. The [manifest](Cargo.toml) and [dependency policy](docs/reproducible-dependencies.md) record the selected versions and source identities.

Consensus is scoped to control-plane state. The replicated Raft and single-process control registry are in-tree implementations; neither calls Trellis. Trellis supplies choreography and JobDag helpers. Ordinary actor and content traffic does not require a single global consensus path. A replay result, transport acknowledgement, or signed receipt remains evidence only for its stated operation and profile.

Read the [system atlas](docs/atlas.md), [architecture guide](docs/architecture.md), and [core/adapter/extension boundaries](docs/distributed-system-fabric.md) for the detailed design.

## Project name and compatibility

**Aspen is the project name and the name used for public releases. Molten is its development codename.** The current branch, executable, Cargo packages, Nix attributes, and wire/schema identifiers still use `molten`. Use those exact identifiers in commands and integrations until a separately documented compatibility migration changes them.

The default branch is `molten`. The legacy `main` branch is a separate requirements and conformance reference. The `v0.1.0` tag records the [August 2026 internal pilot](docs/releases/molten-0.1.0.md); its historical names and evidence remain unchanged.

## Documentation

| If you want to… | Start here |
| --- | --- |
| Find the right guide | [Documentation map](docs/README.md) |
| Complete a development or operator task | [Workflow handbook](docs/handbook/README.md) |
| Understand implementation details | [Technical companion](docs/technical/README.md) |
| Inspect commands and subsystem examples | [Runtime and CLI reference](docs/runtime-reference.md) |
| Configure the bounded local NixOS service | [NixOS module and validation boundary](docs/nixos-service-module.md) |
| Evaluate release scope | [Release status](docs/release-status.md) and [security policy](SECURITY.md) |

<a id="references"></a>

Design sources and selected upstream cohorts are listed in [References](docs/references.md).

<a id="nix"></a>

## Development

Start with [Contributing](CONTRIBUTING.md) and the [first focused check](docs/handbook/contributing/checkout-to-first-check.md). The repository uses pinned tools, focused tests, deterministic simulation, and source/evidence gates. Report the exact checks you ran and the scope they cover.

The [development reference](docs/development.md) contains Cargo and Nix commands, nextest profiles, replay examples, and the ordered [strict Octet source-gate sequence](docs/development.md#strict-octet-source-gate-sequence).

<a id="public-release-checklist"></a>

The [external-release checklist](docs/release-status.md#before-an-external-release) tracks clean source acquisition, executed deployment evidence, cross-host qualification, notices, and private security reporting. A documentation update does not satisfy those runtime gates.

## Runtime reference index

These links preserve entry points from earlier versions of the README.

| Topic | Detailed reference |
| --- | --- |
| <a id="standalone-batch-orchestration-pilot"></a>Standalone batch pilot | [Separate workspace and integration scope](docs/standalone-batch-orchestration.md) |
| <a id="schema-identity-core-pilot"></a>Schema identity pilot | [Schema identity adapter](docs/schema-identity-core-pilot.md) |
| <a id="node-runtime-daemon"></a>Node lifecycle | [Node runtime and control commands](docs/runtime-reference.md#node-runtime-daemon) |
| <a id="iroh-endpoint-identity-persistence"></a>Endpoint identity | [Iroh identity persistence](docs/runtime-reference.md#iroh-endpoint-identity-persistence) |
| <a id="delivery-idempotency-diagnostics"></a>Delivery and duplicates | [Idempotency diagnostics](docs/runtime-reference.md#delivery-idempotency-diagnostics) |
| <a id="upgrade-session-protocol-drains"></a>Upgrade sessions | [Protocol drains](docs/runtime-reference.md#upgrade-session-protocol-drains) |
| <a id="retention-and-gc-pinning-diagnostics"></a>Retention and GC | [Pinning diagnostics](docs/runtime-reference.md#retention-and-gc-pinning-diagnostics) |
| <a id="supply-chain-provenance-diagnostics"></a>Supply-chain provenance | [Provenance diagnostics](docs/runtime-reference.md#supply-chain-provenance-diagnostics) |
| <a id="blob-ref-job-submission"></a>Job submission | [Blob-reference submission](docs/runtime-reference.md#blob-ref-job-submission) |
| <a id="job-dag-iroh-worker-ux"></a>Job DAG workers | [Worker command reference](docs/runtime-reference.md#job-dag-iroh-worker-ux) |
| <a id="coordination-control-plane-ux"></a>Coordination | [Control-plane commands](docs/runtime-reference.md#coordination-control-plane-ux) |
| <a id="production-readiness-receipts"></a>Readiness evidence | [Receipt scope](docs/runtime-reference.md#production-readiness-receipts) |
| <a id="testing-evidence-gates"></a>Test evidence | [Evidence gates](docs/runtime-reference.md#testing-evidence-gates) |

## License

Repository-owned code and documentation are licensed under **AGPL-3.0-or-later**; see [LICENSE](LICENSE). Third-party material retains its original terms and attribution in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md). Earlier permissive grants remain valid for material already received under them; see [license and notices](docs/release-status.md#license-and-notices).
