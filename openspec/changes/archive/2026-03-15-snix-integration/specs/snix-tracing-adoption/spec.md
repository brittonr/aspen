## ADDED Requirements

### Requirement: Use snix-tracing in snix-facing binaries

The `aspen-snix-bridge` binary SHALL use `snix-tracing::TracingBuilder` for tracing initialization instead of manual `tracing_subscriber` setup.

#### Scenario: Default tracing initialization

- **WHEN** `aspen-snix-bridge` starts without OTLP configuration
- **THEN** the binary SHALL initialize tracing with `snix-tracing`'s default formatter (structured, env-filter aware)

#### Scenario: OTLP export enabled

- **WHEN** `aspen-snix-bridge` starts with `OTEL_EXPORTER_OTLP_ENDPOINT` set
- **THEN** the binary SHALL export traces via OpenTelemetry OTLP in addition to stderr logging

### Requirement: Scoped adoption

Only binaries with primary snix integration (`aspen-snix-bridge`) SHALL use `snix-tracing`. Other Aspen binaries SHALL retain their existing tracing setup.

#### Scenario: aspen-node unchanged

- **WHEN** `aspen-node` starts
- **THEN** it SHALL use Aspen's existing tracing initialization, NOT `snix-tracing`

### Requirement: clap integration

The `snix-tracing` clap arguments SHALL be integrated into `aspen-snix-bridge`'s CLI argument parser.

#### Scenario: Log level via CLI

- **WHEN** `aspen-snix-bridge --log-level debug` is invoked
- **THEN** the tracing subscriber SHALL filter at debug level
