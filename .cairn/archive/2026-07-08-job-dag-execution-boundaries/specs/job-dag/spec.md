# Job DAG Delta: Execution Boundaries

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
