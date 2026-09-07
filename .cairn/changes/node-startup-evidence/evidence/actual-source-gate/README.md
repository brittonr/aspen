# Actual pinned source gate: integration failure

## Input and invocation

- Molten source commit: `6667e5616b978f7bb6f64201b76d452634815612`.
- Frozen Git archive BLAKE3: `37aa20df6db6303ea6fccaf9a39a62b90594d933c35c7de90b542b244bc92591`.
- Startup Octet pin: `c9b06bcf565c51d4a77d210e61b69ae51db9df25`.
- Exact command: `cargo octet check --artifact-dir target/octet`.
- Workspace metadata: `-p molten -p molten-node-host`, with `--all-targets`.
- Existing March-21 compiler and driver; previously built fixed CLI and library. No tool rebuild.
- Isolated environment, fresh source and target, explicit offline dependency caches without ambient Cargo configuration, two jobs.
- Unchanged pinned deny-all hook. No baseline, suppression, severity reduction, or scope change.

The complete tracked source was extracted from the commit before execution.
The source archive, tree listing, revision, raw logs, and compiler identity remain private.
Tool/archive identities matched before and after execution. The final tracked-source diff was empty.
This establishes retained inputs for a failed attempt, not complete successful lint coverage or compiler correctness.

## Observed result

Pueue task 10697, systemd invocation `11e6e16318fc4d1dbb250d46c598da55`:

```text
Actual pinned source gate exited 2.
Main processes terminated with: code=exited, status=2/INVALIDARGUMENT
Service runtime: 3min 42.357s
Memory peak: 2.8G
```

The runtime limit was 5400 seconds, with control-group termination, a 12 GiB memory limit, and 512 tasks.
The run ended by failure, not timeout or memory exhaustion.
Octet reported `integration-failure`: 69 findings, comprising 68 errors and one warning. Cargo exited 101.

Representative raw diagnostics:

```text
Checking molten-core v0.1.0
error[E0602]: unknown lint: `compound_assertion`
  = note: requested on the command line with `-D compound_assertion`
  = note: requested on the command line with `-D unknown-lints`
error[E0602]: unknown lint: `no_todo`
error: could not compile `molten-core` (lib) due to 68 previous errors
```

These are lint-registration errors, not 68 established Molten style violations.
The retained status is not clean and cannot support startup admission.

## Bounded diagnosis

At the pinned Octet source, `cargo-octet/src/engine.rs` sets `RUSTC_WORKSPACE_WRAPPER` and `DYLINT_NO_DEPS=1`.
The inspected driver source under `tests/support/dylint/driver/src/lib.rs` skips registration when `CARGO_PRIMARY_PACKAGE` is absent.
Its argument construction still adds `DYLINT_RUSTFLAGS`.
The unchanged hook supplies deny flags for every Octet lint, including unknown-lint denial.

This source path explains a plausible workspace-dependency interaction for `molten-core`, which is outside the explicit primary-package selection.
The tiny earlier fixture had no such workspace dependency.
A focused reproduction and installed-driver/source association are still needed before claiming a proven upstream fix.
No source, tool, or policy patch was made to hide this failure.

## Scope and next step

The guarded lifecycle wiring passed its focused tests separately. This actual source gate did not pass.
Tasks 4 and 5 remain open. No approved cohort or synthetic clean bundle was created.
The May-26 production build, normal-node VM, native replay, and release promotion did not start.
Stage0, compiler replacement, Mantle rebuild, and Darkhttpd rebuild remain prohibited.

Next: reproduce the workspace-dependency registration interaction with the exact installed CLI, library, and driver.
Do not remove deny flags, change workspace scope, or relabel this attempt as a passing gate.
