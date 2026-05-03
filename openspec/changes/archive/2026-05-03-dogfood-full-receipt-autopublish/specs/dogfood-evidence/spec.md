## ADDED Requirements

### Requirement: Full Runs Auto-Publish Final Success Receipts [r[dogfood-evidence.full.auto-publish]]

A successful dogfood `full` run MUST publish its validated canonical run receipt into the running Aspen cluster KV before automatic cleanup begins.

#### Scenario: Successful full run publishes before stop [r[dogfood-evidence.full.auto-publish.success]]

- GIVEN a dogfood `full` run has successfully completed push, build, deploy, and verify stages
- WHEN the run prepares to finish
- THEN it MUST write the canonical local receipt to `dogfood/receipts/<run-id>.json` in Aspen KV before running the stop stage
- AND the local receipt MUST include a succeeded `publish_receipt` stage with the cluster KV key as a receipt artifact
- AND the local receipt file MUST remain the durable fallback evidence after cleanup

#### Scenario: Publication failure fails acceptance [r[dogfood-evidence.full.auto-publish.failure]]

- GIVEN push, build, deploy, and verify have succeeded
- WHEN automatic receipt publication to Aspen KV fails
- THEN the run MUST return a dogfood failure instead of reporting full acceptance success
- AND the local receipt MUST record a failed `publish_receipt` stage
- AND the local receipt MUST remain available for diagnosis

### Requirement: Full Runs Can Leave Cluster Running For Evidence Readback [r[dogfood-evidence.full.leave-running]]

The dogfood CLI MUST provide an explicit `full` mode that leaves a successfully verified cluster running after receipt auto-publication so operators can query the published receipt from Aspen KV.

#### Scenario: Operator leaves cluster running [r[dogfood-evidence.full.leave-running.success]]

- GIVEN an operator runs `aspen-dogfood full --leave-running`
- WHEN push, build, deploy, verify, and automatic receipt publication succeed
- THEN the command MUST exit without running the stop stage
- AND the cluster state MUST remain usable by `receipts cluster-show <run-id> --json`
- AND operator documentation MUST tell the operator to run `stop` after inspection

#### Scenario: Default full still cleans up [r[dogfood-evidence.full.leave-running.default-cleanup]]

- GIVEN an operator runs `aspen-dogfood full` without `--leave-running`
- WHEN push, build, deploy, verify, and automatic receipt publication complete
- THEN the command MUST run the stop stage and clean up the dogfood cluster as before
