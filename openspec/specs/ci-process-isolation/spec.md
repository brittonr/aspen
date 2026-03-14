## ADDED Requirements

### Requirement: NixBuildSupervisor process manager

The CI orchestrator SHALL manage Nix evaluation and build commands through a `NixBuildSupervisor` that runs them as supervised child processes.

#### Scenario: Nix build spawned as child process

- **WHEN** a CI pipeline stage requires a Nix build
- **THEN** the `NixBuildSupervisor` spawns the Nix command as a `tokio::process::Command` child process

#### Scenario: Supervisor reports build result

- **WHEN** the Nix build child process exits with code 0
- **THEN** the supervisor sends a success result (with stdout/stderr) to the orchestrator via a channel

#### Scenario: Supervisor reports build failure

- **WHEN** the Nix build child process exits with a non-zero code
- **THEN** the supervisor sends a failure result (with exit code and stderr) to the orchestrator via a channel

### Requirement: Build timeout enforcement

The `NixBuildSupervisor` SHALL enforce a configurable timeout (`NIX_BUILD_TIMEOUT`) with a default of 30 minutes. If the child process exceeds this timeout, the supervisor SHALL kill it.

#### Scenario: Timeout kill

- **WHEN** a Nix build process runs longer than `NIX_BUILD_TIMEOUT`
- **THEN** the supervisor sends SIGKILL to the child process
- **THEN** the supervisor reports a timeout failure to the orchestrator

#### Scenario: Custom timeout

- **WHEN** a CI pipeline stage sets `nix_build_timeout_secs: 3600`
- **THEN** the supervisor uses 3600 seconds as the timeout for that build

#### Scenario: Default timeout

- **WHEN** no timeout is configured
- **THEN** the supervisor uses 1800 seconds (30 minutes)

### Requirement: Orchestrator survives build process death

The CI orchestrator process SHALL continue running and processing other pipeline stages when a Nix build child process crashes or is killed.

#### Scenario: Build crash does not crash orchestrator

- **WHEN** a Nix build child process is killed by OOM or segfault
- **THEN** the orchestrator receives a failure notification via the channel
- **THEN** the orchestrator marks the job as failed in Raft KV
- **THEN** the orchestrator continues processing other pipeline stages

#### Scenario: Pipeline state survives process death

- **WHEN** a Nix build child process dies
- **THEN** the pipeline run state in Raft KV (under `_ci:runs:`) reflects the failed job
- **THEN** no pipeline state is lost

### Requirement: Channel-based communication

The `NixBuildSupervisor` SHALL communicate with the orchestrator via `tokio::sync` channels, not via IPC sockets or vsock.

#### Scenario: Result delivery via channel

- **WHEN** a Nix build completes
- **THEN** the result is sent on a `tokio::sync::oneshot` channel to the orchestrator task that initiated the build

#### Scenario: Log streaming via channel

- **WHEN** a Nix build produces stdout/stderr output
- **THEN** output chunks are forwarded to the orchestrator via a `tokio::sync::mpsc` channel for real-time log writing
