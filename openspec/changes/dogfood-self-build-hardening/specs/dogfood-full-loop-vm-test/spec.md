## ADDED Requirements

### Requirement: dogfood-local.sh full-loop completes without errors
The `dogfood-local.sh full-loop` command SHALL execute the complete self-build cycle: start a 1-node cluster, push source to Forge, wait for the 3-stage CI pipeline (check → build → test) to succeed, deploy the CI-built binary, and verify the deployed binary is functional.

#### Scenario: Full loop succeeds on clean machine
- **WHEN** `nix run .#dogfood-local -- full-loop` is executed with no prior cluster state
- **THEN** the script starts a cluster, pushes source, CI pipeline completes with status "success", deploy restarts the node with the CI-built binary, and verify confirms the binary runs and reports a version string containing "aspen"

#### Scenario: Build stage produces aspen-node binary
- **WHEN** the CI pipeline's "build" stage runs `nix build .#checks.x86_64-linux.build-node`
- **THEN** the build output contains `/nix/store/.../bin/aspen-node` and the binary is executable

#### Scenario: Deploy restarts cluster with CI-built binary
- **WHEN** the deploy step replaces the running aspen-node with the CI-built binary
- **THEN** the cluster comes back online within 60 seconds, responds to health checks, and the running binary matches the CI-built store path

### Requirement: NixOS VM integration test for full self-build loop
A NixOS VM test (`ci-dogfood-full-loop-test`) SHALL exercise the complete self-build pipeline in an isolated QEMU VM, using the real `.aspen/ci.ncl` configuration and the repo's own flake checks.

#### Scenario: VM test builds aspen-node through CI pipeline
- **WHEN** the VM test pushes the full workspace source to Forge and triggers the CI pipeline
- **THEN** the CI worker runs `nix build` against the real `checks.x86_64-linux.build-node` flake check, producing a working `aspen-node` binary

#### Scenario: VM test verifies build output
- **WHEN** the CI pipeline completes successfully
- **THEN** the test extracts the output path from the job result, confirms the binary exists at `$output_path/bin/aspen-node`, runs it with `--version`, and asserts the output contains "aspen"

#### Scenario: VM test uses pre-populated nix store
- **WHEN** the VM boots
- **THEN** the VM's nix store contains the pre-built crane cargo artifacts and ciSrc derivation, so the inner `nix build` only compiles Aspen's own crates (no network downloads of cargo deps or nixpkgs toolchain required)

### Requirement: dogfood-local.sh error handling hardened
The script SHALL handle failure modes gracefully: stale cluster state, missing binaries, CI pipeline failures, deploy timeouts, and interrupted runs.

#### Scenario: Script recovers from stale cluster state
- **WHEN** `dogfood-local.sh start` is run but `/tmp/aspen-dogfood` already contains state from a previous run
- **THEN** the script cleans up stale PID files and starts fresh without errors

#### Scenario: Script reports CI failure clearly
- **WHEN** the CI pipeline fails (e.g., clippy lint errors, build failure, test failure)
- **THEN** the script prints the failing stage and job name, shows relevant log output, and exits with non-zero status

#### Scenario: Deploy handles slow restart
- **WHEN** the CI-built binary takes longer than usual to start (up to 60 seconds)
- **THEN** the deploy step waits for the cluster ticket file to appear and health check to pass before declaring success
