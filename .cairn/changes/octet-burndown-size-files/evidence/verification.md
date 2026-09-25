# Verification: Octet burn-down, source files within the 300-line limit

Base: `52140a0bf` (C3c sync/archive) on `integration/stack-20260925` (`70a9f0b53`). Octet: pinned `octet-toolchain`
`fc38f593`. Private `CARGO_TARGET_DIR`; `nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`. Cairn: the installed
store binary `/nix/store/z1a7pddlrs9myzyypm10v8ljd0ipfkhd-cairn-0.1.0/bin/cairn`, run with the explicit
`cairn-policy/generated/cairn-policy.json` policy.

## Octet

| Scope | Base findings | After | `excessive_file_length` findings (sites) |
|---|---:|---:|---:|
| workspace (`cargo octet check`) | 2499 | 1948 | 548 (305) → 0 |
| `-p molten --lib` | 1046 | 817 | 229 (229) → 0 |

Per-lint delta (`octet-lint-diff.txt`):
- `excessive_file_length`: 548 → 0 workspace, 229 → 0 lib.
- `path_segment_repetition`: 1886 → 1886 workspace findings and 799 → 799 lib findings. The multiset of
  (item, word, segment) findings is identical.
  - Workspace sites go 1042 → 1038. The four missing sites are the duplicate reports of `src/test/support.rs` items
    through its symlinked alias `tests/src/test/support.rs`. Those items now live in `src/test/parts/support/` and are
    reported once.
- `underscore_in_module_filename`: the 15 workspace sites are unchanged. Findings go 31 → 28 because
  `src/cluster_harness/fabric_transport.rs` is reported 2 times instead of 5.
  - Octet emits this lint once for each workspace file in each crate's source map.
  - [INFERENCE] Dependent crates no longer import spans from that file, because its items now live in `body.rs`
    parts.
- `module_file_count` and `ambient_env` are unchanged.

## Shape of the change

- Files split:
  - 303 Rust files: 200 existing part bodies and 103 plain module files. `src/test/support.rs` is counted once
    for its alias.
  - The result is 526 new part bodies.
- 11 oversize inline test modules became nested `tests/mNNN/` parts.
- 4 oversize inherent `impl` blocks were split into consecutive blocks with the same header. Their expansion diff is
  only the repeated headers and the closing braces.
- `src/lib.rs` hoisted its non-module items into `src/parts/lib/p000/body.rs`, and rustfmt sorted the module
  declarations. The expanded line multiset is identical.
- `expand-comparison.txt` inlines every include in the base tree and the candidate tree for all 164 module hosts.
  Eight hosts differ, all as described:
  - the four `impl` splits
  - the `lib.rs` hoist
  - the `Cargo.lock` run-time read
  - two source-scanning tests (next item)
- The two source-scanning tests now concatenate their target's part bodies with `include_str!`, so they scan the same
  source text as before:
  - `dag_conformance_uses_fabric_ports_without_backend_imports`
  - `public_shell_surface_contains_no_runtime_handle_accessor_or_ambient_fallback`

  Before this fix, each test scanned only the host's include lines, and both failed.
- Tooling references re-pointed:
  - 31 tracey `implementation_path` and `verification_path` records now point at the chunk that carries the marker,
    in both the Nickel source and the generated JSON.
  - 4 evidence-matrix targets point at the chunk that defines the test.
  - 32 scan-list entries that name a split file now also list its new chunks: 20 in ast-grep rule `files:` lists,
    which gained 25 paths, and 12 in flake scan argument lists, which gained 14 paths.
  - Six flake source checks also scan the host's parts directory: five native-host `require_literal` checks and the
    canonical `materialized_output` check. Three `fabric-port-boundaries` adapter entries gained their parts
    directories, and one tracey marker grep was re-pointed.
  - The repository Cairn policy evidence root was added and re-exported. `contract-export-drift-gate` passes.

## Rust gates

- `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace --no-fail-fast`: exit 0, 2091 passed, 0 failed, 0 ignored.
- `cargo test --test fabricboundarycompat`: 5 of 5 passed. `cargo test --lib fabric_execution::`: 10 of 10 passed.
- The diff adds no `allow`. `dylint.toml`, baselines, and quarantine files are unchanged, and no `as` cast is added.

## Byte identity against the base binary (`52140a0bf`)

- `fixture-receipt-comparison.txt`: all 6 harness suites give identical exit codes and report and gate-receipt hashes.
- `simulation-fixture-comparison.txt`:
  - `fabric-simulation run` (100 files) and `shrink` (3 files) are identical.
  - All 482 recursive `--help` pages are identical.

## Flake checks

`flake-checks.txt` lists the 32 touched-surface checks built with `nix build .#checks.x86_64-linux.<name>`. 31 exit
0:
- Cairn policy and traceability: `contract-export-drift-gate` and `requirement-traceability-gate`.
- ast-grep authority checks: `cap-std-store-authority`, `cap-std-test-workspaces`, `node-state-authority`, and
  `materialization-authority`.
- Native host and fabric: `native-system-extension-host-profile`, `native-system-extension-octet-deny-all`,
  `fabric-port-boundaries`, `fabric-execution-profile`, and `fabric-execution-octet-deny-all`.
- World checks: `world-promotion-octet-deny-all`, `world-promotion-dependency-identity`, and
  `world-state-oracle-source`.
- `nominal-reference-domains`, `production-profile-fixtures`, and `molten-node-host`.
- 14 profile checks: `addressable-actor`, `content-store-adapter`, `coordination-delivery`,
  `fabric-cryptographic-identity`, `fabric-membership-placement`, `fabric-observability`, `wasm-component`,
  `wasm-component-performance`, `world-operator`, `world-faults`, `world-state-oracle`, `prolly-map`,
  `dev-function-profiling`, and `release-profile-validation`.

`inherited-tracey-debt` exits 1 at both the base and the candidate. The two builders produce identical output:
`requirements=2791`, `referenced=866`, `uncovered=1925`, and `dangling=18`. The 18 dangling references are
`molten.audit_f01`, `f09`, `f10`, `f12` and `molten.consensus.chaoscontrol_*`. This change does not cause the
failure, and it does not change the check's result.

## Review and lifecycle

The no-spec plan review approves the proposal, design, acceptance, and tasks with no findings. The proposal, design,
and tasks gates pass, and `cairn validate --strict` reports no issues.
