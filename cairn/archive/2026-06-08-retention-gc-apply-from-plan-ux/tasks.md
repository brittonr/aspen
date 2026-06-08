# Tasks: Retention GC Apply From Plan UX

- [x] [serial] r[molten.retention.gc_apply_from_plan_ux] Add canonical `retention-gc-apply-v1` receipts, parsing, storage, summaries, and ledger artifact classification.
- [x] [serial] r[molten.retention.gc_apply_from_plan_ux] Add `molten test retention gc-apply-plan --plan-ref` CLI support that recomputes the stored plan before mutation.
- [x] [serial] r[molten.retention.gc_apply_from_plan_ux] Test passing apply, drift denial without tombstones, denied-plan behavior, CLI receipt output, and validate with Cairn/local/Nix checks before archive.
