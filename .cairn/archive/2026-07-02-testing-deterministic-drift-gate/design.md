## Context

The harness already treats determinism and replay as core invariants, and dogfood evidence includes replay verification. The remaining gap is an operator-visible drift check that repeats a workflow from fresh state and compares the canonical evidence it produces. This catches nondeterminism that still produces individually valid receipts but changes refs between equivalent runs.

## Decisions

### 1. Compare canonical refs, not rendered text

**Choice:** The drift comparator operates on canonical receipt/report refs and normalized canonical evidence values. Rendered logs, summaries, and command output are diagnostic-only.

**Rationale:** Deterministic evidence lives in canonical Preserves/content refs. Text output can be useful for debugging but must not define equality.

### 2. Keep workflow execution in the shell and comparison in the core

**Choice:** The imperative shell runs selected commands in fresh isolated state roots and collects evidence refs. A pure comparator accepts two evidence summaries plus allowed-variance declarations and returns pass/deny diagnostics.

**Rationale:** This keeps ambient filesystem/process behavior out of the core and makes injected drift fixtures cheap to test.

### 3. Require explicit variance declarations

**Choice:** Any accepted volatile field must be declared by path or semantic field name, include a reason class, and be excluded through canonical normalization before comparison. Undeclared drift fails closed.

**Rationale:** Determinism exceptions are easy to abuse. Naming the variance and reason preserves reviewability while allowing legitimate runtime-specific facts such as realized temporary paths when they are not part of semantic evidence.

## Risks / Trade-offs

- Re-running workflows increases validation time; start with focused dogfood/repro/release evidence commands and keep the VM extension explicit.
- Some current receipts may include legitimate store or temp refs; normalize only fields that are documented and non-semantic.
- Drift gates should not hide failures behind retries. Repetition is for comparison, not flake masking.
