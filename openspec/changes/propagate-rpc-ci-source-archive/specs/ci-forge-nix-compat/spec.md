## MODIFIED Requirements

### Requirement: Forge checkout produces valid flake directory

The CI executor SHALL produce a working directory from Forge-checked-out source that `nix build` can evaluate as a flake. The directory MUST contain `flake.nix`, `flake.lock`, and be a git repository (so nix detects it as a git flake). For RPC-triggered CI runs, the checked-out Forge tree MUST also be archived into the configured blob store and attached to the pipeline context as `source_hash` before jobs are enqueued, so isolated VM workers can materialize the same source root.

#### Scenario: Clippy check on Forge checkout

- **WHEN** the CI pipeline runs a nix job with `flake_url = "."` and `attribute = "checks.x86_64-linux.clippy"` on source checked out from Forge
- **THEN** the nix build SHALL evaluate the flake and execute the clippy check successfully (exit code 0)

#### Scenario: flake.lock preserved through Forge push

- **WHEN** source is pushed to Forge via `git-remote-aspen`
- **THEN** the `flake.lock` file SHALL be present in the Forge repository and available in CI checkouts

#### Scenario: CI working directory is a git repo

- **WHEN** the CI executor prepares a working directory for a nix flake job
- **THEN** the directory SHALL be initialized as a git repository with all source files committed, so that `nix build .#<attr>` resolves the flake correctly

#### Scenario: RPC-triggered VM job receives source archive [r[ci-forge-nix-compat.rpc-trigger-source-archive]]

- GIVEN `CiTriggerPipeline` checks out a Forge repository containing `flake.nix` and `.aspen/ci.ncl`
- AND the pipeline orchestrator has a blob store configured
- WHEN the handler starts the pipeline run
- THEN it MUST create a source archive from the checkout directory
- AND it MUST set `PipelineContext.source_hash` to the archive hash before jobs are enqueued
- AND VM/local executor job payloads derived from that context MUST preserve the same `source_hash`

#### Scenario: VM workspace root contains flake after archive materialization [r[ci-forge-nix-compat.vm-workspace-flake-root]]

- GIVEN a VM worker receives a nix job with `source_hash` and `flake_url = "."`
- WHEN the local executor seeds the per-job workspace from the source archive
- THEN the job working directory MUST contain `flake.nix` at the workspace root before `nix build` starts
- AND failure to fetch, unpack, or validate that archive MUST be reported as source materialization failure instead of a generic missing-flake build failure
