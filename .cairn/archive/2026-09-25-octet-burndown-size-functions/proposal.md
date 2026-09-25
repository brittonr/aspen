# Proposal: Octet burn-down, functions within the 70-line limit

## Why

`function_length` is the third C3 size-shape slice. The base is C3b (`8fbab9887`), which has 2692 workspace findings
and 1111 lib findings. `function_length` accounts for 202 of the workspace findings, at 127 functions, 3436 lines over
the 70-line limit in total. It accounts for 70 of the lib findings. The family must reach zero by repair, with no
renames of public items.

## What Changes

- Long functions are split into named, documented helpers. Each helper keeps the original call order and side-effect
  order.
- Repeated struct literals collapse into shared defaults with struct-update syntax (`..base`).
- Parallel statements become tables or loops.
- Early-exit chains use small `ControlFlow` or `Result` helpers.
- Several CLI enum variants with long inline field lists move into `#[derive(clap::Args)]` structs. Flag names, help
  text, and parse behavior do not change. The affected commands are traceability `VerificationRun`, `CiRunReceipt`,
  and `ContextProfile`, and every nixosvm command except `Show`.
- Long `#[test]` functions share assertion closures and fixture builders.
- Three files would otherwise cross the 300-line file limit:
  - The concrete-port assembly test moves from `raft/iroh/tests.rs` into a sibling `raft/iroh/bundle.rs` test module.
  - One chunk-store helper moves from `p002` into the `p003` include body of the same module.
  - The optimization-cap comparison moves into `wasm::performance::optimization`.
- The C3a implementation-traceability markers and `flux_profiler` attributes that had landed on the new input structs are moved back
  onto `execute_live_iroh_stream_get` and `execute_snapshot_export`.

## Impact

- **Files**: 108 changed files (106 under `src/` and 2 under `tests/`), plus one new test module.
- **Public API**: no public item is renamed or changes signature. Some public CLI argument enums now wrap
  `clap::Args` structs, and the command line they parse is unchanged.
- **Testing**:
  - Pinned Octet root and lib runs.
  - fmt, and clippy `-D warnings`.
  - Full workspace tests, including `fabricboundarycompat` and the `fabric_execution::` tests.
  - Harness and fabric-simulation byte identity against the base binary.
  - Touched-surface flake checks.

## Out of Scope

- No renames. `excessive_file_length` is the next slice (C3d).
- Five pre-existing public names are revealed to `path_segment_repetition` once their bodies stop containing an Octet
  compatibility keyword. They are C2 inputs and are not renamed here.
