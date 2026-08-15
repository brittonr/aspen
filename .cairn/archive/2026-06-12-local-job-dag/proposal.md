## Why

Molten now has the local primitives needed for a deterministic job substrate: immutable artifact refs, typed storage refs, chunk/content manifests, effect-handle evidence, evaluation-cache keys, executable transcripts, and catalog/MCP inspection. The broader `unison-distributed-job-dag` change describes a distributed future, but the next useful milestone is a local-only DAG runner that proves the content-addressed model before adding remote placement.

A local job DAG should make computations explicit artifacts and receipts, not ambient scripts or mobile heap closures. It should run over admitted refs, memoize deterministic subresults, and emit evidence that later remote execution can preserve.

## What Changes

- Add a local immutable `job-dag-v1` model with nodes, edges, schemas, stage artifact refs, data refs, effect manifests, policy refs, and evidence refs.
- Define first stage kinds: `source`, `map`, `filter`, `reduce`, and `materialize`.
- Hash DAG definitions and output requests by canonical Preserves identity; names, paths, mtimes, and short ids remain display metadata only.
- Require stage logic to be referenced by admitted artifacts or bounded built-in stage-operation artifacts; do not capture arbitrary live closures.
- Add a deterministic local executor over inline Preserves values, typed-storage refs, and chunk/content refs, with external observations routed through explicit effect/evidence boundaries.
- Reuse the evaluation cache for stage/sub-DAG memoization using keys that bind job refs, stage refs, input refs, dependency closure, schema refs, handler profile, policy/capability refs, and tool versions.
- Emit canonical receipts for install, run, stage execution, memo hit/miss, materialization, and denial.
- Add local CLI commands under `molten test job` for installing, showing, running, inspecting status, and showing receipts.

## Impact

This gives Molten a small but evidence-bearing batch/job core that composes the existing artifact, storage, cache, and catalog work. It also establishes the semantic contract that later distributed job execution must preserve: content-addressed DAGs, no mobile closures, deterministic local replay, explicit policy/effect boundaries, memoized refs, and receipt-backed outputs.
