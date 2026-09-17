# Repository Guidelines

Guidance for AI coding assistants working in this repository (`brittonr/aspen`, containing **Molten**). Read this before editing anything.

## Project Overview

Molten is a workload-neutral, policy-gated distributed-systems fabric built around a canonical Preserves envelope spine. Pure primitives own deterministic laws and plans; capability-rooted adapters perform external effects without defining authority; manifest-installed system extensions own distributed-service semantics.

Core laws (non-negotiable when changing code):

- **Deterministic playback**: the same artifacts, dependency closure, initial state, policy/schema refs, handler profile, and seed or recorded effect log must reproduce identical traces, receipts, outputs, and final state hash.
- **Preserves + BLAKE3** define communication, storage, policy, and evidence identity. Identity is BLAKE3 over canonical Preserves bytes — never Rust struct layout or debug output.
- **Evidence gating**: every trust-boundary effect emits canonical receipts (`molten.*.v1` schema ids). Terminal output, QEMU/systemd logs, and JUnit XML are diagnostics and can never override a canonical deny receipt.
- **Authority separation**: identity/verification do not grant membership or authority; Raft (Trellis-backed) is for control-plane state only; OpenRaft is not used.

## Architecture & Data Flow

Functional-core / imperative-shell layering across four crates:

| Crate | Role |
|---|---|
| `molten` (root, `src/`) | Facade + adapters + CLI. Mounts ~85 domain modules under stable public names; re-exports pure cores and node-host |
| `crates/molten-core` | Pure laws (fabric/world/capability/policy/planning). No filesystem, process, network, clock, tokio, or Preserves deps — stdlib + blake3/serde + pinned git cores only |
| `crates/molten-node-host` | Capability-rooted node state + typed local stores (cap-std). Owns canonical `MoltenError`/`Result`, `node_state`, `local_store`; the root crate re-exports them |
| `crates/molten-release-policy` | Binary: Nickel release profile + repo observation → pure `validate_release_dependencies` |

`crates/aspen-*` directories are vestigial (wip/fixtures only, not workspace members) — never extend them. All active code uses `molten-*` names.

Data flow:

1. Nickel config/policy manifests → typed configs (e.g. `RuntimeStartupConfig::from_nickel_export_json`).
2. All communication crosses the envelope spine as canonical Preserves; BLAKE3 over those bytes is identity.
3. Turn cycle: event → pending assertions/messages/effects → pure validation + policy/capability/budget admission → commit or rollback.
4. Effects execute only through admitted fabric ports: pure cores in `molten-core` plan; adapter shells in the root crate execute (port traits → `live/` adapters or deterministic `simulation.rs`; both satisfy the same contracts).
5. Trust-boundary actions emit canonical receipts; retention/GC and destructive ops are evidence-gated.
6. Iroh (gossip/blobs/docs) = remote substrate; Redb = durable metadata; Wasmtime (deny-ambient WASI) + Steel = execution/predicates.

## Key Directories

| Directory | Purpose |
|---|---|
| `src/` | Root crate: domain modules (`fabric_*`, `world_*`, `runtime/`, `capability/`, `node/`, `preserves/`, …), `cli/` + `main/` (binary-only) |
| `crates/` | Workspace members: `molten-core`, `molten-node-host`, `molten-release-policy` (plus dead `aspen-*`) |
| `tests/` | ~10 active integration binaries + `fixtures/` (canonical Preserves/JSON) + `evidence-matrix.ncl` |
| `docs/` | ~100 governing documents — see Important Files |
| `.cairn/` | **Active lifecycle**: `changes/` (~50 open), `specs/` (~62 accepted requirements with `r[molten....]` ids) |
| `cairn/archive/` | ~437 completed historical slices — retain untouched |
| `cairn-policy/` | Vendored Nickel Cairn policy (from cairn@fde71b2, see `cairn-policy/UPSTREAM.md`); `generated/cairn-policy.json` is the runtime input |
| `checks/` | 19+ standalone per-module Octet deny-all lint workspaces assembled by the flake |
| `config/` | Nickel world/fabric configuration domains |
| `scripts/`, `tools/` | Standalone `cargo -Zscript` codemods/guards, ast-grep authority rules, tracey tooling |
| `wit/` | Wasm component contract (`molten:component-runtime@1.0.0`) hosted by Wasmtime 45 |
| `release/` | Frozen pilot release manifests (evidence-only) |
| `evidence/` | Tracked canonical evidence — append-only, never rewrite |
| `openspec/` | Drained legacy marker; OpenSpec skills are legacy guidance, not lifecycle authority |

Do not touch: `.pi/`, `.agent/`, `.claude/`, `.dogfood-runs/`, `mutants.out/`, `target/` (gitignored session/scratch state). `.claude/prompts/` are historical debug prompts — verify their scripts and target components exist before use; if absent, report the obsolete procedure; never recreate removed components to satisfy a prompt.

## Development Commands

Enter the dev shell first (`nix develop` — provides steel, nickel, wasmtime, cargo-nextest, the pinned Rust toolchain; direnv `use flake` works).

```sh
# Build / general
cargo check --workspace --all-targets
nix build                        # hermetic package build (unit2nix from build-plan.json)

# Test (nextest required, >= 0.9.136; cargo test is the fallback)
cargo nextest run
cargo nextest run --profile fast-core     # also: harness, cli, distributed-simulation,
                                          # vm-platform, dogfood-soak, ci, deterministic, exploratory
cargo test vat                            # focused: module substring
cargo test --test cliharness ci_run_receipt  # focused integration binary
nix run .#nextest-ci                      # hermetic CI profile

# Lint / format (pinned nightly required — rustfmt.toml uses unstable options)
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo octet check                         # Octet dylint catalog (dylint.toml: disabled_lints = [])
cargo octet check -p molten -- --lib

# Release-dependency validation
nix develop -c cargo run -p molten-release-policy -- --root . \
  --evidence-source valence-integrity=../valence --evidence-source octet-cutover=../octet

# Broad Nix rails
nix build .#checks.x86_64-linux.{nextest,dogfood-local-node,nixos-vm-multinode,nextest-config}
nix flake check                           # ~100 hermetic checks; this IS CI
```

Strict source-gate sequence (README "strict" rail, in order): `cargo octet check` (root + `-p molten --lib`) → octet object corpus receipt → `molten test octet artifacts` import/gate/plan → `cargo test` → `cargo clippy --all-targets -- -D warnings` → Cairn strict validate:

```sh
CAIRN_ROOT="${CAIRN_ROOT:-../cairn}"
nix run "path:$CAIRN_ROOT#cairn" -- validate --root . \
  --policy "$CAIRN_ROOT/cairn-policy/generated/cairn-policy.json" --strict
```

Pre-commit hooks run `git diff --check`, `cargo fmt --check`, and `cargo octet check` on commit and push; Cairn validate runs pre-push. Requires a sibling `../cairn` checkout by default.

### Git & lifecycle workflow (repo-specific, overrides defaults)

- Mainline is **`molten`**, not `main`. Remote: `git@github.com:brittonr/aspen.git`. Never merge or overwrite legacy `main` history; never force-push; never create pull requests.
- Cairn completion workflow substitutes `origin/molten` wherever the general workflow names `origin/main`: create dedicated implementation worktrees from current `origin/molten` → fetch `origin` again before integration → if `origin/molten` advanced, merge it into the change branch and rerun checks → integrate into `molten` by fast-forward → verify `origin/molten` contains the completion commit before worktree removal.
- Lifecycle authority is native Cairn under `.cairn/` (`cairn change list --root .` for the active set). Gate order per change: proposal → design → tasks gates, then implementation → validation → review → sync → archive → commit → push. Preserve all required gates; this exception changes branch names only.
- Operational discipline: diagnose recoverable errors in approved scope before continuing; resolve requirement/design changes before implementation; complete required lifecycle review before protected actions. Bounded retries tied to a fix or new evidence. Stop only task-owned processes; remove only task-owned scratch paths. Preserve unrelated changes and failed-attempt evidence. Report passed/blocked/budget-exhausted results without claiming unrun tests.

## Code Conventions & Common Patterns

- **Module aliasing**: public module names ≠ directory names. `src/lib.rs` mounts dirs via `#[path]` under internal names, then `compat_module!(public, internal)` re-exports (e.g. `src/error/mod.rs` → `molten::error` via `failures`; `deterministic/replay.rs` → `playback`). Same pattern in `src/main.rs` (`object_port` / `cli_artifact`). Always resolve a `molten::*` path through `src/lib.rs` before editing; grep both names.
- **Parts assembly**: much code lives in `src/<module>/parts/<x>/pNNN/body.rs` files stitched with `include!` (see `src/preserves/rail.rs`, `src/chunk/store.rs`). Edit part files in NNN order; the `mod.rs` you are reading may be mostly empty.
- **Errors**: canonical `MoltenError` (manual, stringly enum + Display + `From<io::Error>`) lives in `crates/molten-node-host/src/error/mod.rs` and is re-exported everywhere. Scoped subsystem errors use snafu with `#[snafu(display(...))]`. Fabric ports have their own `FabricPortError`/`FabricPortResult<T>` vocabulary with conversions both ways. Never introduce a second error convention.
- **Async**: tokio multi-thread runtime is built in the imperative shell; `runtime.block_on` at the CLI/lib boundary. `tokio::spawn`/`select!`/`time::sleep` only inside live adapters. Pure cores and most tests stay sync (determinism-first).
- **Ports / DI**: effects go through small port traits in `fabric_*/ports.rs` returning `FabricPortResult<T>`, with `live/` and `simulation.rs` implementations behind the same trait. Thread explicit `*Input` structs to functions (e.g. `ControlLiveServeInput`). FS authority is cap-std (`cap-std`/`cap-fs-ext`) rooted in node-host only.
- **Identity & naming**: schema id consts `"molten.<domain>.<thing>.v1"`; canonical projections named `Canonical<Thing> { report, <thing>_ref, value: IOValue }`; booleans use predicate prefixes (Octet `bool_naming`); imports are item-granular, grouped Std/External/Crate.
- **Traceability markers**: `// r[impl molten.<req>]` / `// r[verify ...]` comments link code to `.cairn/specs/` requirement ids; `tools/tracey/inherited_debt_guard.rs` denies new uncovered or dangling refs. Add markers when implementing spec'd requirements.
- **tigerstyle**: registered via `#![register_tool(tigerstyle)]` (crate roots and integration test binaries); suppress with `#![allow(tigerstyle::..., reason = "...")]` — reason required.
- **Purity boundary**: nothing in `molten-core` may read files, spawn, print, access clocks, or do network I/O. Boundary source-scan tests in `tests/` (`fabric_simulation_boundary.rs` etc.) enforce this by `include_str!`-ing real sources and asserting forbidden terms absent — keep them passing.
- **Preserve compatibility**: pure cores avoid a Preserves dependency; canonical projection happens at the root-crate boundary.

## Important Files

| File | Role |
|---|---|
| `src/main.rs` → `src/main/root.rs` → `src/main/root/command.rs` | `molten` binary: clap `Cli`/`Top`/`Test` tree (~19 top, ~40 test subcommands), dispatch |
| `src/lib.rs` | Facade: `compat_module!` aliases, `#[path]` mounts, `core_api` re-export of `molten_core` |
| `crates/molten-core/src/lib.rs` | Pure decision cores; header documents the purity contract |
| `crates/molten-node-host/src/{lib.rs,error/mod.rs}` | Node state authority, `MoltenError` |
| `crates/molten-release-policy/src/main.rs` | Release-dependency validator binary |
| `Cargo.toml` | Workspace, features (`doltlite-oracle`, `executable-extents`, `profiler*`, `world-snapshot-vm-cohort`), rev-pinned git deps, `[workspace.metadata.octet]` scope |
| `flake.nix` + `build-plan.json` (+ `release-policy-build-plan.json`) | Nix build: unit2nix unit graphs (no IFD), dev shell, ~100 checks |
| `.config/nextest.toml` | Test profile matrix |
| `rustfmt.toml`, `clippy.toml`, `dylint.toml`, `rust-toolchain.toml` | Format/lint/toolchain pins |
| `cairn-policy/default.ncl` + `generated/cairn-policy.json` | Lifecycle policy (Nickel source → runtime JSON) |
| `tests/evidence-matrix.ncl` | Traceability matrix source |
| `license-policy.tsv` + `tools/check-license-boundary.rs` | License boundary (workspace source is AGPL-3.0-or-later) |

Load-bearing docs (read before touching their domain): `docs/architecture.md` (master map; runtime dataspace changes must be documented here), `docs/distributed-system-fabric.md` (ownership law), `docs/fabric-port-ownership.md` (port inventory), `docs/modularity-boundaries.md` (crate purity + dependency classes), `docs/node-state-filesystem-authority.md` (node-host authority), `docs/nickel-toolchain.md` (pinned Nickel cohort), `docs/proof-workflow.md` (proof checklist for Cairn changes), `docs/test-workspace-authority.md` (`TestWorkspace` RAII temp roots), `docs/distributed-testing.md` (evidence layers), `docs/reproducible-dependencies.md` (pin-update procedure), `docs/world-*.md` (world-state lifecycle protocols).

## Runtime/Tooling Preferences

- **Nix-first**: use `nix develop` (or direnv) for anything beyond plain `cargo check`; the flake supplies tool versions plain Cargo lacks (Nickel 1.17.0 cohort, steel, wasmtime, nextest, octet toolchain).
- **Rust**: pinned `nightly-2026-05-26` via `rust-toolchain.toml` — formatting and lints require it (nightly rustfmt options). Edition 2024, resolver 3.
- **cargo-nextest** ≥ 0.9.136 is the test runner; `cargo test` only as fallback.
- **cargo-octet** (dylint catalog) is mandatory lint infrastructure; lint suppression is not permitted — repair instead (`scripts/octet-qualify-imports.rs`, `scripts/octet-predicate-names.rs` codemods; `scripts/octet-burndown.scm` guard loop).
- **Cairn CLI** comes from the sibling `../cairn` checkout (`CAIRN_ROOT`); the vendored `cairn-policy/` refresh procedure is in `cairn-policy/UPSTREAM.md` (`cairn policy export` needs a real `target/` dir, not a symlink).
- **Dependencies**: git deps are revision-pinned (OnixResearch SSH, Radicle seeds, git.onix.computer). A version bump must update Cargo.toml rev + Cargo.lock + flake input + `build-plan.json` together (7-step procedure in `docs/reproducible-dependencies.md`). `--override-input` sibling substitution is dev-only and is rejected as release evidence. `vendor/` snapshots are license-manifest material — any version/checksum change requires re-review (`THIRD_PARTY_LICENSES.md`).
- **No wasm32 cross-builds**: Wasm is a hosted target (Wasmtime 45 + `wit/molten-component-runtime`); don't add `wasm32-unknown-unknown` targets.
- **`nix flake check` is CI**: the single GitHub workflow runs exactly that; local `nix flake check -L` before pushing rails.

## Testing & QA

Two layers:

1. **In-crate tests** (the bulk): `src/**/tests.rs` and `src/**/parts/**/body.rs` — `#[test]` (sync, default), `#[hegel::test]` property tests, `#[tokio::test]` only for live raft/iroh/transport shells.
2. **Integration binaries** (`tests/*.rs`, ~10 active): `cliharness.rs` (spawns the real `molten` binary, asserts stdout + canonical `.preserves` receipts, assembled from `include!` parts p000–p022), `nativesystemextension.rs`, `worldcommit.rs`, `executable_extent.rs`, `content_replication.rs`, source-scan boundary tests, `moltennodehostfacade.rs`, feature-gated `doltlite_oracle.rs` (`--features doltlite-oracle`, `#[ignore]` live oracle).

Fixtures are checked-in canonical data consumed via `include_str!` from `tests/fixtures/`; `examples/*.preserves` are harness suites for `molten test run`.

Conventions and expectations:

- **Hegel property tests**: `#[hegel::test(test_cases = N)]` with explicitly bounded generators (`hegel::generators::integers/binary/booleans` with min/max); counterexamples get promoted to fixtures (`hegel-counterexample-fixture-v1`).
- **Determinism law**: CI/release evidence profiles use `retries = 0` and `flaky-result = "fail"` — a pass-after-retry is not deterministic pass evidence. `exploratory` is the only retrying profile.
- **Fixture subcommands** (`molten test vat run-fixture`, `molten test run/replay`, `molten test replayfixture ...`) emit canonical, hash-stable Preserves artifacts that become regression/traceability evidence; unit correctness belongs to nextest.
- **Traceability**: `molten test traceability scan` against `tests/evidence-matrix.ncl` requires positive + negative coverage or an exemption; `molten test drift compare` compares canonical refs, not rendered logs.
- `mutants.out/` is stale historical cargo-mutants output (incomplete run, old package name) — not part of the QA flow; don't extend or trust it. Same for `.pytest_cache/` and the 0-byte `tests/*.tmp` files.
- Before merging anything behavioral: focused nextest for the touched module, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo octet check`, and (for lifecycle changes) Cairn strict validate. Never claim unrun tests.
