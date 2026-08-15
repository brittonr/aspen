## Context

The broader `unison-distributed-job-dag` change captures the long-term idea: immutable distributed computations, data-local placement, fusion, profiling, chaos profiles, and remote admission. This local slice narrows the first implementation to a deterministic single-process runner over Molten's current local stores.

Existing subsystems already provide most building blocks:

- `artifacts`: immutable artifact envelopes, dependency closures, reverse-impact queries, and semantic indexes.
- `typed_storage`: schema-tagged value refs, migration receipts, and chunk-backed large values.
- `chunk_store`: content manifests, verified chunk reads/ranges, pins, and local Iroh-shaped exchange receipts.
- `eval_cache`: deterministic cache keys/values/receipts for content-addressed computations.
- `effects`: effect manifests, handler bindings, operation records, and scoped effect-handle evidence.
- `schema_identity`: structural/unique/branded schema identity and compatibility receipts.
- `catalog`: read-only inspection over artifacts, receipts, dependencies, and visibility/redaction hooks.
- `ledger`: immutable receipt/artifact storage and artifact-kind classification.

Unison's distributed dataset model is useful prior art only. Molten does not adopt Unison's hash format, typechecker, runtime, UCM names, or remote execution protocol.

## Goals

- Define canonical local job DAG artifacts and output-request identities.
- Support first stage kinds: `source`, `map`, `filter`, `reduce`, and `materialize`.
- Keep stage programs explicit: stage logic is an artifact ref or an admitted bounded built-in stage-operation artifact, never a live closure capture.
- Execute local jobs deterministically over canonical refs and values.
- Materialize outputs as typed-storage refs, chunk/content refs, or inline canonical value refs according to explicit output policy.
- Reuse `eval_cache` for deterministic stage and sub-DAG memoization.
- Emit receipts and trace records suitable for catalog inspection, transcript gates, and future remote execution.
- Leave enough structure for future planner/fusion/profiling/chaos/remote admission without making those required in this slice.

## Non-Goals

- No distributed scheduling, remote peer execution, or real multi-peer placement yet.
- No arbitrary closure serialization or heap capture.
- No ambient filesystem, environment, process, network, wall-clock, or OS-scheduler observations in deterministic local execution.
- No full Spark/DataFrame engine; the first executor may use bounded Preserves-oriented stage operations.
- No semantic cache hits that bypass current policy/capability/revocation inputs.
- No fusion optimization in the first local runner, except recording where a future planner may attach fusion evidence.
- No global consistency claims or Raft participation for ordinary job traffic.

## DAG model

Introduce a canonical DAG record:

```preserves
<job-dag-v1 "molten.job-dag.dag.v1"
  <version "v1">
  <nodes [<job-node-v1 ...> ...]>
  <edges [<job-edge-v1 ...> ...]>
  <outputs [<job-output-request-v1 ...> ...]>
  <schemas [<schema-ref> ...]>
  <effect-manifests [<effect-manifest-ref> ...]>
  <policies [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "canonical-dag" "pass"> ...]>>
```

Each node has a stable local id scoped to the DAG, a stage kind, zero or more input ports, zero or more output ports, optional stage artifact ref, schemas, effect manifest refs, policy refs, and stage config:

```preserves
<job-node-v1
  <id "stage-id">
  <kind "source"|"map"|"filter"|"reduce"|"materialize">
  <stage-artifact <none>|<some <artifact-ref>>>
  <inputs [<port "in" <schema-ref-or-none>> ...]>
  <outputs [<port "out" <schema-ref-or-none>> ...]>
  <config <canonical-preserves-value-or-ref>>
  <effects [<effect-manifest-ref> ...]>
  <policy [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "stage-artifact-not-closure" "pass"> ...]>>
```

Edges connect output ports to input ports with schema refs and partition/materialization metadata:

```preserves
<job-edge-v1
  <from <node-id> <port-id>>
  <to <node-id> <port-id>>
  <schema <none>|<some <schema-ref>>>
  <partitioning <single>|<partitioned <partition-ref>>>
  <materialization "stream"|"typed-ref"|"content-ref">
  <checks [<check "schema-bound" "pass"> ...]>>
```

Output requests are separately hashed with the DAG ref so different materialization requests do not mutate DAG identity:

```preserves
<job-output-request-v1
  <dag <job-dag-ref>>
  <roots [<node-id> ...]>
  <materialization "inline"|"typed-storage"|"chunk-manifest">
  <policy [<policy-ref> ...]>
  <handler-profile <handler-profile-ref-or-none>>
  <seed-config <none>|<some <seed-config-ref>>>
  <checks [<check "request-ref-bound" "pass"> ...]>>
```

The `job-id` is the ref of the canonical DAG. A `job-run-id` is the ref of the output request plus runtime identity inputs.

## Stage kinds

Initial local stage semantics are deliberately small and deterministic:

- `source`: introduces existing refs or inline canonical values. It may read typed-storage or chunk/content refs only through explicit effect handles and verification receipts.
- `map`: applies a deterministic stage artifact/operation to each input item and emits a value stream or partitioned value refs.
- `filter`: applies a deterministic predicate artifact/operation and preserves only admitted items.
- `reduce`: folds a bounded stream/partition with an explicit associative/ordered reducer artifact; the local runner records the reduction order.
- `materialize`: writes the requested output as inline canonical bytes, a typed-storage value, or a chunk manifest according to policy.

A first implementation can provide bounded built-in stage-operation artifacts for Preserves values, for example `identity`, `project-field`, `tag-record`, `match-pattern`, `count`, `sum-integers`, and `concat-lists`. These operations still need artifact refs/config refs so DAG identity does not depend on ambient code.

## No mobile closures

Stage logic must cross the DAG boundary as an artifact ref with schema/effect/policy evidence. The DAG must reject:

- raw source snippets without artifact envelopes,
- executable host paths,
- process commands,
- unserialized language closures,
- environment-dependent configuration,
- stage configs that rely on mutable names instead of refs.

This rule keeps local execution compatible with future remote artifact sync: the runner can move admitted artifacts and data refs, not live heap state.

## Local execution model

A local run validates the DAG, resolves the requested roots, maps canonical node ids to Trellis numeric DAG indices, topologically orders stages through Trellis `topo_sort`/`is_topo_order`, and executes each stage only after Trellis job dependency readiness (`all_deps_satisfied` plus zero `unsatisfied_count`) holds for completed predecessor indices.

Execution records should include:

- DAG ref and output-request ref,
- stage id and stage artifact ref,
- input refs and schema refs,
- dependency-closure hash for the stage artifact and config refs,
- handler profile and seed/config refs,
- policy/capability/effect-handle refs,
- output refs,
- memo decision,
- diagnostics and checks.

The core semantics are deterministic. Any external observation (typed-storage read, chunk read, materialization write, future network fetch, etc.) must enter through a receipt-bearing effect boundary. The first local runner may execute against local stores, but the receipts must name the effect/action and bind the relevant refs.

## Memoization

Stage memoization should use `eval_cache` with an operation kind such as `job-stage` or `job-output-request`. Memo keys include:

- job DAG ref,
- output request ref when materialization affects semantics,
- stage id and stage artifact ref,
- input refs and partition refs,
- dependency closure hash,
- schema refs and schema-compatibility receipt refs where relevant,
- handler profile and seed/config refs,
- policy/capability/revocation/effect-handle refs,
- Molten job-runner/tool version refs,
- assumptions/check refs.

A memo hit emits a job receipt referencing the eval-cache hit receipt and the original stage execution evidence. Policy-current cache entries must be revalidated exactly like other eval-cache hits.

## Receipts

Introduce canonical job receipts:

```preserves
<job-dag-receipt-v1 "molten.job-dag.receipt.v1"
  <operation "install"|"run"|"stage"|"memo-hit"|"memo-miss"|"materialize"|"deny">
  <decision "pass"|"deny">
  <job <job-dag-ref-or-none>>
  <request <output-request-ref-or-none>>
  <stage <none>|<some <stage-id>>>
  <inputs [<ref> ...]>
  <outputs [<ref> ...]>
  <cache <none>|<some <eval-cache-receipt-ref>>>
  <effects [<effect-receipt-ref> ...]>
  <policy [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <diagnostics ["..."]>
  <checks [<check "no-mobile-closures" "pass"> ...]>>
```

Run receipts aggregate stage receipts in canonical stage order and bind the final materialized output refs. Denial receipts should be canonical and inspectable by catalog/MCP without requiring access to hidden payloads.

## Ledger and catalog integration

The local ledger should classify job DAGs, output requests, run receipts, stage receipts, and materialization receipts as first-class artifact kinds. Catalog views should expose:

- job DAG summaries,
- stage graph and schema refs,
- dependency and impact links through artifact refs,
- run/memo/materialization receipt refs,
- output refs,
- redacted diagnostics.

## CLI

Add local commands under `molten test job`:

- `install <dag.preserves> --registry <path>`: validate and install a DAG artifact, printing the full job ref and install receipt.
- `show <job-ref> --registry <path>`: render DAG nodes, edges, schemas, effect manifests, policies, and evidence refs.
- `run <job-ref-or-file> --registry <path> --storage <path> --cache <path> [--output-request <file>]`: run locally and emit a run receipt plus output refs.
- `status --ledger <path> [--job <ref>]`: list local run/stage/materialization receipts.
- `receipt-show <receipt-ref> --ledger <path>`: render canonical job receipts.

CLI conveniences may accept short ids only if they expand through catalog/registry ambiguity checks before use. Canonical records must always contain full refs.

## Tests and properties

Required tests:

- DAG refs are stable for identical canonical DAGs and change when nodes, edges, schemas, stage artifacts, or policies change.
- Names, paths, mtimes, and short ids do not affect DAG identity.
- A source → map → filter → materialize pipeline produces deterministic output refs and receipts.
- A reduce pipeline records deterministic reduction order and output refs.
- Re-running the same stage inputs produces an eval-cache-backed memo hit with matching output refs.
- Changed input refs or policy refs produce cache misses or stale-deny receipts.
- Raw closure/path/command stage configs are rejected with denial receipts.
- Materialization binds typed-storage/chunk refs and verification receipts.
- Hegel properties cover DAG hash determinism, topological order determinism, memo-key stability, and no-mobile-closure invariants.

## Future work

Future Cairn changes can extend this local model with:

- planner placement proposals,
- safe stage fusion evidence,
- profiling and chaos execution profiles,
- remote artifact sync and peer-local admission,
- distributed job status assertions,
- operation-id/dedup integration,
- protocol drain hooks for upgrade sessions.
