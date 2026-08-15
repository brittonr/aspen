## ADDED Requirements

### Requirement: Immutable job DAG model
r[molten.job_dag.model] Molten MUST define immutable content-addressed job DAG artifacts with nodes, edges, schema refs, stage artifact refs, data/content refs, effect manifest refs, policy refs, and evidence refs.

#### Scenario: DAG artifact names computation graph
- GIVEN a job DAG artifact with source, map, filter, reduce, and materialize stages
- WHEN Molten parses the artifact
- THEN the parsed graph exposes typed nodes, edges, schemas, stage artifacts, data refs, policies, and evidence refs without mutating the artifact.

### Requirement: Initial stage kinds
r[molten.job_dag.stage_kinds] Molten MUST support the initial stage kinds `source`, `map`, `filter`, `reduce`, and `materialize` for local deterministic jobs.

#### Scenario: Unsupported stage kind denies
- GIVEN a DAG node with an unsupported stage kind
- WHEN the local executor validates the DAG
- THEN execution denies before producing outputs.

### Requirement: Canonical job hashing
r[molten.job_dag.canonical_hashing] Molten MUST hash canonical DAG definitions and root output requests into stable content-addressed job ids and request refs.

#### Scenario: Equivalent DAG hashes identically
- GIVEN two equivalent DAG definitions and output requests
- WHEN Molten encodes each at the canonical boundary
- THEN both produce the same job id and request ref.

### Requirement: Stages are artifacts, not mobile closures
r[molten.job_dag.no_mobile_closures] Molten MUST treat executable job stages as admitted artifacts with effect manifests and handler profiles, not arbitrary serialized live heap closures.

#### Scenario: Inline live closure is rejected
- GIVEN a job stage attempts to carry inline closure state instead of an artifact or admitted built-in operation
- WHEN Molten validates the job
- THEN the stage is rejected before execution.

### Requirement: Local deterministic executor
r[molten.job_dag.local_executor] Molten MUST implement a local deterministic executor for `source`, `map`, `filter`, `reduce`, and `materialize` over canonical Preserves values, content refs, or typed durable refs.

#### Scenario: Local pipeline materializes result
- GIVEN a DAG with source, filter, map, reduce, and materialize stages
- WHEN Molten runs the DAG locally with admitted policy and resources
- THEN it produces deterministic output refs and stage receipts.

### Requirement: Memo keys bind deterministic inputs
r[molten.job_dag.memo_keys] Molten MUST define memo keys over stage artifact or operation refs, input refs, dependency closure hash, schema refs, handler profile, seed/config, and relevant policy refs.

#### Scenario: Policy change invalidates memo hit
- GIVEN a stage output exists in the evaluation cache under prior policy refs
- WHEN the current policy refs differ
- THEN Molten denies semantic use of the stale memo entry.

### Requirement: Memo receipts
r[molten.job_dag.memo_receipts] Molten MUST emit canonical trace or receipt records for memo hits, memo misses, stage execution, and result materialization.

#### Scenario: Memoized rerun records hit
- GIVEN a deterministic stage has already executed with the same memo key
- WHEN the job reruns
- THEN the runtime records a memo-hit receipt referencing the prior output.

### Requirement: Evaluation cache integration
r[molten.job_dag.eval_cache_integration] Molten MUST reuse the evaluation cache for deterministic sub-DAG memoization and MUST preserve policy/currentness checks before semantic reuse.

#### Scenario: Deterministic sub-DAG uses cache
- GIVEN a deterministic map stage output is in the evaluation cache
- WHEN an equivalent job run reaches that stage
- THEN Molten can reuse the cached output only after cache admission passes.

### Requirement: Job planner
r[molten.job_dag.planner] Molten MUST provide a planner that proposes stage order and placement from DAG dependencies, data locality, cache availability, handler profiles, capabilities, resource limits, and policy.

#### Scenario: Planner emits proposal only
- GIVEN a DAG with multiple dependent stages
- WHEN Molten plans the DAG
- THEN it emits a deterministic stage order and placement proposal without executing stages.

### Requirement: Conservative fusion preview
r[molten.job_dag.fusion] Molten MUST fuse or preview adjacent stages only when schema, effect, policy, materialization, and trace constraints allow; current fusion is conservative and preview-only unless later admitted execution support is added.

#### Scenario: Materialization boundary prevents fusion
- GIVEN two adjacent stages separated by an explicit materialization boundary
- WHEN Molten builds a fusion preview
- THEN the preview does not fuse across that boundary.

### Requirement: Profiling profile
r[molten.job_dag.profiling_profile] Molten MUST provide a deterministic profiling surface that records estimated data movement, stage counts, materialization boundaries, cache projections, and stage costs without using wall-clock time as identity.

#### Scenario: Profiling emits canonical estimate
- GIVEN a local job DAG
- WHEN profiling runs
- THEN Molten emits a canonical profile record and receipt with deterministic estimates.

### Requirement: Chaos profile evidence
r[molten.job_dag.chaos_profile] Molten SHOULD bind job planning/execution tests to existing bounded chaos handler profiles or deterministic chaos schedule evidence when fault, delay, reorder, or partition behavior is exercised.

#### Scenario: Chaos remains evidence-only
- GIVEN a job run uses a chaos handler profile or schedule
- WHEN receipts are emitted
- THEN the chaos schedule is canonical evidence and does not grant production authority.

### Requirement: Remote artifact sync for jobs
r[molten.job_dag.remote_sync] Molten MUST use remote artifact sync or loopback sync receipts to move admitted stage artifacts and dependency closures to target peers before worker execution.

#### Scenario: Missing target stage is synced before execution
- GIVEN a worker target is missing a required stage artifact
- WHEN the sync loopback runs with passing provenance and policy evidence
- THEN the target registry receives the stage artifact before execution admission.

### Requirement: Target-side remote admission
r[molten.job_dag.remote_admission] Molten MUST require each target peer or loopback target to admit data access, handler binding, placement, resource profile, source-gate evidence, and stage execution locally before worker execution.

#### Scenario: Missing target admission denies worker
- GIVEN a worker request lacks target admission or required resource evidence
- WHEN the target attempts to execute the job
- THEN execution denies before producing worker output.

### Requirement: Job DAG tests
r[molten.job_dag.basic_tests] Molten MUST include tests for local source/map/filter/reduce/materialize DAGs, memoized reruns, planning/profile/fusion previews, loopback sync, target admission, and worker execution.

#### Scenario: Local DAG test runs twice
- GIVEN the job DAG test suite runs
- WHEN a deterministic local pipeline is executed twice
- THEN the first run executes stages and the second run can record memo hits with equivalent output.

### Requirement: Job DAG property tests
r[molten.job_dag.property_tests] Molten SHOULD include Hegel property tests for DAG hash determinism, fusion safety preconditions, memo-key stability, worker request identity, and no-ordinary-Raft-traffic invariants.

#### Scenario: Generated DAG hash is stable
- GIVEN generated bounded DAG fixtures with equivalent canonical content
- WHEN Molten computes their refs and memo keys
- THEN the refs and memo keys remain stable across repeated runs.
