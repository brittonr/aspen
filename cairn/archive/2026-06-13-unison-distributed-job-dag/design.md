## Context

Molten can route envelopes, sync artifacts, run sandboxed actors, and store typed values. A distributed job layer can compose those primitives into reusable data-parallel computations. Unison's distributed dataset article is prior art: an immutable data structure can become distributed by wrapping chunks in remote values; computations can be lazy, fused, and memoized.

Molten's version should be explicit about artifacts, schemas, policies, capabilities, and evidence. Jobs are not arbitrary mobile closures; each stage is an admitted artifact with an effect manifest and handler bindings.

## Goals

- Model distributed jobs as immutable content-addressed DAGs.
- Treat data partitions and intermediate results as content refs or typed storage refs.
- Move computation artifacts to data when policy and capabilities permit.
- Fuse compatible stages to reduce materialization and network transfer.
- Memoize deterministic sub-DAG results by canonical inputs and dependency closures.
- Support local/profiling/chaos execution profiles for testing and planning.
- Emit trace and receipt data suitable for debugging and cost/performance analysis.

## Non-Goals

- Do not build a full Spark replacement in the first milestone.
- Do not support arbitrary closure capture from live heaps.
- Do not bypass storage, remote sync, or effect-handler admission.
- Do not use Raft for ordinary data-parallel job traffic.
- Do not assume all peers are trusted to execute all stages or access all data.

## DAG model

A job DAG should contain:

- `job_id`: hash of canonical DAG definition and root output request.
- `nodes`: source, map, filter, flat_map, reduce, group, join, materialize, external effect, or future extensions.
- `edges`: typed dataflow edges with schema refs and partitioning metadata.
- `stage_artifact_refs`: executable artifacts for stage logic.
- `data_refs`: content refs or typed durable refs for input partitions.
- `effect_manifest_refs`: stage effect declarations.
- `policy_refs`: placement, data access, resource, and output policies.
- `evidence_refs`: provenance, prior memo receipts, and review records.

The DAG is immutable. Running it creates execution records and result refs, not mutations to the DAG artifact.

## Planning and placement

The planner should decide where each stage can run based on:

- data locality,
- available artifact/dependency cache,
- handler profile availability,
- capabilities and data-access policy,
- resource limits,
- network cost estimates,
- fault/chaos profile if testing.

Placement decisions are proposals until admitted by policy. Remote peers apply their own local admission before executing a stage.

## Fusion and memoization

Fusion combines adjacent stages when:

- schemas compose,
- effect manifests remain admitted,
- no materialization boundary is required,
- policy does not require an intermediate receipt or durable output,
- trace granularity requirements are preserved.

Memo keys include stage/fused-stage artifact ids, input partition refs, dependency closure hash, schema refs, handler profile, deterministic seed/config, and relevant policy refs. Memo hits emit trace records and reference prior execution receipts.

## Execution profiles

- `local`: single-process deterministic execution for tests.
- `profiling`: records estimated network/data movement, stage costs, and hot spots.
- `chaos`: deterministic fault, delay, reorder, and partition injection.
- `production`: remote artifact sync, admitted handlers, and real storage/blob/network adapters.

Profiles share DAG semantics but may differ in handler bindings and trace detail.

## Policy and evidence

Job submission, data access, placement, remote sync, stage execution, memo use, fusion, and materialization are trust-boundary actions. Receipts should include job id, stage id, input refs, output refs, handler profile, placement decision, policy refs, and trace refs.

## Open Questions

- Which stage set should be first: source, map, filter, reduce, materialize?
- Should job DAGs be represented directly as Preserves or through Nickel-authored manifests compiled to Preserves?
- How should partial failures and retries interact with memoized stage outputs?
- What cost model is sufficient for first profiling handlers?
