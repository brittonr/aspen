## Why

Even when canonical proof artifacts exist, reviewers need a compact way to answer why a requirement passed, what evidence supports it, and what remains out of scope. Proof review readbacks make that review path deterministic and operator-readable without making summaries normative.

## What Changes

- Add requirement-centered proof readbacks.
- Show positive evidence, negative evidence, command receipts, aggregate obligations, artifact refs, caveats, and stale diagnostics.
- Provide CLI/readback surfaces for local and release review.
- Use Hegel RS properties for stable ordering, grouping, and summary/ref consistency.

## Impact

- **Files**: operator/readback CLI, traceability summary rendering, docs, tests.
- **Testing**: positive readback fixture, gap/stale diagnostics fixture, Hegel RS generated readback summaries.
