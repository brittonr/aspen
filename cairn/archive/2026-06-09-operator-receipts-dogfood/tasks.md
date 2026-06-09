## Phase 1: Receipt schema and CLI

- [x] [serial] r[molten.dogfood.operator_receipt_schema] Define operator receipt schemas with run id, artifacts, config/policy refs, node identity, state hashes, trace refs, child receipts, status, and redaction metadata.
- [x] [serial] r[molten.dogfood.receipts_cli] Add CLI commands to list, show, validate, and export local receipts.
- [x] [parallel] r[molten.dogfood.redaction] Apply secret/confidentiality redaction policy to receipt rendering and export.
- [x] [parallel] r[molten.dogfood.no_logs_as_evidence] Document that logs are auxiliary; receipts/traces are primary evidence.

## Phase 2: Local dogfood workflow

- [x] [serial] r[molten.dogfood.local_command] Implement `molten dogfood local-node` using deterministic local handlers.
- [x] [serial] r[molten.dogfood.vertical_slice] Exercise config load, node identity, artifact install, handler binding, two-actor dataspace exchange, receipt storage, transcript run, and cleanup.
- [x] [parallel] r[molten.dogfood.leave_running] Add an option to leave local state running or preserved for inspection.
- [x] [parallel] r[molten.dogfood.final_receipt] Store a final success/failure receipt with child receipt refs and final state hash.

## Phase 3: Integration and tests

- [x] [serial] r[molten.dogfood.replay_validation] Allow dogfood runs to validate deterministic replay or include first-divergence diagnostics on failure.
- [x] [parallel] r[molten.dogfood.cluster_readback_plan] Plan cluster-backed receipt readback once Raft/control-plane storage is implemented.
- [x] [serial] r[molten.dogfood.cli_tests] Add tests for receipt list/show/validate/export over a local dogfood run.
- [x] [parallel] r[molten.dogfood.property_tests] Add deterministic graph/export tests for receipt child graph integrity and redacted export stability.
