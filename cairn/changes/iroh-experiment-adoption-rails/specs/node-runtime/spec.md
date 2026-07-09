# Node Runtime Delta: Optional HTTP3 Iroh Readback Adapter

## ADDED Requirements

### Requirement: HTTP3-over-Iroh readback adapter is optional and non-authority
r[molten.node_runtime.http3_iroh_readback_adapter] Molten MAY provide an optional HTTP3-over-Iroh adapter for operator readback, but the adapter MUST translate requests into canonical Molten gateway read, range, or index requests and MUST treat the gateway receipt as the normative evidence.

#### Scenario: HTTP readback delegates to canonical gateway request
- GIVEN an admitted HTTP3-over-Iroh session asks to read a visible artifact range
- WHEN the adapter handles the request
- THEN it builds the same canonical operator gateway range request used by non-HTTP paths
- AND the response is emitted only after the gateway range receipt passes.

#### Scenario: HTTP session does not grant authority
- GIVEN a remote client has an HTTP3-over-Iroh connection, route, header, or endpoint identity
- WHEN the client requests protected, hidden, destructive, or authority-scoped content
- THEN the adapter requires the normal capability, policy, resource, retention, redaction, and gateway visibility evidence
- AND the HTTP session evidence alone is insufficient.

#### Scenario: HTTP adapter cannot bypass Preserves identity
- GIVEN an HTTP route maps to a Molten artifact or chunk manifest
- WHEN Molten evaluates readback
- THEN canonical Preserves or chunk manifest refs remain the identity boundary
- AND HTTP paths, status codes, headers, and rendered MIME metadata are non-normative views.

### Requirement: HTTP3-over-Iroh adapter tests cover pass and denial
r[molten.node_runtime.http3_iroh_readback_tests] Molten SHOULD cover the optional HTTP3-over-Iroh adapter with positive tests for admitted read-only gateway requests and negative tests for unauthorized routes, hidden refs, unsupported methods, oversized requests, malformed ranges, and attempts to use HTTP transport as authority.

#### Scenario: Unauthorized HTTP route denies before read
- GIVEN an HTTP3-over-Iroh request targets a protected artifact without reveal or visibility evidence
- WHEN the adapter translates the request
- THEN the canonical gateway gate denies
- AND the adapter returns only diagnostic readback without exposing protected bytes.

#### Scenario: Unsupported method cannot mutate state
- GIVEN an HTTP3-over-Iroh request uses a method intended to pin, delete, install, execute, or mutate policy
- WHEN the optional readback adapter validates the request
- THEN it denies before invoking any destructive or privileged subsystem
- AND emits diagnostics that the adapter is read-only.
