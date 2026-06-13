# Job DAG Delta: Iroh Worker Execution

### Requirement: Remote worker execution requires target admission
r[molten.job_dag_iroh_worker.spec.target_admission] A remote job worker MUST execute only after target-side sync, target-side admission, and loopback-compatible execution request verification pass.

#### Scenario: Admitted worker executes
- GIVEN a job closure synced to target peer `peer:b`
- AND a passing target admission receipt for selected stages
- AND a valid execution request bound to the target state
- WHEN the worker receives the request
- THEN it executes selected stages from target roots only
- AND emits a worker result binding execution receipt and outputs

#### Scenario: Missing admission denies
- GIVEN a worker request without a readable passing admission receipt
- WHEN the target receives it
- THEN it emits a denial receipt
- AND no stage logic runs

### Requirement: Worker transport is evidence, not authority
r[molten.job_dag_iroh_worker.spec.transport_not_authority] Iroh endpoint identity and message delivery MUST NOT grant job execution authority without explicit authority, resource, sync, and admission refs.

#### Scenario: Known peer without authority denied
- GIVEN a worker request delivered from a bootstrapped peer
- AND no authority context admitting `job:execute` for the job ref
- WHEN the target validates the request
- THEN execution is denied despite valid transport evidence

### Requirement: Replay distinguishes recorded and live runs
r[molten.job_dag_iroh_worker.spec.replay] Worker runs MUST be replayable from recorded delivery/effect logs or explicitly marked non-replayable and excluded from deterministic pass gates.

#### Scenario: Recorded worker run gates
- GIVEN a worker request/status/result delivery log and stage effect log
- WHEN gate validation replays the run
- THEN the same result refs and execution receipts are produced

#### Scenario: Live unrecorded run is diagnostic only
- GIVEN a live Iroh worker run without complete recorded delivery/effect logs
- WHEN pass-evidence gate validation runs
- THEN the worker result is marked non-replayable diagnostic evidence
