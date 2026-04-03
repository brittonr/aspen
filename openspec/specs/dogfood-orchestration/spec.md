## ADDED Requirements

### Requirement: Cluster lifecycle management

The dogfood orchestrator SHALL start, health-check, and stop aspen-node processes using typed process handles. It SHALL support single-node and multi-node (federation) topologies. Node processes SHALL be terminated on orchestrator exit (SIGINT/SIGTERM) via a shutdown handler.

#### Scenario: Start single-node cluster

- **WHEN** the orchestrator starts in single-node mode
- **THEN** it spawns an `aspen-node` process, waits for the health endpoint to report healthy via `GetHealth` RPC, and stores the cluster ticket for subsequent operations

#### Scenario: Start federation clusters

- **WHEN** the orchestrator starts in federation mode
- **THEN** it spawns two independent `aspen-node` processes (alice and bob), waits for both to report healthy, and establishes federation trust between them via `AddPeerCluster` RPC

#### Scenario: Graceful shutdown on signal

- **WHEN** the orchestrator receives SIGINT or SIGTERM
- **THEN** it sends SIGTERM to all managed node processes, waits up to 10 seconds for exit, then sends SIGKILL if any remain

#### Scenario: Node crash detection

- **WHEN** a managed node process exits unexpectedly during an orchestration step
- **THEN** the orchestrator reports the failure with the node's stderr output and exits with a non-zero code

### Requirement: Forge push via typed client API

The orchestrator SHALL create Forge repositories and push source code using `aspen-client` RPC calls, not by shelling out to `aspen-cli` or `git-remote-aspen`. Repository creation SHALL use the appropriate Forge RPC. Git push SHALL use `git-remote-aspen` as a git remote helper (this is the one external process that must remain — git push requires the remote helper protocol).

#### Scenario: Create forge repo and push source

- **WHEN** the orchestrator executes the push step
- **THEN** it creates a Forge repository via RPC if it doesn't exist, configures a git remote using `aspen://` URL, and pushes the workspace source via `git push`

#### Scenario: Repository already exists

- **WHEN** the target Forge repository already exists from a previous run
- **THEN** the orchestrator skips creation and pushes to the existing repository

### Requirement: CI pipeline monitoring via typed client API

The orchestrator SHALL trigger CI builds and poll pipeline status using `aspen-client` RPCs. It SHALL support both automatic (push-triggered) and manual (`ci run`) pipeline execution. Polling SHALL use exponential backoff with a configurable timeout.

#### Scenario: Wait for push-triggered CI pipeline

- **WHEN** the orchestrator has pushed source and a CI pipeline triggers automatically
- **THEN** it polls pipeline status via RPC with exponential backoff (starting at 1s, max 10s), and reports success when the pipeline completes with status "success"

#### Scenario: CI pipeline timeout

- **WHEN** a CI pipeline does not complete within the configured timeout (default: 600 seconds)
- **THEN** the orchestrator fetches and displays the last 50 lines of CI logs via RPC and exits with an error

#### Scenario: CI pipeline failure

- **WHEN** a CI pipeline completes with a non-success status
- **THEN** the orchestrator fetches full CI logs via RPC, displays them, and exits with a non-zero code

### Requirement: Artifact verification

The orchestrator SHALL verify that CI-built artifacts are functional by comparing them against locally-built artifacts and/or executing them. Verification SHALL use typed blob and KV operations via `aspen-client`.

#### Scenario: Verify CI-built binary

- **WHEN** the orchestrator executes the verify step
- **THEN** it retrieves the CI-built artifact via blob RPC, executes it with `--version`, and confirms the output contains the expected version string

#### Scenario: Verify deploy completes

- **WHEN** the orchestrator executes the deploy step
- **THEN** it triggers a deploy via RPC, polls deploy status until complete or timeout, and verifies the running node reports the new version via `GetRaftMetrics` RPC

### Requirement: Shared orchestration logic across modes

The single-node, VM-CI, and federation modes SHALL share common orchestration steps (health-check, forge push, CI wait, verify). Mode-specific behavior (second cluster, VM executor config) SHALL be injected via configuration, not code duplication.

#### Scenario: Single-node and federation share push logic

- **WHEN** the push step executes in either single-node or federation mode
- **THEN** the same forge-push function is called, parameterized only by which cluster's client to use

#### Scenario: VM-CI mode uses same CI monitoring

- **WHEN** the CI wait step executes in VM-CI mode
- **THEN** it uses the same CI polling logic as standard mode — only the cluster's executor configuration differs
