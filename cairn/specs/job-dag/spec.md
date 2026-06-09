# Job DAG Specification

## Purpose

Defines the `job-dag` capability.

## Requirements

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
