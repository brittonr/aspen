# Proposal: Make the fabric_execution stdin tests deterministic

## Why

Three `fabric_execution` tests pass `INPUT_BYTES` to shell children that never read standard input. If the child exits
before the pinned `bounded-exec` revision (`29dac88ecded94457572db3fdfaaaab95fa91525`) writes those bytes, the stdin
writer gets `EPIPE`. `bounded-exec` suppresses that error only for timeout and cancellation, so a normal exit becomes
`RunError::Io { operation: WriteStdin }`. `LiveExecutionAdapter::record_run_failure` then maps it to
`UnknownAfterStart`, and the test fails. Observed rates were 1 failure in 40 local runs, plus several Nix nextest failures.
The CI profiles use `retries = 0`, so every such failure fails the gate.

Exposed tests:

- `live_adapter_bounds_output_and_preserves_publication_failure_receipt` (`printf` only);
- `live_adapter_preserves_rejected_exit_and_descendant_teardown` (`exit N` and a backgrounded loop plus `printf`);
- `live_and_simulation_compositions_share_command_and_outcome_shape` (live half used `cat >/dev/null`. Under the cleared
  environment, `cat` cannot be resolved in the Nix sandbox, so the child never read its input).

## What Changes

- Children that do not read stdin receive no input: the request carries no `stdin_ref`, and the resolved context
  carries no bytes. This covers the timeout and cancellation children as well.
- The composition test's child reads its input with the shell builtin `read`, so it needs no program lookup under the
  cleared environment, and it prints `bounded:<input>`. Live and simulation still compare the same output.
- Add one deterministic test that pins today's conservative behaviour. A child exits `0` without reading 256 KiB of
  input. That is more than the default Linux pipe capacity, so the writer must hit `EPIPE`. The adapter reports
  `UnknownAfterStart`, with no process observation, no receipt, no publication, and
  `UnknownRequiresReconciliation`.

Each test keeps its subject: exit-code rejection, publication-failure receipt, descendant teardown, and composition shape.

## Impact

- **Files**: `src/fabric_execution/tests.rs`, `src/fabric_execution/tests/live.rs`,
  `src/fabric_execution/tests/simulation.rs`.
- **Testing**: the `fabric_execution::` tests 200 consecutive times, `nix build .#checks.x86_64-linux.nextest`,
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.

## Out of Scope

- Adapter classification does not change. A follow-up that adopts a `bounded-exec` revision with an explicit
  input-delivery observation (`ClosedByChild`) will route that case through the normal publication path. That follow-up
  will revise the pinning test added here. It needs a published `bounded-exec` revision and a dependency pin bump.
