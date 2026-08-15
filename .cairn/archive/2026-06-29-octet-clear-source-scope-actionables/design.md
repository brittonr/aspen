## Context

The source-scope classifier records deterministic evidence that distinguishes Molten-owned source from generated, remapped, registry, rustlib, and unknown rows. The latest remediation notes identify Molten-owned actionable rows that should be handled before claiming source-scope cleanup.

## Design

### Actionable-row workflow

Start from a fresh source-scope remediation-plan run. For each Molten-owned row, choose the smallest behavior-preserving refactor: split the local shell, rename crate-private modules, or adjust local declarations so the finding is removed without changing public behavior.

### Classification boundary

Generated/remapped external findings remain evidence-only classifications. Unknown findings remain blocked and actionable. This change must not narrow Octet source scope unless the classification receipt explains why every excluded row is external or remapped.

### Documentation

Update the remediation document with the current classification receipt, object inventory reference, actionable-row count, and any remaining blocked rows.

## Validation

Run focused tests for touched dogfood/node authority paths, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, the source-scope remediation-plan command, and a no-disabled Octet probe.

## Non-goals

- Do not remove source-scope caveats for unknown or Molten-owned rows that still warn.
- Do not change node-control authority semantics or dogfood release evidence contracts.
- Do not treat generated/remapped classification as authority, policy, provenance, or release trust.
