## Why

Three foundational verification mechanisms in Aspen fail open or count non-evidence as proof:

1. **Broken proof-composition verifier (`src/testing/traceability/parts/p005/body.rs:66-160`):** `layered_proof_diagnostics` checks decision syntax without checking denial propagation from child to parent, allowing a passing parent layer to bind **denied** child layers. `layer_cycles` uses a single `seen` set without an active recursion stack, misclassifying valid acyclic DAG diamonds as cycles. `coverage_from_aggregate_proof` extracts obligation coverage without checking `manifest.decision`.
2. **Porous Tracey requirement coverage (`tools/tracey/inherited_debt_guard.rs:20-23`, `tools/tracey/*_proof.rs`):** Out of 3,135 accepted requirements, 1,930 (61.6%) remain in `inherited-debt-baseline.txt`, and only 677 (21.6%) carry both `impl` and `verify` traceability markers in Rust code/tests. The scanner counts `impl`, `verify`, `depends`, and `related` identically across `docs/`, `tools/`, `scripts/`, `pilots/`, and `nix/` (`flake.nix` shell comments). Consequently, 164 requirements appear "covered" purely through non-code files and 56 solely through 22 uncompiled comment-only `tools/tracey/*_proof.rs` stubs—including `state_machine_proof_trace_contract.rs`, which cites a deleted file (`src/testing/proof_trace.rs`).
3. **Tamper-blind static authority audit (`src/audit/parts/ast_grep/p000/body.rs:210-244`, `p001/body.rs:57-141`):** `rule_bundle_hash` hashes formatted profile metadata struct fields instead of actual YAML rule bytes, and `scan_scope_hash` hashes sorted surface ID strings instead of scanned source files. Meanwhile `required_surfaces()` omits `node-state-adapters` (while embedding a prose literal `"selected converted unit-test helper pages"`), `inventory_rules()` omits the active node-state blocking rules, and receipt checking hardcodes `passed: true`.

## What Changes

- Repair `layered_proof_diagnostics`, `layer_cycles`, and `coverage_from_aggregate_proof` in `src/testing/traceability/parts/p005/body.rs` so denied children propagate denial, acyclic DAG diamonds pass while real cycles fail, and coverage extraction fails closed on denied aggregate manifests.
- Harden `tools/tracey/inherited_debt_guard.rs` to count code/test coverage only from compiled code/test roots (`src/`, `crates/`, `tests/`), require distinct `impl` and `verify` coverage instead of equating prose `related`/`depends` markers, delete the 22 uncompiled comment-only `tools/tracey/*_proof.rs` stub files (moving any genuinely uncovered requirements honestly into tracked debt or adding real tests), and reject broken file citations.
- Repair `src/audit/parts/ast_grep/` so `rule_bundle_hash` hashes actual YAML rule file bytes, `scan_scope_hash` hashes the exact scanned source file inventory/digests, the default Rust profile includes `node-state-adapters` and all active node-state blocking rules, and `passed` is derived from diagnostics rather than hardcoded.

## Impact

- **Files**: `src/testing/traceability/parts/p005/body.rs`, `src/testing/traceability/parts/tests/`, `tools/tracey/inherited_debt_guard.rs`, `tools/tracey/*_proof.rs`, `evidence/tracey/inherited-debt-baseline.txt`, `src/audit/parts/ast_grep/p000/body.rs`, `src/audit/parts/ast_grep/p001/body.rs`, and corresponding companion docs in `docs/technical/`.
- **Verification**: Unit and negative tests for denied child propagation, diamond DAG vs cycle detection, denied aggregate coverage extraction, YAML-byte rule hashing, source-byte scope hashing, and Tracey code-only `impl`+`verify` enforcement.
- **Non-claims**: Repairing verifiers and scanners makes local structural/proof checks truthful; it does not eliminate the 1,930 inherited requirement gaps by itself or certify runtime production readiness.

## Out of Scope

- Burning down all 1,930 inherited requirement debt entries in one change (debt re-baselining after removing fake stubs is explicit; substantive feature implementation belongs to domain changes).
- CI wiring of `molten-core` tests and Octet (tracked in `close-ci-verification-blind-spots`).
