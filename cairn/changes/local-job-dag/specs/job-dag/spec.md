# Job DAG Delta: Local Deterministic Job DAGs

### Requirement: Job DAG identity is canonical
r[molten.local_job_dag.spec.canonical_identity] Local job DAGs MUST derive job refs and output-request refs from canonical Preserves records, not from names, paths, mtimes, short ids, or host-local display metadata.

#### Scenario: Display metadata does not affect identity
- GIVEN two DAG records with identical nodes, edges, schemas, policies, stage artifact refs, and output requests
- AND different CLI names or short-id display handles
- WHEN their refs are computed
- THEN the refs are equal

#### Scenario: Semantic DAG changes affect identity
- GIVEN a DAG record
- WHEN a node kind, edge, schema ref, stage artifact ref, policy ref, or output request changes
- THEN the affected job ref or output-request ref changes

### Requirement: Stage logic is artifact-backed, not closure-backed
r[molten.local_job_dag.spec.no_mobile_closures] Local job DAG stage logic MUST be referenced by admitted artifacts or bounded built-in stage-operation artifacts, and MUST reject raw closures, host paths, process commands, or ambient environment-dependent configs.

#### Scenario: Raw closure is rejected
- GIVEN a DAG node whose stage config contains raw source text, a process command, or a host path as executable identity
- WHEN the DAG is installed or run
- THEN Molten emits a canonical denial receipt
- AND no stage execution side effect occurs

#### Scenario: Artifact-backed stage is admitted
- GIVEN a DAG node with an admitted stage artifact ref, schema refs, effect manifest refs, and policy refs
- WHEN the local runner validates the node
- THEN the node passes the no-mobile-closure check

### Requirement: Local execution is deterministic and effect-bound
r[molten.local_job_dag.spec.local_execution] Local job execution MUST map canonical node ids to Trellis DAG indices, use Trellis topological order validation with deterministic canonical tie-breaking, check Trellis dependency readiness before each stage, and route every storage, chunk, materialization, or future external observation through an explicit effect/evidence boundary.

#### Scenario: Independent stages are ordered canonically
- GIVEN two ready stages with no dependency edge between them
- WHEN a local run schedules the stages
- THEN the runner orders them by the canonical stage scheduling key through the Trellis topo-sort adapter
- AND the run receipt records that order
- AND the run receipt includes `trellis-topo-order` and `trellis-deps-ready` checks

#### Scenario: Storage read is receipt-bound
- GIVEN a source stage that reads a typed-storage ref
- WHEN the stage executes
- THEN the stage receipt includes the typed-storage/effect receipt ref
- AND the input ref is bound in the stage execution record

### Requirement: Memo keys bind all deterministic inputs
r[molten.local_job_dag.spec.memo_keys] Job DAG memo keys MUST bind the job ref, output-request ref where relevant, stage id, stage artifact ref, input refs, dependency closure hash, schema refs, handler profile, policy/capability/revocation refs, effect-handle refs, and tool version refs.

#### Scenario: Same inputs produce a memo hit
- GIVEN a completed deterministic stage execution
- AND a second execution with identical memo-key inputs
- WHEN the stage is evaluated
- THEN the runner may return an eval-cache-backed memo hit
- AND the job receipt references the cache hit and original stage evidence

#### Scenario: Changed policy ref prevents unsafe hit
- GIVEN a policy-current cached stage result
- WHEN the current policy/capability/revocation refs differ from the memo key
- THEN the runner emits a stale-deny or miss receipt
- AND it does not return the cached semantic output as admitted evidence

### Requirement: Job receipts bind execution evidence
r[molten.local_job_dag.spec.receipts] Local job runs MUST emit canonical receipts for install, run, stage execution, memo hit/miss, materialization, and denial, binding job refs, request refs, stage ids, input refs, output refs, effect refs, policy refs, cache refs, diagnostics, and checks.

#### Scenario: Run receipt aggregates stages
- GIVEN a successful local job run
- WHEN the run receipt is emitted
- THEN it references the job ref and output-request ref
- AND it aggregates stage receipts in canonical execution order
- AND it binds final output refs

#### Scenario: Denial receipt is inspectable
- GIVEN a malformed or unauthorized DAG
- WHEN validation fails
- THEN Molten emits a canonical denial receipt with diagnostics and checks
- AND catalog views can render the denial without exposing hidden payloads

### Requirement: CLI expands display handles before use
r[molten.local_job_dag.spec.cli_identity] Local job CLI commands MAY accept names or short ids for convenience only after unambiguous registry/catalog expansion, and MUST store or emit full canonical refs in all DAG, request, and receipt records.

#### Scenario: Ambiguous short id denies
- GIVEN a short-id prefix matching more than one visible job ref
- WHEN a CLI command receives the prefix
- THEN the command denies before running the job
- AND no canonical receipt treats the short id as identity
