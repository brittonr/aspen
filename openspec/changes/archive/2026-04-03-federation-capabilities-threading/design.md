## Context

Federation handshake already exchanges capabilities and credentials at the wire level. The handler verifies credentials and stores them in `session_credential`. But the client side throws away the response capabilities in `connect_to_cluster()` and no caller ever passes a credential. The plumbing exists end-to-end; it's just not connected.

## Goals / Non-Goals

**Goals:**

- Thread actual peer capabilities from handshake response through `ConnectResult`
- Consolidate `connect_to_cluster` and `connect_to_cluster_full` into one function
- Wire credential lookup into federation connection paths (orchestrator + forge handlers)
- Gate operations on peer capabilities (e.g., skip streaming-sync for peers that don't support it)

**Non-Goals:**

- New capability types beyond what's already defined
- Changes to the wire format or handshake protocol
- Credential issuance or management (already handled by `aspen-auth` + CLI)
- Per-request credential refresh (existing `RefreshToken` request handles that)

## Decisions

### 1. Merge `connect_to_cluster` into returning `ConnectResult`

Change `connect_to_cluster()` to return `Result<ConnectResult>` directly, extracting capabilities from the `FederationResponse::Handshake` variant. Remove `connect_to_cluster_full()`.

*Alternative*: Keep both functions. Rejected because `connect_to_cluster_full` is just a wrapper with hardcoded capabilities — no reason for two entry points.

### 2. Credential lookup via KV prefix `_sys:fed:token:received:<cluster_key>`

The orchestrator and forge handlers already have access to the KV store. Before connecting, scan for a stored credential at the well-known key. This matches the existing spec in `federation-auth-enforcement`.

*Alternative*: Pass credentials from the caller. Rejected because callers don't have credential context — the orchestrator/handler is the right place to look it up.

### 3. Callers destructure `ConnectResult` at the call site

Sites that only need `(connection, identity)` destructure with `let ConnectResult { connection, identity, .. } = ...`. No adapter functions needed.

### 4. Capability-gated operations use `has_capability()`

Before attempting streaming-sync or other optional protocols, check `result.has_capability("streaming-sync")`. Fall back gracefully when the peer doesn't advertise the capability.

## Risks / Trade-offs

- **[Signature change breadth]** → 10 call sites need updating. Mechanical refactor, but each site needs review for credential availability. Mitigated by doing client.rs first, then updating callers one file at a time.
- **[KV lookup latency on connect]** → One extra KV read per federation connection. Federation connections are infrequent (sync intervals are minutes/hours), so this is negligible.
- **[Old peers without capabilities]** → Peers running older code may not send capabilities in the response. The handshake response already has `capabilities: Vec<String>` with `#[serde(default)]` — an empty vec is fine, and callers gate on `has_capability()`.
