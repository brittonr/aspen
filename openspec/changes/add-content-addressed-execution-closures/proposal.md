## Why

Unison shows that distributed execution gets simpler when code and dependencies are content-addressed and moved as a typed closure rather than as ad hoc scripts, logs, and implicit worker state. Aspen already has Iroh transfer, BLAKE3 blob identity, Snix artifacts, jobs, runtime-host workers, CI, Forge, and dogfood receipts, but job payload identity is still fragmented across executor-specific fields.

Aspen should define a content-addressed execution closure contract so CI jobs, deploy stages, runtime-host proofs, hooks, and future services can be admitted, transferred, cached, executed, and audited by immutable closure identity.

## What Changes

- Define an immutable execution closure manifest with code/artifact hash, dependency graph, typed input/output schema hashes, runtime target, capability requirements, and provenance.
- Require workers to fetch missing closure dependencies by hash over Aspen's existing blob/Iroh path before execution.
- Require receipts to record closure hash, dependency root, input/output handles, runtime target, and capability proof summary.
- Add deterministic validation and negative coverage for malformed, missing, or mismatched closure manifests.

## In Scope

- OpenSpec requirements for the closure model, admission, transfer, receipt, and first executor slice.
- Compatibility with existing job payloads via adapter/wrapper behavior.
- Bounded receipt and redaction rules.

## Out of Scope

- Replacing every executor in one change.
- Building a Unison-like language/code database.
- Claiming semantic equivalence for arbitrary Rust source code from textual diffs.

## Verification

- `openspec validate add-content-addressed-execution-closures --strict`
- Focused closure manifest/admission unit tests.
- One product-path executor proof that records a closure receipt.
- Negative tests for missing blob, schema mismatch, and denied capability.
- `openspec validate --all --strict --json`
- `git diff --check`
