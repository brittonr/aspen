## ADDED Requirements

### Requirement: Auto-register with service mesh on startup

When the `net` feature is enabled, the forge-web binary SHALL publish itself to the `aspen-net` service registry on startup using `NetPublish`. The service name SHALL be `forge-web`. The registration SHALL include the endpoint ID and TCP proxy port.

#### Scenario: Startup with net feature enabled

- **WHEN** the forge-web binary starts with the `net` feature compiled in and a valid cluster connection
- **THEN** the binary sends a `NetPublish` RPC to register `forge-web` as a service, logs success, and the service becomes discoverable at `forge-web.aspen` via SOCKS5/DNS

#### Scenario: Startup without net feature

- **WHEN** the forge-web binary is compiled without the `net` feature
- **THEN** no service registration is attempted and the binary operates as before

#### Scenario: Registration failure is non-fatal

- **WHEN** the forge-web binary starts with `net` enabled but the cluster node doesn't have the net handler loaded
- **THEN** the binary logs a warning and continues serving. All web pages remain functional.

### Requirement: Deregister on shutdown

When the `net` feature is enabled, the forge-web binary SHALL send `NetUnpublish` on graceful shutdown (SIGINT/SIGTERM). If shutdown is forced, stale registrations SHALL be handled by the service registry's TTL mechanism.

#### Scenario: Graceful shutdown

- **WHEN** the forge-web binary receives SIGINT (ctrl-c)
- **THEN** the binary sends `NetUnpublish` for `forge-web` before exiting, and the service is no longer discoverable

#### Scenario: Forced shutdown

- **WHEN** the forge-web process is killed with SIGKILL
- **THEN** the stale service entry expires via the registry's TTL (no cleanup by the binary)
