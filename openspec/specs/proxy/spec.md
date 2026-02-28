# Proxy Specification

## Purpose

Reverse proxy enabling external HTTP/HTTPS traffic to reach services running within Aspen clusters. Routes traffic based on configuration stored in the Raft KV store.

## Requirements

### Requirement: Reverse Proxy Routing

The system SHALL route incoming HTTP requests to backend services based on host and path matching rules stored in the KV store.

#### Scenario: Route by hostname

- GIVEN a proxy rule mapping `api.example.com` to service `"api-backend"`
- WHEN a request arrives with `Host: api.example.com`
- THEN the request SHALL be forwarded to the `"api-backend"` service

#### Scenario: Route by path prefix

- GIVEN a proxy rule mapping `/api/v1/*` to service `"api-v1"`
- WHEN a request arrives for `/api/v1/users`
- THEN the request SHALL be forwarded to `"api-v1"`

### Requirement: Dynamic Configuration

The system SHALL reload proxy routing rules when the underlying KV entries change, without requiring a restart.

#### Scenario: Add route at runtime

- GIVEN the proxy is running with existing routes
- WHEN a new route is added to the KV store
- THEN the proxy SHALL begin routing to the new backend without restart
