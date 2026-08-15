## Phase 1: Measurement and priority queue

- [x] [serial] r[molten.octet_tigerstyle_remediation.baseline_metrics] Capture current Octet counts by lint/path for full workspace, lib-only, and focused critical paths, with content refs to status/summary/object corpus artifacts.
- [x] [serial] r[molten.octet_tigerstyle_remediation.critical_surface_inventory] Inventory critical source surfaces: harness/gates, admission, job execution, node runtime startup, ledger/evidence, adapter boundaries, redaction/export, and CLI artifact-output paths.
- [x] [serial] r[molten.octet_tigerstyle_remediation.priority_order] Define burn-down priority: critical deny classes first, resource bounds second, high-arity/long functions third, file/module splits fourth, style/autofix last.
- [x] [parallel] r[molten.octet_tigerstyle_remediation.no_suppression_policy] Require every retained warning to have either removal work scheduled, a reviewed quarantine receipt, or a documented configuration caveat; no hidden suppressions.

## Phase 2: Critical caveat removal

- [x] [serial] r[molten.octet_tigerstyle_remediation.no_panic_unwrap] Remove, deny, or require review for `panic`, `unwrap`, and `expect` findings on critical evidence-bearing paths.
- [x] [serial] r[molten.octet_tigerstyle_remediation.no_ambient_clock] Remove ambient wall-clock/time findings from deterministic evidence paths or isolate them behind explicit shell receipts.
- [x] [serial] r[molten.octet_tigerstyle_remediation.no_unbounded_loops] Add explicit limits/checkpoints or review requirements for unbounded loops and recursion findings in runtime, harness, job, adapter, and report paths.
- [x] [parallel] r[molten.octet_tigerstyle_remediation.no_sentinel_fallbacks] Replace sentinel fallback patterns in admission/source-gate code with typed option/result handling and denial receipts.

## Phase 3: Resource and API shape remediation

- [x] [serial] r[molten.octet_tigerstyle_remediation.collection_bounds] Add deterministic bounds or validated prior limits for unbounded collection growth findings on critical paths.
- [x] [serial] r[molten.octet_tigerstyle_remediation.builder_input_structs] Land input-struct remediation on recent critical helpers and keep remaining high-arity helper burn-down visible in the remediation plan.
- [x] [serial] r[molten.octet_tigerstyle_remediation.typed_ref_boundaries] Validate source-gate and public evidence refs at runtime/admission boundaries and keep remaining raw-string CLI/config edges visible as future burn-down.
- [x] [parallel] r[molten.octet_tigerstyle_remediation.assertion_density] Add positive and negative assertions/tests around source-gate, baseline, remediation-plan, collection-bound, and receipt-binding helpers.

## Phase 4: Hotspot module splits

- [x] [serial] r[molten.octet_tigerstyle_remediation.cli_shell_split] Record `src/main.rs` shell splitting as future source-remediated-zero work and preserve current configuration-clean caveat rather than claiming the split is complete.
- [x] [serial] r[molten.octet_tigerstyle_remediation.job_dag_split] Record `src/job_dag.rs` splitting as future source-remediated-zero work and require future splits to preserve canonical refs.
- [x] [serial] r[molten.octet_tigerstyle_remediation.node_runtime_shape] Keep node runtime startup denial/source-gate validation receipt-backed and record remaining shape debt as visible remediation evidence.
- [x] [parallel] r[molten.octet_tigerstyle_remediation.object_corpus_refresh] Refresh object corpus/fingerprint evidence for changed critical paths and bind refs in Octet gate receipts.

## Phase 5: Burn-down validation

- [x] [serial] r[molten.octet_tigerstyle_remediation.focused_octet_runs] Re-run focused Octet checks after slices where available and record before/after finding deltas or configuration-clean caveats.
- [x] [serial] r[molten.octet_tigerstyle_remediation.strict_profile_dry_run] Run strict Octet gate dry-runs and record whether the result is source-remediated zero, quarantine, or configuration-clean with disabled-lint caveats.
- [x] [serial] r[molten.octet_tigerstyle_remediation.no_regression_tests] Add or retain tests proving remediation/source-gate logic does not change canonical refs, report receipts, job execution outputs, or node startup evidence except where intentionally versioned.
- [x] [parallel] r[molten.octet_tigerstyle_remediation.cairn_task_drain] Check off corresponding Octet fail-close/quarantine tasks as code, docs, caveats, and strict gate receipts become available.
