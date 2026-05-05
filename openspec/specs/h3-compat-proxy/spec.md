# h3-compat-proxy Specification

## Purpose

Defines the H3 Compat Proxy capability requirements preserved by Aspen's archived OpenSpec records.

## Requirements

### Requirement: H3 proxy requirement 1

The proxy MUST listen on a configurable TCP address (default `127.0.0.1:8080`) and accept HTTP/1.1 requests.

#### Scenario: Requirement 1 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST listen on a configurable TCP address (default `127.0.0.1:8080`) and accept HTTP/1.1 requests.

### Requirement: H3 proxy requirement 2

The proxy MUST forward each HTTP request as an HTTP/3 request to the specified iroh endpoint ID over QUIC, using the configured ALPN for protocol negotiation.

#### Scenario: Requirement 2 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST forward each HTTP request as an HTTP/3 request to the specified iroh endpoint ID over QUIC, using the configured ALPN for protocol negotiation.

### Requirement: H3 proxy requirement 3

The proxy MUST preserve the request method, path, query string, headers, and body when forwarding.

#### Scenario: Requirement 3 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST preserve the request method, path, query string, headers, and body when forwarding.

### Requirement: H3 proxy requirement 4

The proxy MUST stream the h3 response body back to the TCP client without buffering the entire response in memory.

#### Scenario: Requirement 4 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST stream the h3 response body back to the TCP client without buffering the entire response in memory.

### Requirement: H3 proxy requirement 5

The proxy MUST return HTTP 502 Bad Gateway when the iroh endpoint is unreachable or the h3 request fails.

#### Scenario: Requirement 5 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST return HTTP 502 Bad Gateway when the iroh endpoint is unreachable or the h3 request fails.

### Requirement: H3 proxy requirement 6

The proxy MUST reconnect to the iroh endpoint automatically if the QUIC connection drops, with exponential backoff (1s, 2s, 4s, max 30s).

#### Scenario: Requirement 6 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST reconnect to the iroh endpoint automatically if the QUIC connection drops, with exponential backoff (1s, 2s, 4s, max 30s).

### Requirement: H3 proxy requirement 7

The proxy MUST support concurrent requests with multiple TCP connections handled simultaneously via multiplexed h3 streams on a single QUIC connection.

#### Scenario: Requirement 7 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST support concurrent requests with multiple TCP connections handled simultaneously via multiplexed h3 streams on a single QUIC connection.

### Requirement: H3 proxy requirement 8

The proxy MUST expose a library API (`H3Proxy::new`, `H3Proxy::run`) for embedding in other binaries without spawning a subprocess.

#### Scenario: Requirement 8 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST expose a library API (`H3Proxy::new`, `H3Proxy::run`) for embedding in other binaries without spawning a subprocess.

### Requirement: H3 proxy requirement 9

The proxy MUST accept `--endpoint-id`, `--alpn`, and `--port` as CLI arguments in the proxy binary.

#### Scenario: Requirement 9 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST accept `--endpoint-id`, `--alpn`, and `--port` as CLI arguments in the proxy binary.

### Requirement: H3 proxy requirement 10

The proxy MUST bind to localhost by default and require an explicit `--bind 0.0.0.0` flag for wildcard binding.

#### Scenario: Requirement 10 is enforced

- **WHEN** the h3 compatibility proxy handles traffic or configuration for this capability
- **THEN** it MUST bind to localhost by default and require an explicit `--bind 0.0.0.0` flag for wildcard binding.
