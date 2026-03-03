## ADDED Requirements

### Requirement: Daemon Lifecycle

The daemon SHALL orchestrate all service mesh components and connect to the cluster as a client.

#### Scenario: Daemon startup

- **WHEN** a user runs `aspen net up --ticket <cluster-ticket>`
- **THEN** the daemon SHALL:
  1. Create an iroh endpoint
  2. Connect to the cluster using the provided ticket
  3. Start the SOCKS5 proxy on the configured port (default: 1080)
  4. Start the MagicDNS resolver on the configured port (default: 5353) unless `--no-dns`
  5. Auto-publish any configured local services
  6. Begin watching the service registry for changes
- **AND** the daemon SHALL run in the foreground (or background with `--daemon`)

#### Scenario: Daemon shutdown

- **WHEN** the daemon receives SIGTERM or SIGINT
- **THEN** the daemon SHALL:
  1. Stop accepting new SOCKS5 connections
  2. Drain active connections with bounded timeout (`NET_SHUTDOWN_TIMEOUT_SECS`, 30s)
  3. Unpublish any auto-published services
  4. Stop the DNS resolver
  5. Close the iroh endpoint

#### Scenario: Daemon status

- **WHEN** a user runs `aspen net status`
- **THEN** the output SHALL show:
  - Connection state (connected/disconnected)
  - SOCKS5 proxy address and port
  - DNS resolver address and port (if running)
  - Number of active tunnels
  - Number of known services
  - Own endpoint ID and tags

### Requirement: Service Auto-Publishing

The daemon SHALL support auto-publishing local services on startup.

#### Scenario: Publish local services from config

- **GIVEN** the daemon is started with `--publish mydb:5432:tcp --tag db --tag prod`
- **WHEN** the daemon connects to the cluster
- **THEN** the service `mydb` SHALL be published with the daemon's endpoint ID, port `5432`, proto `tcp`, and tags `["db", "prod"]`

#### Scenario: Unpublish on shutdown

- **GIVEN** the daemon auto-published `mydb` on startup
- **WHEN** the daemon shuts down
- **THEN** the service `mydb` SHALL be unpublished from the registry

### Requirement: Registry Watching

The daemon SHALL maintain a local cache of the service registry for fast name resolution.

#### Scenario: Initial registry load

- **WHEN** the daemon connects to the cluster
- **THEN** it SHALL load all services from `/_sys/net/svc/` into a local cache

#### Scenario: Registry update detection

- **WHEN** a new service is published or an existing one is removed
- **THEN** the daemon SHALL detect the change within `NET_REGISTRY_POLL_INTERVAL_SECS` (10 seconds)
- **AND** update its local cache

#### Scenario: Revocation list refresh

- **WHEN** a token is revoked in the cluster's revocation store
- **THEN** the daemon SHALL detect the revocation within `NET_REVOCATION_POLL_INTERVAL_SECS` (60 seconds)
- **AND** subsequent authorization checks SHALL deny the revoked token

### Requirement: CLI Commands

#### Scenario: `aspen net up`

- **WHEN** a user runs `aspen net up --ticket <ticket> --token <capability-token> [--socks5-port 1080] [--dns-port 5353] [--no-dns] [--publish name:port:proto] [--tag tag] [--daemon]`
- **THEN** the daemon SHALL verify the token on startup
- **AND** start with the specified configuration

#### Scenario: `aspen net down`

- **WHEN** a user runs `aspen net down`
- **THEN** the running daemon SHALL be signaled to shut down gracefully

#### Scenario: `aspen net forward`

- **WHEN** a user runs `aspen net forward [bind_addr:]local_port:service[:remote_port] --ticket <ticket> --token <token>`
- **THEN** the token SHALL be verified for `NetConnect` capability
- **AND** a one-shot port forward SHALL be established (no daemon required)

#### Scenario: `aspen net publish`

- **WHEN** a user runs `aspen net publish <name> --port <port> [--proto tcp] [--tag <tag>]... --ticket <ticket> --token <token>`
- **THEN** the token SHALL be verified for `NetPublish` capability for the service name
- **AND** the service SHALL be published using the user's endpoint ID

#### Scenario: `aspen net unpublish`

- **WHEN** a user runs `aspen net unpublish <name> --ticket <ticket>`
- **THEN** the service SHALL be removed from the registry

#### Scenario: `aspen net services`

- **WHEN** a user runs `aspen net services [--tag <tag>] --ticket <ticket>`
- **THEN** all matching services SHALL be listed with their name, endpoint (short), port, proto, and tags

#### Scenario: `aspen net peers`

- **WHEN** a user runs `aspen net peers --ticket <ticket>`
- **THEN** all nodes with published services SHALL be listed with their endpoint ID (short), hostname, tags, and service count

#### Scenario: `aspen net dns set`

- **WHEN** a user runs `aspen net dns set <hostname>.aspen <ip> --ticket <ticket>`
- **THEN** the DNS override SHALL be stored

#### Scenario: `aspen net dns list`

- **WHEN** a user runs `aspen net dns list --ticket <ticket>`
- **THEN** all DNS overrides SHALL be listed
