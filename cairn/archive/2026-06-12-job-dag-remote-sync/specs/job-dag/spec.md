# Job DAG Delta: Remote Sync Loopback

### Requirement: Sync requests are transport-neutral and non-executing
r[molten.job_dag_remote_sync.spec.request_no_execution] Job sync requests MUST bind job refs, selected stages, target peer identity, policy/capability/evidence refs, and no-execution checks without naming a concrete transport as semantic identity.

#### Scenario: Request does not execute
- GIVEN a job sync request
- WHEN it is parsed or planned
- THEN no stage logic is executed
- AND the request checks include `no-execution`

### Requirement: Sync plans compute dependency closures and missing sets
r[molten.job_dag_remote_sync.spec.plan_missing_set] Job sync plans MUST compute source dependency closures from full artifact refs and target missing sets from target registry contents, never from names, paths, or mtimes.

#### Scenario: Empty target reports full closure missing
- GIVEN a source registry with a job artifact and stage dependency closure
- AND an empty target registry
- WHEN sync-plan runs
- THEN the missing set contains the closure refs required by the job/stage selection

### Requirement: Loopback sync verifies hashes before install acceptance
r[molten.job_dag_remote_sync.spec.loopback_hash_verify] Loopback sync MUST install missing artifacts in dependency-first order and verify that target artifact refs and canonical envelopes match the source after install.

#### Scenario: Missing dependency installs before dependent
- GIVEN a stage artifact that depends on another artifact
- WHEN sync-loopback runs to an empty target
- THEN the dependency is installed before the dependent
- AND the final target envelope refs match source refs

### Requirement: Sync receipts bind no-execution evidence
r[molten.job_dag_remote_sync.spec.receipts] Sync receipts MUST bind the job ref, request ref, plan ref, installed refs, already-present refs, and checks for dependency closure, hash verification, loopback transfer, no-mobile-closures, and no-execution.

#### Scenario: Repeated sync is no-op evidence
- GIVEN a target registry already containing the source closure
- WHEN sync-loopback runs again
- THEN no new missing artifacts are installed
- AND the receipt records already-present refs and `no-execution`
