# Aspen Hook Trigger URL Design

**Date**: 2026-01-08
**Status**: Implemented
**Author**: Claude (Ultra Mode Analysis)

## Executive Summary

This document analyzes how to enable external programs to trigger Aspen hooks via a shareable "Aspen Iroh URL". The solution leverages Aspen's existing Iroh-based Client RPC infrastructure with a new URL format that encodes connection information and hook trigger parameters.

## Problem Statement

External programs need a way to trigger Aspen hooks without:

1. Hard-coding connection details
2. Managing complex Iroh connection setup
3. Understanding Aspen's internal RPC protocol

The goal is to provide a simple, shareable URL that encapsulates everything needed to trigger a hook.

## Existing Infrastructure Analysis

### Current Hook Triggering Mechanisms

1. **Internal Event Bridge** (`hooks_bridge.rs`): Automatically converts committed Raft log entries to hook events
2. **Manual CLI Trigger**: `aspen-cli hooks trigger <event_type> --payload '{}'`
3. **Client RPC**: `ClientRpcRequest::HookTrigger { event_type, payload_json }`

### Current URL/Ticket Formats

| Format | Prefix | Purpose |
| ------ | ------ | ------- |
| `AspenClusterTicket` | `aspen` | Gossip-based cluster discovery |
| `SignedAspenClusterTicket` | `aspensigned` | Authenticated cluster tickets |
| `AspenClientTicket` | `aspenclient` | Client access with auth tokens |
| `AspenDocsTicket` | `aspendocs` | CRDT document sync |
| `DnsClientTicket` | `aspendns` | DNS zone sync |

### Client RPC Protocol

- **ALPN**: `b"aspen-client"` (CLIENT_ALPN)
- **Serialization**: postcard binary format
- **Connection**: Iroh QUIC with NAT traversal
- **Authentication**: Optional capability tokens (HMAC-based)

### HookTrigger Request Structure

```rust
ClientRpcRequest::HookTrigger {
    event_type: String,  // e.g., "write_committed", "delete_committed"
    payload_json: String, // JSON payload as string
}
```

**Supported Event Types**:

- `write_committed` - KV write operations
- `delete_committed` - KV delete operations
- `leader_elected` - New Raft leader
- `membership_changed` - Cluster membership change
- `node_added` / `node_removed` - Node lifecycle
- `snapshot_created` / `snapshot_installed` - Snapshot operations

## Proposed Solution: AspenHookTicket

### URL Format

```
aspenhook<base32-encoded-payload>
```

### Ticket Structure

```rust
/// Aspen hook trigger ticket for external programs.
///
/// Encapsulates everything needed to trigger a hook on a remote Aspen cluster:
/// - Connection information (bootstrap peers)
/// - Authentication (optional capability token)
/// - Hook configuration (event type, default payload)
///
/// Tiger Style: Bounded fields, fail-fast validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AspenHookTicket {
    /// Protocol version for forward compatibility.
    pub version: u8,

    /// Cluster identifier (human-readable).
    pub cluster_id: String,

    /// Bootstrap peers for connection (max 16).
    pub bootstrap_peers: BTreeSet<EndpointId>,

    /// Event type to trigger (e.g., "write_committed").
    pub event_type: String,

    /// Default payload template (JSON string).
    /// Programs can override this when triggering.
    pub default_payload: Option<String>,

    /// Optional authentication token for authorized triggers.
    pub auth_token: Option<[u8; 32]>,

    /// Ticket expiration (Unix timestamp, seconds).
    pub expires_at_secs: Option<u64>,

    /// Optional relay URL for NAT traversal.
    pub relay_url: Option<String>,
}
```

### Usage Examples

**Generating a Hook URL**:

```bash
# Via CLI (new command)
aspen-cli hooks create-url \
    --event-type write_committed \
    --payload '{"source": "external"}' \
    --expires 24h \
    --auth-token <token>

# Output: aspenhook7g2wc...agd6q
```

**Using the Hook URL** (external program):

```rust
// Minimal external client usage
use aspen_hook_client::HookClient;

let url = "aspenhook7g2wc...agd6q";
let client = HookClient::from_url(url)?;

// Trigger with default payload
client.trigger().await?;

// Trigger with custom payload
client.trigger_with_payload(json!({"key": "value"})).await?;
```

**Shell Integration** (for scripts):

```bash
# Using aspen-cli with URL
aspen-cli hooks trigger-url "aspenhook7g2wc...agd6q" \
    --payload '{"custom": "data"}'

# Or via environment variable
ASPEN_HOOK_URL="aspenhook7g2wc...agd6q"
aspen-cli hooks trigger-url "$ASPEN_HOOK_URL"
```

## Architecture

```
External Program
      |
      | (1) Parse aspenhook ticket
      v
AspenHookTicket
      |
      | (2) Create Iroh endpoint
      v
Iroh QUIC Connection
      |
      | (3) Connect to bootstrap peer (CLIENT_ALPN)
      v
Aspen Node ClientProtocolHandler
      |
      | (4) Send HookTrigger request
      v
HookHandler (RPC)
      |
      | (5) Dispatch to HookService
      v
Hook Handlers (InProcess, Shell, Forward)
```

## Implementation Plan

### Phase 1: Core Ticket Type (Priority: HIGH)

**Files to create/modify**:

1. **New file**: `crates/aspen-hooks/src/ticket.rs`
   - `AspenHookTicket` struct
   - Serialization/deserialization (iroh-tickets pattern)
   - Validation logic

2. **Modify**: `crates/aspen-hooks/src/lib.rs`
   - Export ticket module

### Phase 2: CLI Commands (Priority: HIGH)

**Files to modify**:

1. **Modify**: `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs`
   - Add `create-url` subcommand
   - Add `trigger-url` subcommand

### Phase 3: Lightweight Client Library (Priority: MEDIUM)

**Files to create**:

1. **New crate**: `crates/aspen-hook-client/`
   - Minimal async client for external programs
   - Zero-config connection from URL
   - Optional: synchronous wrapper for non-async contexts

### Phase 4: Signed Tickets (Priority: LOW)

**Files to create/modify**:

1. **New file**: `crates/aspen-hooks/src/ticket.rs`
   - `SignedAspenHookTicket` for authenticated URLs
   - Ed25519 signature verification
   - Expiration enforcement

## Security Considerations

### Authentication Levels

1. **Unauthenticated URLs** (default):
   - Anyone with the URL can trigger the hook
   - Rate-limited by distributed rate limiter
   - Suitable for low-risk hooks (logging, metrics)

2. **Token-authenticated URLs**:
   - HMAC capability token required
   - Validated by ClientProtocolHandler
   - Suitable for sensitive hooks (state changes)

3. **Signed URLs** (future):
   - Ed25519 signature by cluster member
   - Time-limited validity
   - Suitable for untrusted environments

### Rate Limiting

Hook triggers via URL are subject to the same rate limiting as CLI triggers:

- **Exempt from distributed rate limiter** (admin operations)
- **Bounded by hook system limits**:
  - `MAX_CONCURRENT_HANDLER_EXECUTIONS = 32`
  - `MAX_HANDLER_TIMEOUT_MS = 30,000`

### Mitigations

| Threat | Mitigation |
| ------ | ---------- |
| URL sharing/leakage | Short expiration, token auth, signed tickets |
| Replay attacks | Nonce in signed tickets, expiration |
| DoS via hook spam | Rate limiting, bounded concurrency |
| Information disclosure | Minimal error details in responses |

## Alternative Approaches Considered

### 1. HTTP Webhook Endpoint

**Rejected**: Violates Aspen's Iroh-first architecture. Would require adding axum/HTTP server, increasing attack surface and complexity.

### 2. gRPC Service

**Rejected**: Adds protobuf dependency, doesn't leverage existing Iroh infrastructure.

### 3. Custom Binary Protocol

**Rejected**: Reinvents wheel when postcard + Iroh already provides efficient binary RPC.

### 4. Environment Variable with Connection String

**Considered**: Less portable than URL, harder to share. Could be added as complementary option.

## Compatibility

### Existing Code

- **No breaking changes** to existing hook system
- New URL format is additive
- Existing `aspen-cli hooks trigger` continues to work

### Version Migration

- Version field in ticket allows future protocol changes
- Unknown versions fail-fast with clear error

## Testing Strategy

### Unit Tests

- Ticket serialization roundtrip
- Validation edge cases (empty fields, max lengths)
- Expiration logic

### Integration Tests

- End-to-end URL creation and triggering
- Multi-node cluster URL resolution
- Authentication token validation

### Property Tests

- Arbitrary valid tickets serialize/deserialize correctly
- Invalid inputs rejected consistently

## Open Questions

1. **Should we support batch triggering in a single URL?**
   - Pro: Efficiency for multi-event workflows
   - Con: Complexity, harder to reason about

2. **Should the client library be a separate crate?**
   - Pro: Minimal dependency footprint for external programs
   - Con: More crates to maintain

3. **Should we add DNS-SD discovery for hook endpoints?**
   - Pro: Auto-discovery in local networks
   - Con: Security implications, complexity

## Success Metrics

- External programs can trigger hooks with single URL
- Connection establishment < 500ms on first use
- No HTTP dependencies required
- Works across NAT boundaries via Iroh relay

## Implementation Status

### Completed

- **Phase 1**: `AspenHookTicket` type implemented in `crates/aspen-hooks/src/ticket.rs`
- **Phase 2**: CLI commands implemented in `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs`
  - `aspen-cli hooks create-url` - Generate shareable hook trigger URLs
  - `aspen-cli hooks trigger-url` - Trigger hooks using a URL
- **Phase 3**: `aspen-hook-client` crate created at `crates/aspen-hook-client/`

### Files Created/Modified

| File | Description |
| ---- | ----------- |
| `crates/aspen-hooks/src/ticket.rs` | AspenHookTicket struct and serialization |
| `crates/aspen-hooks/src/lib.rs` | Export ticket module |
| `crates/aspen-hooks/Cargo.toml` | Added iroh, iroh-tickets, postcard dependencies |
| `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs` | CreateUrl and TriggerUrl commands |
| `crates/aspen-cli/Cargo.toml` | Added aspen-hooks dependency |
| `crates/aspen-hook-client/` | New lightweight client crate |
| `Cargo.toml` | Added aspen-hook-client to workspace |

### Test Results

- 63 tests passing in aspen-hooks (including 13 new ticket tests)
- 2 tests passing in aspen-hook-client
- Full workspace builds successfully

## Next Steps

1. Gather feedback from initial usage
2. Implement signed tickets (Phase 4) based on demand
3. Add end-to-end integration tests with live cluster
