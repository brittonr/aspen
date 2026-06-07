## Phase 1: Schedule evidence schema and summaries

- [x] [serial] r[molten.job_worker_coordination_scheduling_ux.schedule_receipt] Add canonical job worker schedule receipts that bind queue, lease, coordination report, worker receipt, and result refs.
- [x] [parallel] r[molten.job_worker_coordination_scheduling_ux.status_summary] Classify schedule receipts in the ledger and show them via job status/receipt UX.

## Phase 2: Coordination-backed scheduled run

- [x] [serial] r[molten.job_worker_coordination_scheduling_ux.queue_admission] Enqueue worker requests through coordination queue operations and record duplicate operation replay.
- [x] [serial] r[molten.job_worker_coordination_scheduling_ux.lease_gate] Acquire a coordination lock/fencing token before worker execution and deny stale token use before worker side effects.
- [x] [serial] r[molten.job_worker_coordination_scheduling_ux.scheduled_local_run] Add the scheduled local worker CLI command and durable evidence directory output.

## Phase 3: Coverage and docs

- [x] [serial] r[molten.job_worker_coordination_scheduling_ux.cli_tests] Cover pass, duplicate operation replay, stale-token denial, output equivalence, and ledger-visible schedule receipts in CLI tests.
- [x] [parallel] r[molten.job_worker_coordination_scheduling_ux.docs] Document the scheduled worker UX and evidence-only trust boundary.
