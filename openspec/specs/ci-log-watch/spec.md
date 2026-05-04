# CI Log Watch Specification

## Purpose

This spec defines push-based CI log streaming for CLI and TUI clients over Aspen WatchSession subscriptions.

## Requirements

### Requirement: CLI follow mode uses push-based log streaming

The CLI `ci logs --follow` command SHALL use a `WatchSession` KV prefix subscription to receive log chunks in real time instead of polling. The subscription prefix SHALL be `_ci:logs:{run_id}:{job_id}:`.

#### Scenario: Follow a running job's logs

- **WHEN** a user runs `aspen-cli ci logs <run_id> <job_id> --follow` and the job is producing output
- **THEN** new log chunks SHALL appear within 1 second of being flushed to KV

#### Scenario: Follow a job that has not started yet

- **WHEN** a user runs `ci logs --follow` before the job has produced any output
- **THEN** the CLI SHALL wait for the watch connection and display chunks as they arrive, without exiting

#### Scenario: Stream completes when log writer closes

- **WHEN** the watch subscription receives a Set event for the shared completion marker key (`CI_LOG_COMPLETE_MARKER`)
- **THEN** the CLI SHALL print any remaining buffered output and exit with status 0
- **AND** the marker SHALL mean the log stream is closed, not that the CI job succeeded or failed

### Requirement: Historical catch-up before watch

The CLI SHALL fetch all existing log chunks via `CiGetJobLogs` RPC before opening the watch subscription, so that logs produced before the follow command started are not missed.

#### Scenario: Job already has partial output

- **WHEN** a user runs `ci logs --follow` on a job that has already produced 5 chunks
- **THEN** the CLI SHALL display all 5 existing chunks immediately, then stream new chunks via watch

#### Scenario: Job already completed

- **WHEN** a user runs `ci logs --follow` on a completed job
- **THEN** the CLI SHALL display all chunks and exit without opening a watch connection

### Requirement: Graceful fallback to polling

If the `WatchSession` connection fails, the CLI SHALL fall back to the existing 1-second polling loop without user-visible errors.

#### Scenario: Watch connection refused

- **WHEN** the node does not accept `LOG_SUBSCRIBER_ALPN` connections
- **THEN** the CLI SHALL silently fall back to polling via `CiGetJobLogs` every 1 second

#### Scenario: Watch connection drops mid-stream

- **WHEN** the watch connection drops after receiving some chunks
- **THEN** the CLI SHALL resume polling from the last received chunk index

### Requirement: TUI live log streaming

The TUI log viewer SHALL use a background `WatchSubscription` to receive log updates when `is_streaming` is true, pushing new lines to the display without manual refresh.

#### Scenario: Open log viewer for running job

- **WHEN** a user opens the log viewer for a running job in the TUI
- **THEN** new log lines SHALL appear automatically as chunks are committed to KV

#### Scenario: Log viewer auto-scroll

- **WHEN** new log lines arrive via watch and auto-scroll is enabled
- **THEN** the scroll position SHALL advance to show the latest lines

### Requirement: AspenClient exposes connection primitives

The CLI's `AspenClient` SHALL expose methods to access the Iroh `Endpoint`, cluster identifier, and peer addresses, so that callers can construct `WatchSession` connections.

#### Scenario: Access endpoint for watch connection

- **WHEN** a caller needs to create a `WatchSession`
- **THEN** `AspenClient::endpoint()` SHALL return a reference to the Iroh `Endpoint`
- **AND** `AspenClient::first_peer_addr()` SHALL return the first bootstrap peer's `EndpointAddr`
- **AND** `AspenClient::cluster_id()` SHALL return the cluster identifier string
