# Tasks: Retention GC plan dry-run UX

- [x] [serial] r[molten.retention.gc_plan_dry_run_ux] Add canonical retention GC plan evidence, parsing, storage, and summaries that bind candidate, index, destructive evidence inputs, gates, and diagnostics.
- [x] [serial] r[molten.retention.gc_plan_dry_run_ux] Add `molten test retention gc-plan` CLI coverage using explicit destructive retention evidence flags.
- [x] [serial] r[molten.retention.gc_plan_dry_run_ux] Test passing and denied plans, prove planning does not write tombstones or retention receipts, and validate with Cairn/local/Nix checks before archive.
