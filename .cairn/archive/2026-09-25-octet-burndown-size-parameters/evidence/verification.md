# Verification: Octet burn-down, explicit input structs for long parameter lists

Base: `cec959753`, which is `integration/stack-20260925` (`70a9f0b53`) plus C4c2 (`de83a2612`/`83c91c78d`) and
the README `default-run` fix (`15aa6dfd0`/`cec959753`). Octet: pinned `octet-toolchain` `fc38f593`. Private
`CARGO_TARGET_DIR`; `nice -n 10`, `CARGO_BUILD_JOBS=16`, Nix `--max-jobs 2`. Cairn: the installed store binary
`/nix/store/z1a7pddlrs9myzyypm10v8ljd0ipfkhd-cairn-0.1.0/bin/cairn`, run with the explicit
`cairn-policy/generated/cairn-policy.json` policy.

## Octet

| Scope | Base findings | After | `too_many_parameters` findings (sites) |
|---|---:|---:|---:|
| workspace (`cargo octet check`) | 3207 | 3052 | 143 (72) → 0 |
| `-p molten --lib` | 1363 | 1287 | 70 (70) → 0 |

`octet-lint-diff.txt` shows that no family grew. Others fell as a side effect:
- `borrowed_argument_types`: 368 → 360.
- `function_length`: 204 → 202.
- `excessive_file_length`: 550 → 548.

The first candidate run added some sites, which were fixed:
- Five `path_segment_repetition` sites came from new struct names that repeated a module segment. Renamed to
  `RequestProgress`, `AdapterDelivery`, `EventHeader`, `RecoveredState`, and `AdmissionSet`.
- Two `excessive_file_length` sites came from `content_store_adapter/local.rs` and
  `wasm/component/evidence/materialization.rs` going past 300 lines. The status builders moved unchanged to
  `integration.rs`, and two bodies use field access instead of a destructure.
- One `function_length` site was an observability test. It now uses a `shell_state` helper.

The final run shows no new site in any family, checked per file for `path_segment_repetition`,
`excessive_file_length`, `borrowed_argument_types`, and `function_length`.

## Shape of the change

- 72 functions: the trailing related parameters become one named struct, and the owner, context, and output-sink
  parameters stay positional.
- The body destructures the struct first, so the body logic is unchanged.
- Repeated groups share one struct: `DependencyEdgeInput`, `RunArtifacts`, `ActionPorts`, `ReplicaAdapterSet` (with
  the `ConcreteReplicaAdapterSet` alias), `AdapterDelivery`, `EventHeader`, and `FailureEvidence` (with
  `FailureEvidence::NONE`).
- No function, method, or type is renamed. Public functions change only their parameter shape, and every in-tree
  caller is migrated.
- One unused parameter (`duplicate_or_conflict_decision`'s `_input`) is removed.

## Rust gates

- `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace --no-fail-fast`: exit 0, 2091 passed, 0 failed, 0 ignored.
- `cargo test --test fabricboundarycompat`: 5 of 5 passed. `cargo test --lib fabric_execution::`: 10 of 10 passed.
- The diff adds no `allow`. `dylint.toml`, baselines, and quarantine files are unchanged, and no `as` cast is added.

## Byte identity against the base binary (`cec959753`)

- `fixture-receipt-comparison.txt`: all 6 `examples/*.preserves` harness suites give identical exit codes and report
  and gate-receipt BLAKE3 hashes.
- `simulation-fixture-comparison.txt`: `molten fabric-simulation run` (100 files) and `shrink` (3) are identical.

## Flake checks

`flake-checks.txt` lists the touched-surface checks built with `nix build .#checks.x86_64-linux.<name>`. All 15 exit 0:
- The `*-octet-deny-all` checks for addressable-actor, coordination-delivery, fabric-execution,
  native-system-extension, and world-head.
- The `*-profile` checks for addressable-actor, coordination-delivery, fabric-execution, content-store-adapter,
  fabric-observability, wasm-component, native-system-extension-host, and fabric-cryptographic-identity.
- `fabric-port-boundaries` and `requirement-traceability-gate`.

## Review and lifecycle

The no-spec plan review approves the proposal, design, acceptance, and tasks with no findings. The proposal, design,
and tasks gates pass, and `cairn validate --strict` reports no issues.
