## Phase 1: CLI records and summaries

- [x] [serial] r[molten.job_dag_iroh_worker_cli_ux.worker_request_cli] Add CLI generation for canonical job worker requests from admission and execution request evidence.
- [x] [parallel] r[molten.job_dag_iroh_worker_cli_ux.worker_summary] Extend receipt/status summaries for job worker receipts and results.

## Phase 2: Recorded local-gossip run

- [x] [serial] r[molten.job_dag_iroh_worker_cli_ux.local_gossip_run] Add a CLI local-gossip worker runner that records transport receipts, delivery logs, worker status, result, receipt, and output evidence.
- [x] [serial] r[molten.job_dag_iroh_worker_cli_ux.target_roots] Require target registry/storage/cache/chunk roots for worker execution and keep source-registry execution out of the worker run path.
- [x] [parallel] r[molten.job_dag_iroh_worker_cli_ux.ledger_import] Import worker artifacts into the evidence ledger when requested.

## Phase 3: Coverage and docs

- [x] [serial] r[molten.job_dag_iroh_worker_cli_ux.cli_tests] Cover request generation, local-gossip worker execution, receipt-show, ledger status, and output equivalence in CLI tests.
- [x] [parallel] r[molten.job_dag_iroh_worker_cli_ux.docs] Document the worker CLI UX and evidence-only trust boundary.
