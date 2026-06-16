## Phase 1: First CLI shell split

- [x] [serial] r[molten.octet_source_remediated_zero.cli_octet_shell_split] Extract Octet command parsing and dispatch from `src/main.rs` into `src/cli/ops/octet.rs`.
- [x] [serial] r[molten.octet_source_remediated_zero.no_cli_semantic_drift] Run focused Rust validation after the Octet CLI split to confirm command parsing and dispatch still compile.

## Phase 2: Evidence refresh

- [x] [serial] r[molten.octet_source_remediated_zero.evidence_refresh] Refresh workspace/lib Octet artifacts, focused object corpus, strict gate, remediation plan, and release dogfood evidence after the source scope is ready for a checkpoint.

## Phase 3: Disabled lint family burn-down

- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Retention command parsing and dispatch from `src/main.rs` into `src/cli/workflow/retention.rs` as the next CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Delivery command parsing and dispatch from `src/main.rs` into `src/cli/workflow/delivery.rs` as a smaller follow-up CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Provenance command parsing and dispatch from `src/main.rs` into `src/cli/workflow/provenance.rs` as another focused CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Protocol command parsing and dispatch from `src/main.rs` into `src/cli/workflow/protocol.rs` as the next focused CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Job command parsing and dispatch from `src/main.rs` into `src/cli/workflow/job.rs` as the next CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Secrets and Plugin command parsing and dispatch from `src/main.rs` into `src/cli/runtime/secrets.rs` and `src/cli/ops/plugin.rs` as a low-risk CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Node command parsing and dispatch from `src/main.rs` into `src/cli/ops/node.rs` as the next top-level CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Repro command parsing and dispatch from `src/main.rs` into `src/cli/runtime/repro.rs` as the next test CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Catalog command parsing and dispatch from `src/main.rs` into `src/cli/core/catalog.rs` as the next catalog/MCP CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Chunk command parsing and dispatch from `src/main.rs` into `src/cli/core/chunk.rs` as the next chunk-store CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Cache command parsing and dispatch from `src/main.rs` into `src/cli/core/cache.rs` as the next eval-cache CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Artifact command parsing and dispatch from `src/main.rs` into `src/cli/core/artifact.rs` as the next artifact-registry CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Storage command parsing and dispatch from `src/main.rs` into `src/cli/core/storage.rs` as the next typed-storage CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Schema command parsing and dispatch from `src/main.rs` into `src/cli/core/schema.rs` as the next schema-identity CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Upgrade command parsing and dispatch from `src/main.rs` into `src/cli/runtime/upgrade.rs` as the next upgrade-session CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Transcript command parsing and dispatch from `src/main.rs` into `src/cli/core/transcript.rs` as the next transcript/replay CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Rewrite command parsing and dispatch from `src/main.rs` into `src/cli/runtime/rewrite.rs` as the next structured-rewrite CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Remote command parsing and dispatch from `src/main.rs` into `src/cli/workflow/remote.rs` as the next remote-dataspace CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Ledger and Chain command parsing and dispatch from `src/main.rs` into `src/cli/ops/ledger.rs` as the next evidence-ledger CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Receipts command parsing and dispatch from `src/main.rs` into `src/cli/evidence/receipts.rs` as the next operator-receipt CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Service command parsing and dispatch from `src/main.rs` into `src/cli/runtime/service.rs` as the next service-runtime CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Vat command parsing and dispatch from `src/main.rs` into `src/cli/runtime/vat.rs` as the next runtime-vat CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Coordination command parsing and dispatch from `src/main.rs` into `src/cli/workflow/coordination.rs` as the next coordination CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Dogfood command parsing and dispatch from `src/main.rs` into `src/cli/ops/dogfood.rs` as the next operator-dogfood CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Raft command parsing and dispatch from `src/main.rs` into `src/cli/runtime/raft.rs` as the next control-plane CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract replay-fixture command parsing and dispatch from `src/main.rs` into `src/cli/test/replayfixture.rs` as the next deterministic-replay CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Report command parsing and dispatch from `src/main.rs` into `src/cli/evidence/report.rs` as the next report validation/show CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Gate command parsing and dispatch from `src/main.rs` into `src/cli/evidence/gate.rs` as the next gate-check CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract test Receipt command parsing and dispatch from `src/main.rs` into `src/cli/evidence/receipts.rs` as the next signed-receipt CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract harness Run/Replay command handling and failure receipt IO from `src/main.rs` into `src/cli/test/harness.rs` as the next harness CLI shell hotspot split.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Relocate Gate, Harness, Receipts, replay-fixture, and Report CLI shells under `src/cli/` to narrow root module-count and underscore-filename pressure while preserving module names and command semantics.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Relocate the remaining CLI shell modules under bounded `src/cli/{core,workflow,ops,runtime,evidence,test}/` groups to further narrow root module-count and underscore-filename pressure without changing command semantics.
- [ ] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Continue splitting CLI/module hotspots and remove or narrow disabled lint families when source-remediated-zero evidence can replace the current configuration-clean caveat.
