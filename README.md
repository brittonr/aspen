# Aspen

Aspen is a workload-neutral, policy-gated distributed-systems fabric for
operators and integrators who need explicit authority, bounded effects and
deterministic playback. Start by running one local extension fixture and
reading its canonical evidence, rather than starting a cluster. Pure primitives
own deterministic laws and plans; capability-rooted adapters perform effects;
optional manifest-installed extensions own distributed-service semantics.

**Molten is the internal codename**, not a second public product. This default
branch, Rust crates, CLI and schemas retain their `molten` identifiers. Legacy
[Aspen `main`](https://github.com/brittonr/aspen/tree/main) is a separate
clean-room requirements and conformance reference; this branch does not use
OpenRaft. The frozen August [`v0.1.0` internal pilot](docs/releases/molten-0.1.0.md)
is historical, not a newly tested candidate or a production-readiness claim.
There is no production-supported profile or qualified cross-host workflow.

## Try one local fixture

**Prerequisites:** work from the repository root of the reviewed checkout, keep
its manifest and locks intact, and use the pinned Nix development shell with
Rust `nightly-2026-05-26` and the selected native/linker dependencies. Have Nix
with flakes enabled, source-fetch access, and writable build/temporary storage.
The shell supplies tools, not a newly built `molten`. Required private/custom
sources must be obtainable through their reviewed public seed or local Radicle
profile; see [dependency bootstrap requirements](docs/reproducible-dependencies.md)
and [environment diagnosis](docs/handbook/contributing/diagnosing-development-environment-failures.md).
If shell evaluation or fetching fails, stop and retain that error; do not change
pins or substitute sibling overrides as clean public-build evidence.

**Execution observation (2026-11-13):** the block below completed in
**3m34.165s**, with a fresh task-owned Cargo target, state and output directory
inside the pinned Nix shell. The environment selected `CARGO_PROFILE_DEV_DEBUG=0`
and `CARGO_PROFILE_TEST_DEBUG=0`; Cargo/Nix source caches already existed. The
fixture retained 68 evidence entries, and canonical status readback reported
generation 3, stopped, with one restart and 13 invocations. This is not a cold
anonymous bootstrap or Nix-built-binary result; the remaining exact public-source
limitations stay recorded under [#23](https://github.com/brittonr/aspen/issues/23).

After the environment prerequisite succeeds, copy this from the repository root:

```sh
nix develop
```

Then, in that shell, still at the repository root:

```sh
(
  set -eu
  mkdir -p target
  WORK=$(mktemp -d "$PWD/target/first-workflow.XXXXXX")
  export CARGO_TARGET_DIR="$WORK/cargo-target"
  printf 'Retain this task-owned output for review: %s\n' "$WORK"
  cargo run -- system-extension run-fixture \
    --profile sandboxed-component \
    --out "$WORK/system-extension"
  cargo run -- system-extension show \
    --status "$WORK/system-extension/status.preserves"
  test -s "$WORK/system-extension/manifest.preserves"
  test -s "$WORK/system-extension/status.preserves"
  test -s "$WORK/system-extension/recovered-status.preserves"
  test -s "$WORK/system-extension/upgraded-status.preserves"
  test -s "$WORK/system-extension/rolled-back-status.preserves"
  find "$WORK/system-extension/evidence" -type f -name '*.preserves' -print
)
```

In that isolated run both commands exited zero. The first printed
`system-extension fixture ok profile=sandboxed-component` plus manifest/status
refs and the evidence count. `show` parsed the status and printed extension,
service, generation, phase, profile, health, restart/invocation counts and its
ref. Review the retained indexed lifecycle, callback, effect-completion,
migration and readiness receipts in `evidence/`; file presence or terminal text
alone is not qualification. The fixture exercises initialization, start,
request, checkpoint, retryable failure, restart/recovery, upgrade/rollback,
drain and shutdown under the no-WASI `sandboxed-component` profile. See the
[host contract](docs/system-extension-runtime.md) and
[artifact producer](src/cli/runtime/system_extension/ops.rs).

No pre-existing output is deleted. The printed `WORK` directory owns this run's
build and fixture outputs. Retain it on failure or until review is complete;
optional cleanup is `rm -rf -- /exact/printed/first-workflow.DIRECTORY`, replacing
that entire operand with this run's printed path only. Do not delete `target/`,
other runs or node roots. Cargo/Nix source caches are not task-owned cleanup.

### Limits beside the example

| Not established by this fixture | Owning work / exact boundary |
| --- | --- |
| Clean anonymous cold bootstrap or measured setup duration | [#23](https://github.com/brittonr/aspen/issues/23), [first-workflow qualification #25](https://github.com/brittonr/aspen/issues/25) |
| Consensus, durability, protocol compatibility or extension semantic correctness | [Exact source/build/profile capability qualification](docs/capability-qualification-report.md), [#22](https://github.com/brittonr/aspen/issues/22) |
| Successful signed node/NixOS startup | [Source-trust provisioning and positive rail #16](https://github.com/brittonr/aspen/issues/16); [NixOS module boundary](docs/nixos-service-module.md) |
| Cross-host transport or a supported two-host workflow | [#15](https://github.com/brittonr/aspen/issues/15); loopback is not two-host evidence |
| Production or external-release readiness | [Public release checklist](docs/handbook/contributing/workspace-and-toolchain-reference.md#public-release-checklist), [historical pilot scope](docs/releases/molten-0.1.0.md) |

No two-host variant is offered until #15 and the selected runtime path have
actual qualification. Fixture evidence grants no runtime, operation or release
authority.

## Choose your next task

- **Understand ownership:** [architecture](docs/architecture.md),
  [system atlas](docs/atlas.md) ([#4](https://github.com/brittonr/aspen/issues/4))
  and [fabric boundaries](docs/distributed-system-fabric.md). The atlas is a map,
  not a support matrix: keep its [open questions](docs/atlas.md#open-questions)
  beside [unsupported cross-host work #15](https://github.com/brittonr/aspen/issues/15)
  and [exact capability evidence #22](https://github.com/brittonr/aspen/issues/22).
- **Find a workflow:** [handbook](docs/handbook/README.md),
  [documentation map](docs/README.md), [technical companion](docs/technical/README.md).
- **Find a command or receipt:** [CLI and artifact reference](docs/handbook/cli-navigation/command-families-and-artifact-reference.md).
- **Review an operational claim:** [exact capability qualification](docs/capability-qualification-report.md)
  separates implementation from actual source/build/profile/environment evidence.
- **Report or contribute:** read [Security](SECURITY.md), [Contributing](CONTRIBUTING.md)
  and the [first focused check](docs/handbook/contributing/checkout-to-first-check.md).

## Development

The canonical [workspace and toolchain reference](docs/handbook/contributing/workspace-and-toolchain-reference.md#development)
owns development commands, semantic nextest profiles, Nix rails and receipt
readbacks. Use the complete [strict Octet source-gate sequence](docs/handbook/contributing/workspace-and-toolchain-reference.md#strict-octet-source-gate-sequence),
not an abbreviated substitute. [Proof workflow](docs/proof-workflow.md) owns
proof-affecting changes. All reference commands run from the repository root.

## Reference links retained from the long README

Old section anchors remain here and route to the exact retained section. Long
CLI examples and the archived roadmap have moved to existing references, not a
second reference page. Historical receipts remain historical.

| Previous section | Canonical destination |
| --- | --- |
| <a id="architecture-direction"></a>Architecture direction | [Architecture direction](docs/handbook/contributing/workspace-and-toolchain-reference.md#architecture-direction) |
| <a id="standalone-batch-orchestration-pilot"></a>Standalone batch orchestration pilot | [Standalone batch orchestration pilot](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#standalone-batch-orchestration-pilot) |
| <a id="schema-identity-core-pilot"></a>Schema identity core pilot | [Schema identity core pilot](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#schema-identity-core-pilot) |
| <a id="node-runtime-daemon"></a>Node runtime daemon | [Node runtime daemon](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#node-runtime-daemon) |
| <a id="iroh-endpoint-identity-persistence"></a>Iroh endpoint identity persistence | [Iroh endpoint identity persistence](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#iroh-endpoint-identity-persistence) |
| <a id="delivery-idempotency-diagnostics"></a>Delivery idempotency diagnostics | [Delivery idempotency diagnostics](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#delivery-idempotency-diagnostics) |
| <a id="upgrade-session-protocol-drains"></a>Upgrade session protocol drains | [Upgrade session protocol drains](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#upgrade-session-protocol-drains) |
| <a id="retention-and-gc-pinning-diagnostics"></a>Retention and GC pinning diagnostics | [Retention and GC pinning diagnostics](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#retention-and-gc-pinning-diagnostics) |
| <a id="supply-chain-provenance-diagnostics"></a>Supply-chain provenance diagnostics | [Supply-chain provenance diagnostics](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#supply-chain-provenance-diagnostics) |
| <a id="blob-ref-job-submission"></a>Blob-ref job submission | [Blob-ref job submission](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#blob-ref-job-submission) |
| <a id="job-dag-iroh-worker-ux"></a>Job DAG Iroh worker UX | [Job DAG Iroh worker UX](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#job-dag-iroh-worker-ux) |
| <a id="coordination-control-plane-ux"></a>Coordination control-plane UX | [Coordination control-plane UX](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#coordination-control-plane-ux) |
| <a id="production-readiness-receipts"></a>Production readiness receipts | [Production readiness receipts](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#production-readiness-receipts) |
| <a id="testing-evidence-gates"></a>Testing evidence gates | [Testing evidence gates](docs/handbook/cli-navigation/command-families-and-artifact-reference.md#testing-evidence-gates) |
| <a id="nix"></a>Nix | [Nix](docs/handbook/contributing/workspace-and-toolchain-reference.md#nix) |
| <a id="public-release-checklist"></a>Public release checklist | [Public release checklist](docs/handbook/contributing/workspace-and-toolchain-reference.md#public-release-checklist) |
| <a id="references"></a>References | [References](docs/handbook/contributing/workspace-and-toolchain-reference.md#references) |

## License

Repository-owned source—including cores, contracts, schemas, fixtures, templates, documentation, and policy—is licensed under `AGPL-3.0-or-later`; see [LICENSE](LICENSE). Third-party dependencies, vendored manifest snapshots, generated material containing upstream code, and external references retain their original terms; see [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

Earlier permissive grants remain valid, including for byte-identical content previously received under those grants. The current declaration applies prospectively to project-owned contributions and does not revoke rights already received. Licensing metadata is distribution evidence only: it does not establish contributor authority, legal compliance, runtime correctness, or release eligibility. AGPL is an accepted project distribution choice; the typed distribution profile records project-required notices and source-export evidence, is not legal advice, and does not prove compliance in every jurisdiction.

The focused [license policy](license-policy.tsv) covers every current workspace package exactly once, including `molten-node-host`; [the boundary checker](tools/check-license-boundary.rs) rejects missing package rules before checking license expressions and notice artifacts, and its `notice-revision` rules compare current third-party notice rows with the revisions selected in `Cargo.toml`. This does not make a private Radicle source obtainable without its documented public seed or a local Radicle profile.

