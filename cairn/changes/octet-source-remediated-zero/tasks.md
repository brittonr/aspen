## Phase 1: First CLI shell split

- [x] [serial] r[molten.octet_source_remediated_zero.cli_octet_shell_split] Extract Octet command parsing and dispatch from `src/main.rs` into `src/cli_octet.rs`.
- [x] [serial] r[molten.octet_source_remediated_zero.no_cli_semantic_drift] Run focused Rust validation after the Octet CLI split to confirm command parsing and dispatch still compile.

## Phase 2: Evidence refresh

- [x] [serial] r[molten.octet_source_remediated_zero.evidence_refresh] Refresh workspace/lib Octet artifacts, focused object corpus, strict gate, remediation plan, and release dogfood evidence after the source scope is ready for a checkpoint.

## Phase 3: Disabled lint family burn-down

- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Retention command parsing and dispatch from `src/main.rs` into `src/cli_retention.rs` as the next CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Delivery command parsing and dispatch from `src/main.rs` into `src/cli_delivery.rs` as a smaller follow-up CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Provenance command parsing and dispatch from `src/main.rs` into `src/cli_provenance.rs` as another focused CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Protocol command parsing and dispatch from `src/main.rs` into `src/cli_protocol.rs` as the next focused CLI shell burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Job command parsing and dispatch from `src/main.rs` into `src/cli_job.rs` as the next CLI shell hotspot burn-down slice.
- [x] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Extract Secrets and Plugin command parsing and dispatch from `src/main.rs` into `src/cli_secrets.rs` and `src/cli_plugin.rs` as a low-risk CLI shell burn-down slice.
- [ ] [serial] r[molten.octet_source_remediated_zero.disabled_lint_burndown] Continue splitting CLI/module hotspots and remove or narrow disabled lint families when source-remediated-zero evidence can replace the current configuration-clean caveat.
