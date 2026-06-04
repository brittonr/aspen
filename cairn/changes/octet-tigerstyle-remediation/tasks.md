## Phase 1: Measurement and priority queue

- [x] [serial] r[molten.octet_tigerstyle_remediation.baseline_metrics] Capture current Octet counts by lint/path for full workspace, lib-only, and focused critical paths, with content refs to status/summary/object corpus artifacts.
- [x] [serial] r[molten.octet_tigerstyle_remediation.critical_surface_inventory] Inventory critical source surfaces: harness/gates, admission, job execution, node runtime startup, ledger/evidence, adapter boundaries, redaction/export, and CLI artifact-output paths.
- [x] [serial] r[molten.octet_tigerstyle_remediation.priority_order] Define burn-down priority: critical deny classes first, resource bounds second, high-arity/long functions third, file/module splits fourth, style/autofix last.
- [x] [parallel] r[molten.octet_tigerstyle_remediation.no_suppression_policy] Require every retained warning to have either removal work scheduled or a reviewed quarantine receipt; no hidden suppressions.

## Phase 2: Critical caveat removal

- [x] [serial] r[molten.octet_tigerstyle_remediation.no_panic_unwrap] Remove or review `panic`, `unwrap`, and `expect` findings on critical evidence-bearing paths, replacing them with structured denials or typed errors.
- [x] [serial] r[molten.octet_tigerstyle_remediation.no_ambient_clock] Remove ambient wall-clock/time findings from deterministic evidence paths or isolate them behind explicit shell receipts.
- [x] [serial] r[molten.octet_tigerstyle_remediation.no_unbounded_loops] Add explicit limits/checkpoints for unbounded loops and recursion findings in runtime, harness, job, adapter, and report paths.
- [x] [parallel] r[molten.octet_tigerstyle_remediation.no_sentinel_fallbacks] Replace sentinel fallback patterns in admission/source-gate code with typed option/result handling and denial receipts.

## Phase 3: Resource and API shape remediation

- [x] [serial] r[molten.octet_tigerstyle_remediation.collection_bounds] Add deterministic bounds or validated prior limits for unbounded collection growth findings on critical paths.
- [ ] [serial] r[molten.octet_tigerstyle_remediation.builder_input_structs] Replace high-arity receipt/value builders with input structs that validate typed refs and invariants before rendering canonical Preserves values.
- [ ] [serial] r[molten.octet_tigerstyle_remediation.typed_ref_boundaries] Replace raw strings/generic hashes at public evidence boundaries with typed ref/id/profile structs or parsing functions that fail closed.
- [ ] [parallel] r[molten.octet_tigerstyle_remediation.assertion_density] Add positive and negative assertions/tests around pure helpers introduced by remediation.

## Phase 4: Hotspot module splits

- [ ] [serial] r[molten.octet_tigerstyle_remediation.cli_shell_split] Split `src/main.rs` into thin CLI dispatch modules and pure command input conversion helpers, preserving canonical failure artifact behavior.
- [ ] [serial] r[molten.octet_tigerstyle_remediation.job_dag_split] Split `src/job_dag.rs` into DTO, parse, sync, admission, execution, memo/cache, and test-support modules without changing canonical refs.
- [ ] [serial] r[molten.octet_tigerstyle_remediation.node_runtime_shape] Refactor `src/node_runtime.rs` to use typed inputs, bounded adapter lists, deterministic duplicate-free ordering, and short receipt helpers.
- [x] [parallel] r[molten.octet_tigerstyle_remediation.object_corpus_refresh] Refresh object corpus/fingerprint evidence for changed critical paths and bind refs in Octet gate receipts.

## Phase 5: Burn-down validation

- [ ] [serial] r[molten.octet_tigerstyle_remediation.focused_octet_runs] Re-run focused Octet checks after each slice and record before/after finding deltas.
- [ ] [serial] r[molten.octet_tigerstyle_remediation.strict_profile_dry_run] Run strict Octet gate dry-runs until warning-only status is eliminated or only reviewed noncritical debt remains under quarantine.
- [ ] [serial] r[molten.octet_tigerstyle_remediation.no_regression_tests] Add tests/fixtures proving remediation does not change canonical refs, report receipts, job execution outputs, or node startup evidence except where intentionally versioned.
- [ ] [parallel] r[molten.octet_tigerstyle_remediation.cairn_task_drain] Check off corresponding Octet fail-close/quarantine tasks as code lands and strict gate receipts become available.
