## MODIFIED Requirements

### Requirement: VM-CI Post-Registration Diagnostics [r[dogfood-evidence.vm-ci-post-registration-diagnostics]]

VM-CI dogfood evidence MUST distinguish post-registration execution boundaries after a guest worker has registered, including job assignment, executor start, source/workspace materialization, job result publication, and cluster-side CI status reporting.

#### Scenario: Missing source archive is classified separately [r[dogfood-evidence.vm-ci-post-registration-diagnostics.missing-source-archive]]

- GIVEN a VM-CI dogfood run reaches guest worker registration and job assignment
- AND a nix job fails before build evaluation because the per-job workspace lacks `flake.nix`
- WHEN diagnostics are collected
- THEN the evidence summary MUST classify the failure as `workspace_source_materialization` or an equivalent source-materialization category
- AND the evidence MUST include bounded, redacted facts showing whether `source_hash` was present, whether archive materialization ran, and whether root `flake.nix` existed after materialization
- AND the evidence MUST NOT require scraping unbounded VM serial logs to identify the failing boundary
