# Job DAG Delta: Coordination-backed Worker Scheduling UX

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
