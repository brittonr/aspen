## Why

Molten's remote artifact sync and effect handlers make it possible to run work across peers, but batch/data-parallel jobs need a higher-level model than one remote invocation at a time. Unison's distributed dataset examples show a powerful pattern: represent computations as lazy immutable DAGs, move functions to data, fuse stages, and memoize subtrees.

Molten should adopt this pattern for a policy-gated distributed job DAG layer built from content-addressed artifacts, content refs, and admitted effect handlers.

## What Changes

- Add a distributed job DAG model for immutable, content-addressed computations over partitioned data/content refs.
- Represent map/filter/reduce/join-like stages as artifacts with declared schemas, effects, and capabilities.
- Keep jobs lazy until materialization, reduction, subscription, or explicit run request.
- Move admitted computation artifacts to data peers through remote artifact sync instead of moving large data by default.
- Fuse compatible stages where schema/effect/policy constraints allow.
- Memoize intermediate sub-DAG results by stage artifact id, input partition refs, dependency closure hash, handler profile, and policy context.
- Support local, profiling, and chaos handlers for testing distributed job plans before production execution.
- Emit receipts and traces for planning, placement, dependency sync, stage execution, memo hits, and result materialization.

## Impact

This gives Molten a principled batch/job substrate aligned with its artifact, policy, and evidence model. The first milestone can be local-only: define a DAG over content refs, run map/filter/reduce stages with local handlers, cache subresults, and emit canonical traces.
