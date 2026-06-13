# Job DAG Delta: Loopback Execution Gate

### Requirement: Loopback execution requires passing admission
r[molten.job_dag_loopback_execution.spec.admission_required] Loopback job execution MUST require a readable canonical `job-admission-receipt-v1` with decision `pass` before any stage logic is executed.

#### Scenario: Missing admission denies before execution
- GIVEN a synced target registry
- WHEN execute-loopback is requested without an admission receipt
- THEN execution denies
- AND no stage receipt or output is produced

#### Scenario: Denied admission denies execution
- GIVEN a `job-admission-receipt-v1` with decision `deny`
- WHEN execute-loopback is requested
- THEN execution denies before running any stage
- AND the execution receipt identifies the denied admission ref

### Requirement: Execution verifies admission binding
r[molten.job_dag_loopback_execution.spec.admission_binding] Loopback execution MUST verify that the admission receipt binds the requested job ref, target peer, sync ref, selected stages, target closure refs, authority admission refs, and resource refs.

#### Scenario: Target peer mismatch denies
- GIVEN a passing admission receipt for target peer `peer:a`
- WHEN execute-loopback is requested for target peer `peer:b`
- THEN execution denies before running stages

#### Scenario: Job mismatch denies
- GIVEN a passing admission receipt for job A
- WHEN execute-loopback is requested for job B
- THEN execution denies before running stages

### Requirement: Execution revalidates target closure immediately before running
r[molten.job_dag_loopback_execution.spec.closure_revalidate] Loopback execution MUST recompute the target job/stage artifact closure immediately before running and deny if it diverges from the admitted closure refs.

#### Scenario: Closure changes after admission
- GIVEN a target registry that passed admission
- AND a required artifact is removed, missing, or replaced after admission
- WHEN execute-loopback is requested
- THEN execution denies
- AND no stage logic is executed

### Requirement: Execution uses target roots only
r[molten.job_dag_loopback_execution.spec.target_only] Loopback execution MUST use the target registry, target storage, target cache, and target chunk roots only; it MUST NOT accept or read a source registry argument.

#### Scenario: Source registry unavailable after sync
- GIVEN a job closure synced and admitted in the target registry
- AND the source registry is unavailable
- WHEN execute-loopback runs
- THEN execution can pass using only target roots

### Requirement: Execution receipts bind admission and outputs
r[molten.job_dag_loopback_execution.spec.receipts] Loopback execution receipts MUST bind the execution request ref, admission receipt ref, sync ref, authority admission refs, resource refs, stage receipt refs, output refs, target peer, and checks for admission-pass, closure-revalidated, target-only-execution, and deterministic-stage receipts.

#### Scenario: Successful execution receipt is replay evidence
- GIVEN a synced target closure and passing capability-backed admission
- WHEN execute-loopback succeeds
- THEN the receipt decision is pass
- AND the receipt contains admission, sync, authority, stage, output, target peer, and resource refs
- AND the outputs match equivalent deterministic local execution
