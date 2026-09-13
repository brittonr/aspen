# Validation-source repairs

## Scope and owner

Molten maintainers own these narrow repairs to existing validation paths.
The changes preserve process outcome categories and all existing assertions.
They add no allowance, warning budget, feature exclusion, or runtime retry policy.

## Clippy source corrections

The original workspace baseline rejected two ChaosControl source patterns.
`identity.rs` bound an unused fallback value.
`ledger.rs` used a nested condition that Clippy required as a let-chain.

The corrections preserve the rejection branches and evaluation order.
The focused ChaosControl baseline passed 20 tests before these edits.
The workspace Clippy run in `workspace-clippy-final.*` passed with all targets and `-D warnings`.
Later checks remain bound to their own source identities.

## Process fixture correction

The workspace run in `workspace-tests-warm.*` failed one existing process test.
The focused baseline in `process-fixture-baseline.*` repeated that failure.
It passed three tests and failed `live_adapter_preserves_rejected_exit_and_descendant_teardown`.

The rejected-exit child received a nonempty stdin request but exited without reading it.
The adapter reported `UnknownAfterStart` with `WriteStdin failed: Broken pipe (os error 32)`.
The test expected an observed rejected exit instead.
A prior workspace run passed the same test, so that run did not establish deterministic behavior.

Short-lived fixture scripts now read the supplied newline-terminated input before their intended output or exit.
The rejected-exit code, output limits, publication-error assertions, descendant teardown, timeout, cancellation, and pre-start rejection checks remain intact.
The environment test still checks that the adapter clears the inherited environment.
Named constants replace its unexplained exit code and published-stream count.

The focused post-edit run passed all four live tests, exit 0.
`process-fixture-green.*` retains the complete output and direct exit.
This repair changes test orchestration only.
It does not change how the runtime classifies a real broken pipe or an uncertain process outcome.

## Reproduction commands

Run these commands from the repository worktree.
The campaign used two Cargo build jobs and an eight-minute command deadline.
Process and workspace tests used `RUST_TEST_THREADS=2`.
The target directory was `/tmp/molten-completion-20260913-target`.

```console
nix develop --no-write-lock-file -c cargo test --locked -p molten --lib fabric_consistency::chaoscontrol
nix develop --no-write-lock-file -c cargo test --locked -p molten --lib fabric_execution::tests::live
nix develop --no-write-lock-file -c cargo clippy --locked --workspace --all-targets -- -D warnings
nix develop --no-write-lock-file -c cargo test --locked --workspace
```

## Evidence limits

The campaign retains raw transcripts under `.pi/molten-completion/logs/`.
Repository evidence binds the changed source and lossless transcript archives with BLAKE3.
The final workspace Clippy run in `workspace-clippy-process-repaired.*` passed with all targets and `-D warnings`.
The workspace test run in `workspace-tests-repaired.*` reached its eight-minute deadline, exit 124.
Its 1,463 library tests and 74 CLI unit tests passed, along with the preceding integration suites.
The command stopped during `tests/nativesystemextension.rs` without a terminal result for that suite.
Current `cairn validate --root . --strict` passed all 55 changes, exit 0.
Its receipt selects the repository's unchanged generated policy through `legacy_default`.
That structural result does not grant implementation, archive, or release acceptance.
Nextest and strict Octet remain separate obligations.
A passing focused test does not establish sandboxing, whole-program correctness, or release acceptance.
