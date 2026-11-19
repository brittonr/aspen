## Why

Issue #41 asks for per-profile measurements of the reviewed Wasm optimization
profiles on an executable workload before anything learned is considered. The
first two measurement rounds lived in throwaway worktrees: the numbers were
posted to the issue, but nothing in the repository could reproduce them and no
reviewed source carried the workload definition. That fails the issue's own
"reproducible commands and source IDs" requirement.

The measurement also settled a design question. On a compute-bound workload the
four reviewed profiles are indistinguishable (2.7-10.7% spread, inside host
variance); on a workload that instantiates many short-lived instances per
process, `pooling` is about 3x faster than `baseline` while `cow` and
`instance-pre` are indistinguishable from baseline. The winner is fixed per
workload shape, so the first artifact should be a reproducible measurement, not
a selector.

## What Changes

- Add a diagnostic example, `examples/wasm-optimization-profiles.rs`, that
  compiles and runs two named workloads under the four reviewed profiles and
  prints per-phase `median`/`p90`/`max` samples as JSON. It mirrors
  `molten.wasm.optimization.*.v1` and grants no capability.
- Add a short command entry to `docs/development.md` so the run is discoverable.
- Record one run of both workloads as change evidence, with the commit, binary
  image, sample counts, raw numbers and the decision they support.

Non-goals: no selector, no trained artifact, no change to the reviewed profile
admission path, no runtime effect, no claim about production workloads.

## Impact

- **Files**: `examples/wasm-optimization-profiles.rs`,
  `docs/development.md`, this change's `evidence/`.
- **Testing**: `cargo test --release --example wasm-optimization-profiles` for
  the sample aggregation, the recorded run for both workloads, `cargo fmt
  --check`, `cargo clippy --all-targets -- -D warnings`, Octet, and Cairn
  validation plus the proposal, design and tasks gates.
