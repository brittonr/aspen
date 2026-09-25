# Change-local acceptance

a[deterministic-fabric-execution-stdin-tests.no-unread-input] No `fabric_execution` live test passes input to a child that does not read it, and the composition child reads its input without a `PATH` lookup.
a[deterministic-fabric-execution-stdin-tests.subjects-kept] The exit-code rejection, publication-failure receipt, descendant teardown, timeout, cancellation, and composition-shape assertions are unchanged.
a[deterministic-fabric-execution-stdin-tests.conservative-pin] A child that exits without reading 256 KiB of input is reported as `UnknownAfterStart`, with no process observation, receipt, or publication, and reconciles as `UnknownRequiresReconciliation`.
a[deterministic-fabric-execution-stdin-tests.repeatable] The `fabric_execution::` tests pass 200 consecutive runs, and the Nix nextest check passes.
