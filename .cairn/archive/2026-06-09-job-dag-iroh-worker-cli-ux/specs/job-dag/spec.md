# Job DAG Delta: Iroh Worker CLI UX

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
