# Repository Guidelines

AI-assistant guidance for **Molten** in `brittonr/aspen`. Molten contributors maintain this guide alongside the manifests, governing docs, and accepted requirements.

## Project Overview

Molten is a workload-neutral, policy-gated distributed-systems fabric built around canonical Preserves envelopes. Pure primitives own deterministic laws/plans; capability-rooted adapters perform effects; manifest-installed system extensions own distributed-service semantics.

Non-negotiable boundaries:

- **Deterministic playback:** identical artifacts, dependency closure, initial state, policy/schema refs, handler profile, and seed or recorded effect log reproduce identical canonical traces, receipts, outputs, and final-state hashes.
- **Identity:** BLAKE3 over canonical packed Preserves bytes, never Rust layout, debug output, or rendered logs.
- **Authority:** identity, verification, connectivity, and successful output do not grant permission. Logs/JUnit cannot override canonical deny receipts.
- **Consistency:** Trellis-backed Raft is for control-plane state, not ordinary actor traffic. OpenRaft is not used.

## Architecture & Data Flow

| Owner | Responsibility |
| --- | --- |
| Root `molten` crate (`src/`) | Facade, canonical runtime kernels, encoding, ports, adapters, orchestration, CLI. Not all root code is pure. |
| `crates/molten-core/` | In-memory decision cores, transitions, typed identities, plans. No filesystem/process/network/clocks or execution/backend runtimes. |
| `crates/molten-node-host/` | Capability-rooted node state, typed local stores, shared `MoltenError`/`Result`; no CLI/workload/release semantics. |
| `crates/molten-release-policy/` | Normalizes policy/repository observations for the pure `molten_core::release_dependency` validator. |

Typical flow: Nickel exports → validated typed inputs → event/transition preview → policy/capability/budget admission → commit or rollback → admitted shell effects and canonical receipts.

Shells own effect order; adapters supply mechanisms, not policy. Preserve **authority check → transition → persist intent → effect → commit** where required. Post-effect failure is uncertainty, not policy denial; see `src/fabric_membership/{ports,shell}.rs`.

Iroh supplies peer transport/blobs/docs; Redb supplies durable metadata; Wasmtime and Steel supply admitted execution/predicates. Runtime snapshots carry explicit logical time, RNG state, and effect sequence.

## Key Directories

| Path | Purpose / caution |
| --- | --- |
| `src/runtime/`, `src/fabric_*/`, `src/world*/` | Runtime and domain composition; public module names often differ from directories. |
| `src/cli/`, `src/main/` | Parsing, dispatch, imperative orchestration. |
| `tests/` | Integration entrypoints, split suites in `parts/`, canonical `fixtures/`, traceability matrix. |
| `config/` | Nickel contracts/profiles/negative fixtures and generated exports; edit sources, not exported JSON. |
| `checks/`, `scripts/`, `tools/` | Focused Octet workspaces, maintenance codemods, hashing, Tracey/license guards, ast-grep audits. |
| `docs/` | Governing docs; `handbook/` gives workflows, `technical/` explains implementation. |
| `.cairn/`, `cairn-policy/` | Accepted specs, active changes, archives; reviewed Nickel lifecycle policy and generated JSON. |
| `wit/`, `evidence/`, `release/` | Component contracts, append-only evidence, frozen pilot release material. |

Do not extend non-workspace `crates/aspen-*` or use legacy `openspec/` as lifecycle authority. Preserve `.cairn/archive/`, older `cairn/archive/`, and historical evidence. Leave `.pi/`, `.agent/`, `.claude/`, `.dogfood-runs/`, and `mutants.out/` alone; do not hand-edit `target/`. Never recreate removed components to satisfy historical prompts.

## Development Commands

Run from this repository inside `nix develop` (or direnv's `use flake`). The shell supplies tools, **not a freshly built `molten` binary**.

```sh
nix develop
cargo check --workspace --all-targets
nix build
cargo run -- --help

cargo nextest run
cargo nextest run --profile fast-core
cargo test --lib distributed                      # focused Cargo fallback
cargo test --test cliharness ci_run_receipt

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo octet check --artifact-dir target/octet
cargo octet check -p molten --artifact-dir target/octet-lib -- --lib

nix build .#checks.x86_64-linux.nextest             # hermetic test check
nix build .#checks.x86_64-linux.nixos-vm-multinode  # actual VM integration
nix flake check                                  # full CI rail
```

`nix run .#nextest-ci` wraps local `cargo nextest run --profile ci`; it is not the hermetic test check. `nextest-config` is the flake check for profile readback.

When source-gate evidence is required, follow the complete [strict sequence](README.md#development): both Octet runs → object-corpus receipt → artifact import/gate/remediation plan → Cargo tests → Clippy → strict Cairn validation. Octet `warning-only` denies even if the producer exits zero.

```sh
CAIRN_ROOT="${CAIRN_ROOT:-../cairn}"
nix run "path:$CAIRN_ROOT#cairn" -- validate --root . \
  --policy "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict
```

This selects the sibling provider's policy, not the local vendored export. Installed hooks enforce whitespace/formatting/Octet on commit and push; pre-push adds non-strict Cairn validation, even for prose-only commits.

### Repository workflow

- Mainline: **`molten`**; remote: `git@github.com:brittonr/aspen.git`. Never merge/overwrite legacy `main`, force-push, or create pull requests.
- Cairn implementation uses dedicated worktrees from current `origin/molten`. Fetch before integration; merge advanced upstream into the change branch and rerun checks. Integrate by fast-forward; verify the completion commit is on `origin/molten` before removing the worktree.
- Gate order: proposal → design → tasks → implementation → validation → review → sync → archive → commit → push. Use `cairn change list --root .`; guidance/generated plans do not authorize completion or publication.
- Resolve requirement/design changes first. Preserve unrelated work and failed-attempt evidence; stop only task-owned processes and remove only task-owned scratch paths.

## Code Conventions & Common Patterns

- **Facade navigation:** follow `#[path]` mounts in `src/lib.rs` and `compat_module!` mappings in `src/parts/lib/p000/body.rs`. Example: `src/node/runtime.rs` → internal `kernel` → public `node_runtime`. `core_api` re-exports `molten_core`; do not invent a second facade convention.
- **Textual parts:** `parts/**/body.rs` contains real implementation sharing the enclosing scope/imports. Follow the actual `include!` list, not numeric order: `src/node/runtime.rs` loads `p006` before `p001`; `src/preserves/rail.rs` loads `p002` before `p001`.
- **Errors:** canonical `MoltenError`/`Result` live in `crates/molten-node-host/src/error/mod.rs`; `src/error/mod.rs` only re-exports them. Reuse subsystem Snafu errors and `FabricPortError`/`FabricPortResult<T>`; preserve failure class/phase and uncertainty until outer-boundary conversion.
- **Injection/authority:** use small port traits and explicit `*Input` values; select live/simulation implementations in shells. Ongoing I/O uses `NodeStateRoot`, namespace capabilities, and typed store roots, not reconstructed ambient paths. A parsed reference is not authorization.
- **Async/state:** build Tokio runtimes and `block_on` at imperative boundaries; keep live I/O/spawns/timers out of pure cores. Preserve preview/check/commit-or-rollback semantics, explicit logical/RNG state, deterministic iteration (`BTreeMap`/`BTreeSet` where established), and canonical snapshot projections.
- **Style/naming:** nightly rustfmt, 120-column code, item-granular imports grouped Std/External/Crate; follow `*Input`/`*Outcome`/`*Issue`, typed-reference, and predicate-boolean patterns. Repair Octet findings instead of disabling lints. Import/bool codemods support `--dry-run`; inspect limitations. The burn-down script can mutate config and publish—it is not a read-only check.
- **Traceability:** link actual requirements with `// r[impl molten.<requirement>]` and `// r[verify ...]`. `tools/tracey/inherited_debt_guard.rs` rejects debt drift/dangling references; do not expand baselines to conceal missing coverage.

## Important Files

| File | Role |
| --- | --- |
| `src/lib.rs`, `src/parts/lib/p000/body.rs` | Physical/public module maps and core/prelude/error exports. |
| `src/main.rs` → `src/main/root.rs` → `src/main/root/command.rs` | Synchronous entrypoint, Clap dispatch, included command definitions. |
| `Cargo.toml`, `flake.nix` | Workspace/binaries/features/dependencies/Octet scope; Nix builds, apps, shell, and checks. |
| `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `dylint.toml` | Compiler/source-policy settings; Octet has no disabled lints. |
| `.config/nextest.toml`, `tests/evidence-matrix.ncl` | Selection/timeouts/retries and traceability authoring data. |
| `.pre-commit-config.yaml`, `.github/workflows/reproducible-dependencies.yml` | Hooks and CI's `nix flake check` command. |

Start with [docs/README.md](docs/README.md). Read `docs/architecture.md`, `docs/distributed-system-fabric.md`, and `docs/modularity-boundaries.md` for ownership; `docs/fabric-port-ownership.md` and `docs/node-state-filesystem-authority.md` for effects. Runtime/dataspace changes must update architecture documentation. World-state changes follow the matching `docs/world-*.md` protocol. Companion articles and discrepancy notes are not requirement changes or approved exceptions.

## Runtime/Tooling Preferences

- **Pinned tools:** Rust edition 2024/resolver 3, `nightly-2026-05-26`; nextest ≥ `0.9.136`; Nickel CLI `1.17.0` plus the embedded cohort in `docs/nickel-toolchain.md`. Use the Nix shell for tests/lints. Octet has a separately pinned producer toolchain; ambient `cargo-octet` is not provenance evidence.
- **Hosted Wasm:** Wasmtime 45 and `wit/molten-component-runtime/`; do not add `wasm32-unknown-unknown` cross-builds or assume shell CLI versions equal library versions.
- **Optional features:** `doltlite-oracle`, `executable-extents`, `profiler*`, `world-snapshot-vm-cohort`. No unqualified `--all-features` baseline; defaults do not cover optional backends. Profiling is platform-scoped and diagnostic-only.
- **Dependency updates:** coordinate the release profile, Cargo manifest/lock, Nix input/lock, `crate-hashes.json`, and both unit2nix plans (`build-plan.json`, `release-policy-build-plan.json`). Follow [the pin procedure](docs/reproducible-dependencies.md#updating-pins): hash with `scripts/git-source-hashes.sh` before regenerating plans; never hand-edit locks or let unit2nix invent Git-source hashes. Private inputs need authentication; sibling overrides are development-only, not release identities.
- **Upstream inputs:** follow `THIRD_PARTY_LICENSES.md`/`license-policy.tsv`, preserve notices, and re-review changed vendor versions/checksums. Aspen `main` is a clean-room requirements/conformance reference, not permission to copy implementation, comments, or fixtures without compatible relicensing.

## Testing & QA

- **Frameworks:** synchronous `#[test]`, bounded Hegel properties (`#[hegel::test(test_cases = N)]`, `tc.draw(...)`), Tokio tests for async shells. CLI suites run the real binary and assert canonical receipts, not only stdout. Add meaningful deny/boundary cases, not incidental source-text tests.
- **Selection/determinism:** `fast-core`, `harness`, `cli`, and `distributed-simulation` are name-filtered profiles scoped to package `molten`, not all workspace packages. Inspect filters. CI/deterministic profiles use zero retries and fail flaky results; `exploratory` alone retries and is diagnostic. A `vm-platform` name does not prove VMs ran.
- **Test authority:** use `TestWorkspace::new(label)` with narrow typed roots, or `ProcessWorkspace` for path-oriented processes; retain guards through async/process work. Never clean directories by PID/prefix inference. Follow `docs/test-workspace-authority.md`. `tests/src/test/support.rs` is a live bridge into `src/test/parts/support/`; do not delete the nested mirror indiscriminately.
- **Fixtures/replay:** compare canonical event/receipt/final-state refs. `tests/fixtures/fabric-boundary/` is frozen; changes require an approved versioned boundary change. `examples/*.preserves` feed the CLI harness. DoltLite also requires its feature, ignored-test selection, and exact external cohort.
- **Evidence:** changed/evidence requirements need positive and negative coverage or a reviewed exemption; matrix fixture refs/logs/JUnit are not recorded execution. Follow `docs/proof-workflow.md` and `docs/distributed-testing.md`. Before behavioral integration, run focused tests, formatting, Clippy, Octet, applicable Cairn strict validation, and required Nix rails. Prose-only changes use source/command review and document-link/structure checks under the documented exemption. Report only checks actually run.
