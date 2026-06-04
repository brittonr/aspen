## Phase 1: CLI surface

- [x] [serial] r[molten.remote_dataspace_harness_cli.remote_subcommand] Add `molten test remote` subcommands under the Clap CLI.
- [x] [serial] r[molten.remote_dataspace_harness_cli.envelope_build] Add `remote envelope build` for canonical remote dataspace envelopes.
- [x] [parallel] r[molten.remote_dataspace_harness_cli.publish_deliver_local] Add `remote publish-local` and `remote deliver-local` for deterministic Iroh-shaped transport.
- [x] [parallel] r[molten.remote_dataspace_harness_cli.run_two_peer] Add `remote run-two-peer` that emits delivery log, admission receipt, gate receipt, and summary artifacts.

## Phase 2: Gate and examples

- [x] [serial] r[molten.remote_dataspace_harness_cli.gate_command] Add `remote gate` for remote dataspace delivery-log/admission/turn-context gate receipts.
- [x] [parallel] r[molten.remote_dataspace_harness_cli.example_fixture] Add `examples/remote-service-ready.preserves`.
- [x] [parallel] r[molten.remote_dataspace_harness_cli.report_show] Extend report/show or command output so remote dataspace gate receipts are operator-readable.

## Phase 3: Tests and docs

- [x] [serial] r[molten.remote_dataspace_harness_cli.cli_lifecycle_test] Add a CLI lifecycle test covering two-peer run and gate receipt parsing.
- [x] [parallel] r[molten.remote_dataspace_harness_cli.docs] Document the remote CLI workflow in README or architecture docs.
- [x] [parallel] r[molten.remote_dataspace_harness_cli.fail_closed] Ensure missing/non-replayable delivery logs or missing admission receipts fail closed.
