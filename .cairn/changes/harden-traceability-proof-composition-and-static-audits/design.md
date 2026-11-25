# Design

## Layered and Aggregate Proof Verifier Repairs

- In `src/testing/traceability/parts/p005/body.rs`:
  - Update `layered_proof_diagnostics` to look up every referenced `child_id` in the layer map and emit a `denied-child-layer` diagnostic whenever a parent layer claiming `pass` references a child layer whose `decision != "pass"`.
  - Replace the single `seen` set in `layer_cycles` with three-color DFS (`unvisited`, `visiting`/active recursion stack, `visited`) so shared-descendant DAG diamonds are accepted while true cycles produce `layer-cycle`.
  - Update `coverage_from_aggregate_proof` to check `manifest.decision == "pass"` (and zero blocking diagnostics) before returning extracted coverage entries, returning a typed denial error on denied manifests.

## Code-Backed Traceability Guard

- In `tools/tracey/inherited_debt_guard.rs`:
  - Restrict coverage-discharging scans to `src/`, `crates/`, and `tests/` with `.rs` (and compiled contract `.ncl`) sources; scan `docs/`, `tools/`, `scripts/`, `pilots/`, and `nix/` only as non-coverage cross-references that cannot remove a requirement from uncovered debt.
  - Track `impl` and `verify` separately; `depends` and `related` MUST NOT discharge requirement coverage.
  - Delete all 22 uncompiled comment-only `tools/tracey/*_proof.rs` files (including `state_machine_proof_trace_contract.rs` citing nonexistent `src/testing/proof_trace.rs`) and replace them either with genuine `impl`/`verify` annotations on real code/tests or explicit tracked debt entries so coverage counts reflect reality.

## Content-Hashed Static Authority Audit

- In `src/audit/parts/ast_grep/p000/body.rs`:
  - Compute `rule_bundle_hash` over canonical packed Preserves of sorted `(relative_rule_path, blake3(yaml_bytes))` pairs supplied from the actual rule files rather than struct summary metadata alone.
  - Compute `scan_scope_hash` over canonical packed Preserves of sorted `(relative_source_path, blake3(source_bytes))` pairs representing the actual scanned surface rather than surface name strings.
- In `src/audit/parts/ast_grep/p001/body.rs`:
  - Add `node-state-adapters` to `required_surfaces()` and remove the prose literal `"selected converted unit-test helper pages"`.
  - Add the active node-state blocking rules (`node-state-root-reacquisition.yml` and companion node-state blocking rule) to `inventory_rules()` so the default Rust profile matches `tools/ast-grep/runtime-authority/rules/` and `docs/ast-grep-runtime-authority-audits.md`.
  - Derive receipt `passed` directly from `diagnostics.is_empty()` instead of hardcoding `passed: true`.

## Pure Core vs Shell Boundaries

- Proof composition checks, DAG cycle detection, and audit receipt hashing over supplied byte-digest maps remain pure deterministic functions; filesystem reading of YAML rules and scanned Rust files stays in the audit CLI shell.
