# Job DAG Specification

## Purpose

Defines the `job-dag` capability.

## Requirements

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

### Requirement: Worker request CLI emits canonical evidence
r[molten.job_dag_iroh_worker_cli_ux.worker_request_cli] Molten MUST provide a CLI command that emits canonical `job-worker-request-v1` records from target admission and execution request artifacts while preserving explicit sync, authority, resource, peer bootstrap, node identity, and evidence refs.

#### Scenario: Worker request binds admission and execution request
- GIVEN a passing target admission receipt and a matching execution request artifact
- WHEN the operator runs `molten test job worker-request`
- THEN Molten emits a canonical worker request
- AND the request binds sync, admission, execution request, authority, resource, peer bootstrap, node identity, and evidence refs.

### Requirement: Worker summaries are read-only
r[molten.job_dag_iroh_worker_cli_ux.worker_summary] Molten MUST summarize worker receipts and results without granting execution authority or mutating worker state.

#### Scenario: Worker receipt appears in job status
- GIVEN a worker receipt has been imported into the ledger
- WHEN `molten test job status` or `receipt-show` reads it
- THEN the CLI reports worker execution decision and refs
- AND no additional worker execution is performed.

### Requirement: Local-gossip worker run records replay evidence
r[molten.job_dag_iroh_worker_cli_ux.local_gossip_run] Molten MUST provide a deterministic local-gossip worker runner that records the worker envelope, transport receipts, delivery log, assignment, status records, result, worker receipt, execution receipt, and output evidence.

#### Scenario: Local-gossip worker completes with recorded delivery
- GIVEN a canonical worker request and target roots
- WHEN `molten test job worker-run-local` runs
- THEN the request is published and delivered through local gossip
- AND a replayable delivery log and worker receipt are written.

### Requirement: Worker execution uses target roots
r[molten.job_dag_iroh_worker_cli_ux.target_roots] Worker CLI execution MUST use explicit target registry, storage, cache, and chunk roots and MUST NOT execute from source registry arguments after assignment.

#### Scenario: Target root arguments are required
- GIVEN a worker request artifact
- WHEN the operator runs the local-gossip worker command
- THEN target registry, storage, cache, and chunk roots are explicit inputs
- AND execution proceeds through the existing target-root worker verifier.

### Requirement: Worker artifacts can be imported to the ledger
r[molten.job_dag_iroh_worker_cli_ux.ledger_import] Worker CLI execution MAY import assignment, status, result, and worker receipt artifacts into the evidence ledger when a ledger root is supplied.

#### Scenario: Worker receipt is ledger-visible
- GIVEN `worker-run-local` is run with `--ledger`
- WHEN the worker completes
- THEN worker artifacts are imported into the ledger
- AND `job status` can list the worker receipt.

### Requirement: Worker CLI behavior is tested
r[molten.job_dag_iroh_worker_cli_ux.cli_tests] Molten SHOULD cover worker request generation, recorded local-gossip execution, receipt summaries, ledger status, and output equivalence in automated tests.

#### Scenario: CLI test runs a worker
- GIVEN the CLI job DAG test suite runs
- WHEN it generates a worker request and runs the local-gossip worker
- THEN the worker receipt decision is pass
- AND the worker output matches the equivalent deterministic local execution.

### Requirement: Worker UX is documented
r[molten.job_dag_iroh_worker_cli_ux.docs] Molten SHOULD document the worker CLI workflow and clarify that transport and CLI receipts are evidence only.

#### Scenario: Operator reads worker docs
- GIVEN an operator reviews Molten documentation
- WHEN they inspect job worker commands
- THEN the docs show request and local-gossip run usage
- AND state that transport does not grant authority, policy, resource, provenance, or execution trust.

### Requirement: Schedule receipts bind coordination and worker evidence
r[molten.job_worker_coordination_scheduling_ux.schedule_receipt] Molten MUST emit canonical `job-worker-schedule-receipt-v1` records for scheduled worker runs, binding the worker request, queue key, lease key, coordination apply report, queue/claim/lease/release receipts, fencing token, worker receipt, and worker result when present.

#### Scenario: Scheduled worker receipt binds refs
- GIVEN a worker request is scheduled and executed
- WHEN the scheduled run completes
- THEN the schedule receipt binds coordination and worker evidence refs
- AND the receipt is canonical and replayable.

### Requirement: Schedule receipts are visible in job UX
r[molten.job_worker_coordination_scheduling_ux.status_summary] Molten MUST classify schedule receipts in the ledger and summarize them through job status and receipt display commands without executing additional work.

#### Scenario: Job status lists schedule receipt
- GIVEN a schedule receipt has been imported into the ledger
- WHEN `molten test job status` reads the ledger
- THEN it lists the schedule receipt and decision
- AND no worker side effect is performed by status rendering.

### Requirement: Worker requests enter through coordination queues
r[molten.job_worker_coordination_scheduling_ux.queue_admission] Scheduled worker execution MUST enqueue the worker request ref through the coordination queue and MUST prove duplicate operation-id replay does not enqueue the request twice.

#### Scenario: Duplicate enqueue replays prior receipt
- GIVEN a scheduled worker command applies the same enqueue request twice
- WHEN coordination idempotency handles the duplicate operation id
- THEN the duplicate enqueue receipt ref equals the original enqueue receipt ref
- AND the worker request is claimed only once.

### Requirement: Lease fencing gates worker side effects
r[molten.job_worker_coordination_scheduling_ux.lease_gate] Scheduled worker execution MUST acquire a coordination lock/fencing token before invoking worker execution and MUST deny stale token use before worker side effects.

#### Scenario: Stale token denies before worker
- GIVEN a worker request has been dequeued and a current fencing token has been acquired
- WHEN a stale token override is supplied
- THEN Molten emits a denying schedule receipt
- AND no worker receipt or output is written.

### Requirement: Scheduled local command writes durable evidence
r[molten.job_worker_coordination_scheduling_ux.scheduled_local_run] Molten MUST provide a scheduled local worker CLI command that writes schedule receipt, coordination manifest/report/evidence, queue receipts, lease token, release receipt, and nested worker execution evidence.

#### Scenario: Scheduled local worker passes
- GIVEN a synced, admitted worker request and target roots
- WHEN `molten test job worker-schedule-local` runs without a stale token
- THEN the worker executes through the recorded local-gossip path
- AND output evidence is written under the scheduled run directory.

### Requirement: Scheduled worker CLI is tested
r[molten.job_worker_coordination_scheduling_ux.cli_tests] Molten SHOULD cover scheduled pass, duplicate enqueue replay, stale-token denial, output equivalence, and ledger-visible schedule receipts in automated tests.

#### Scenario: CLI test exercises schedule flow
- GIVEN the CLI job DAG test suite runs
- WHEN it schedules and executes a worker request
- THEN the schedule receipt decision is pass
- AND stale token coverage denies before worker output.

### Requirement: Scheduled worker trust boundary is documented
r[molten.job_worker_coordination_scheduling_ux.docs] Molten SHOULD document that scheduling receipts, queue claims, lease tokens, and transport logs are evidence only and do not grant authority, policy, resource, source-gate, provenance, sync, admission, or execution trust.

#### Scenario: Operator reads schedule docs
- GIVEN an operator reviews Molten job worker documentation
- WHEN they inspect scheduled worker commands
- THEN the docs describe the coordination queue and lease flow
- AND state that those receipts do not replace authority or provenance gates.

### Requirement: Job DAG topological order is deterministic
r[molten.job_dag_state_machine_proof.topological_order_determinism] Molten MUST prove that valid acyclic job DAGs produce deterministic topological order ids, node index maps, and dependency indices, while duplicate nodes, unknown edge endpoints, and cycles deny before execution.

#### Scenario: Generated acyclic DAG has stable order
- GIVEN a generated bounded acyclic job DAG
- WHEN Molten computes the execution plan more than once
- THEN the order ids, node indices, and dependency indices match
- AND every edge points from an earlier or completed dependency to a later dependent node.

### Requirement: Job scheduler admits only dependency-ready nodes
r[molten.job_dag_state_machine_proof.dependency_readiness_gate] Molten MUST prove that worker scheduling admits a job node only when all dependency indices for that node have completed, and unsatisfied dependency attempts MUST deny before execution.

#### Scenario: Unsatisfied dependency denies worker run
- GIVEN a job node whose dependency index is not present in the completed set
- WHEN the worker scheduler attempts to run the node
- THEN admission denies
- AND no stage execution receipt is emitted for that node.

### Requirement: Job worker schedule receipts replay deterministically
r[molten.job_dag_state_machine_proof.worker_schedule_replay] Molten MUST prove worker schedule receipts bind request identity, stage order, completed indices, output refs, diagnostics, and replay identity so reordered, stale, or mismatched schedules fail closed.

#### Scenario: Reordered worker schedule denies replay
- GIVEN a recorded worker schedule receipt and a schedule replay with stages reordered
- WHEN Molten validates replay identity
- THEN validation denies
- AND diagnostics identify the stage order or output-ref mismatch.


### Requirement: Job DAG responsibilities are semantically separated
r[molten.job_dag.modularity.boundaries] Job DAG implementation SHOULD separate planning, admission, scheduling, worker execution, blob-ref IO, coordination adapter use, receipt construction, and CLI shell behavior.

#### Scenario: Job module ownership is clear
- GIVEN job DAG code is reorganized
- WHEN reviewers inspect the module layout
- THEN each module has an identifiable responsibility such as plan, admission, schedule, worker, blob_io, coordination, receipts, or cli

### Requirement: Job planning and scheduling have pure plans
r[molten.job_dag.modularity.pure_plans] Job DAG planning and scheduling decisions SHOULD be deterministic functions over typed inputs that return structured plans without performing storage, transport, coordination, or executor side effects.

#### Scenario: Valid DAG produces plan
- GIVEN a valid job DAG and admitted dependency inputs represented in memory
- WHEN the planning core evaluates the DAG
- THEN it returns an ordered plan, dependency diagnostics, and receipt input without fetching blobs or executing workers

#### Scenario: Cycle denies plan
- GIVEN a job DAG with a dependency cycle or missing dependency ref
- WHEN the planning core evaluates the DAG
- THEN it returns a deny result without queueing, fetching, or executing work

### Requirement: Execution trust remains explicit
r[molten.job_dag.modularity.execution_trust] Worker execution MUST require explicit admitted job, executable, input, policy, provenance, resource, and effect evidence; blob availability, queue delivery, coordination lease, or transport identity MUST NOT grant execution trust by itself.

#### Scenario: Complete evidence admits execution intent
- GIVEN job admission, executable/input refs, policy, provenance, resource, and effect evidence are valid
- WHEN the execution planner evaluates the request
- THEN it returns an admitted execution intent for the shell to run

#### Scenario: Queue delivery alone denies execution
- GIVEN a worker receives a queued request without required admission or provenance evidence
- WHEN the execution planner evaluates the request
- THEN it denies before fetching executable bytes or invoking an executor

### Requirement: Job boundary changes include positive and negative tests
r[molten.job_dag.modularity.tests] Job DAG boundary refactors SHOULD include positive tests for admitted plans and negative tests for missing provenance, stale admission, dependency cycles, stale leases, missing blob manifests, or unsupported executor profiles.

#### Scenario: Job boundary tests cover denied execution
- GIVEN a job execution boundary is extracted
- WHEN reviewers inspect tests
- THEN at least one denied execution case proves no executor invocation is planned
