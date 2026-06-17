## Why

Molten's Octet gate currently passes in a configuration-clean state, but the documented caveat remains: several broad lint families are disabled in `dylint.toml` while source-remediated-zero work is still outstanding. The largest visible hotspot is the monolithic CLI shell in `src/main.rs`, where command parsing and dispatch for many domains live in one file.

Strict release evidence should eventually distinguish a true source-shaped zero from a configuration-clean pass. This change starts the source-remediated-zero burn-down by extracting self-contained CLI command groups into focused shell modules, preserving canonical receipt behavior while reducing the main CLI surface.

## What Changes

- Move Octet command enums and dispatch from `src/main.rs` into a focused `src/cli/ops/octet.rs` module.
- Move Retention command enums and dispatch from `src/main.rs` into a focused `src/cli/workflow/retention.rs` module as part of the disabled-lint-family burn-down.
- Move Delivery command enums and dispatch from `src/main.rs` into a focused `src/cli/workflow/delivery.rs` module as a CLI shell burn-down step.
- Move Provenance command enums and dispatch from `src/main.rs` into a focused `src/cli/workflow/provenance.rs` module as a CLI shell burn-down step.
- Move Protocol command enums and dispatch from `src/main.rs` into a focused `src/cli/workflow/protocol.rs` module as a CLI shell burn-down step.
- Move Job command enums and dispatch from `src/main.rs` into a focused `src/cli/workflow/job.rs` module as the next CLI shell burn-down step.
- Move Secrets and Plugin command enums and dispatch from `src/main.rs` into focused `src/cli/runtime/secrets.rs` and `src/cli/ops/plugin.rs` modules as a small low-risk follow-up split.
- Move Node command enums and dispatch from `src/main.rs` into a focused `src/cli/ops/node.rs` module as the next top-level CLI shell hotspot split.
- Move Repro command enums and dispatch from `src/main.rs` into a focused `src/cli/runtime/repro.rs` module as the next test CLI shell hotspot split.
- Move Catalog command enums and dispatch from `src/main.rs` into a focused `src/cli/core/catalog.rs` module as the next catalog/MCP CLI shell hotspot split.
- Move Chunk command enums and dispatch from `src/main.rs` into a focused `src/cli/core/chunk.rs` module as the next chunk-store CLI shell hotspot split.
- Move Cache command enums and dispatch from `src/main.rs` into a focused `src/cli/core/cache.rs` module as the next eval-cache CLI shell hotspot split.
- Move Artifact command enums and dispatch from `src/main.rs` into a focused `src/cli/core/artifact.rs` module as the next artifact-registry CLI shell hotspot split.
- Move Storage command enums and dispatch from `src/main.rs` into a focused `src/cli/core/storage.rs` module as the next typed-storage CLI shell hotspot split.
- Move Schema command enums and dispatch from `src/main.rs` into a focused `src/cli/core/schema.rs` module as the next schema-identity CLI shell hotspot split.
- Move Upgrade command enums and dispatch from `src/main.rs` into a focused `src/cli/runtime/upgrade.rs` module as the next upgrade-session CLI shell hotspot split.
- Move Transcript command enums and dispatch from `src/main.rs` into a focused `src/cli/core/transcript.rs` module as the next transcript/replay CLI shell hotspot split.
- Move Rewrite command enums and dispatch from `src/main.rs` into a focused `src/cli/runtime/rewrite.rs` module as the next structured-rewrite CLI shell hotspot split.
- Move Remote command enums and dispatch from `src/main.rs` into a focused `src/cli/workflow/remote.rs` module as the next remote-dataspace CLI shell hotspot split.
- Move Ledger and Chain command enums and dispatch from `src/main.rs` into a focused `src/cli/ops/ledger.rs` module as the next evidence-ledger CLI shell hotspot split.
- Move Receipts command enums and dispatch from `src/main.rs` into a focused `src/cli/evidence/receipts.rs` module as the next operator-receipt CLI shell hotspot split.
- Move Service command enums and dispatch from `src/main.rs` into a focused `src/cli/runtime/service.rs` module as the next service-runtime CLI shell hotspot split.
- Move Vat command enums and dispatch from `src/main.rs` into a focused `src/cli/runtime/vat.rs` module as the next runtime-vat CLI shell hotspot split.
- Move Coordination command enums and dispatch from `src/main.rs` into a focused `src/cli/workflow/coordination.rs` module as the next coordination CLI shell hotspot split.
- Move Dogfood command enums and dispatch from `src/main.rs` into a focused `src/cli/ops/dogfood.rs` module as the next operator-dogfood CLI shell hotspot split.
- Move Raft command enums and dispatch from `src/main.rs` into a focused `src/cli/runtime/raft.rs` module as the next control-plane CLI shell hotspot split.
- Move replay-fixture command enums and dispatch from `src/main.rs` into a focused `src/cli/test/replayfixture.rs` module as the next deterministic-replay CLI shell hotspot split.
- Move Report command enums and dispatch from `src/main.rs` into a focused `src/cli/evidence/report.rs` module as the next report validation/show CLI shell hotspot split.
- Move Gate command enums and dispatch from `src/main.rs` into a focused `src/cli/evidence/gate.rs` module as the next gate-check CLI shell hotspot split.
- Move test Receipt command enums and dispatch from `src/main.rs` into `src/cli/evidence/receipts.rs` alongside the top-level receipt operator shell as the next signed-receipt CLI hotspot split.
- Move test Run/Replay command handling, harness failure receipt emission, and report output IO from `src/main.rs` into `src/cli/test/harness.rs` as the next harness CLI shell hotspot split.
- Relocate the CLI shell corpus under bounded `src/cli/{core,workflow,ops,runtime,evidence,test}/` groups with path-mapped module declarations to narrow root module-file-count and underscore-filename pressure without changing command semantics.
- Relocate underscore-named source modules from root, harness, runtime, and test paths into directory/file layouts such as `src/node/runtime.rs`, `src/octet/gate.rs`, `src/job/dag.rs`, `src/harness/steel/executor.rs`, and `tests/cliharness.rs`, preserving public Rust module names through explicit path mapping and removing `underscore_in_module_filename` from the disabled lint set once the probe reaches zero.
- Relocate the remaining broad flat modules into directory `mod.rs` layouts such as `src/artifacts/mod.rs`, `src/ledger/mod.rs`, `src/resources/mod.rs`, and `src/runtime/envelope/mod.rs` so Molten source no longer contributes `module_file_count` findings in the no-disabled probe; the residual `module_file_count` entries are external registry/rustlib paths that need Octet/config/tooling support before the disabled family can be removed.
- Split low-risk file-length hotspots by moving runtime envelope tests, NixOS VM tests, the coordination bounded batch helper, and rewrite CLI input structs into child files while preserving module names and command behavior; the no-disabled probe had `excessive_file_length` down to 129.
- Split the Octet baseline CLI shell and provenance CLI bounded input helpers into child files while preserving command syntax and receipt behavior; the no-disabled probe now has `excessive_file_length` down to 125 and `path_segment_repetition` down to 3032.
- Preserve existing `molten test octet ...`, `molten test retention ...`, `molten test delivery ...`, `molten test provenance ...`, `molten test protocol ...`, `molten test job ...`, `molten test secrets ...`, `molten test plugin ...`, `molten test repro ...`, `molten test catalog ...`, `molten test chunk ...`, `molten test cache ...`, `molten test artifact ...`, `molten test storage ...`, `molten test schema ...`, `molten test upgrade ...`, `molten test transcript ...`, `molten test rewrite ...`, `molten test remote ...`, `molten test ledger ...`, `molten test chain ...`, `molten test service ...`, `molten test vat ...`, `molten test coordination ...`, `molten test raft ...`, `molten test replay-fixture ...`, `molten test report ...`, `molten test gate ...`, `molten test receipt ...`, `molten test run ...`, `molten test replay ...`, `molten dogfood ...`, `molten receipts ...`, and `molten node ...` command syntax, receipt output, denial behavior, and canonical Preserves values.
- Track the remaining disabled lint family burn-down as explicit future work rather than claiming the full source-remediated-zero state is complete.
- Require focused validation and refreshed Octet evidence before claiming source-gate improvements.

## Impact

This is a low-risk incremental slice in the source-remediated-zero roadmap. It reduces the monolithic imperative CLI shell and root module fan-out without changing runtime semantics or release evidence contracts. Future slices should continue splitting long files/functions and broad import/path-repetition hotspots, then remove or narrow the remaining disabled lint family caveats when the source shape and Octet/tooling support allow it.
