## Context

Federation sync has three layers of auth today:

1. **TrustManager** — in-memory Trusted/Public/Blocked per cluster key. Manual `add_trusted()` or credential-derived. Not persisted across restarts.
2. **Per-resource FederationSettings** — Public/AllowList/Disabled mode per FederatedId. Stored in KV.
3. **Credential in handshake** — the wire protocol accepts an optional `Credential` in the Handshake request, and `handle_push_objects` checks `has_credential` as a boolean.

The gap: the sync client always sends `credential: None`. The push handler checks credential _presence_ but not _contents_. There's no way to say "cluster B can pull repos matching `forge:*` but not push" or "this token expires in 48 hours."

The `aspen-auth` crate already has the full token machinery — `TokenBuilder`, `Credential`, delegation chains, `Capability::authorizes()`, and `Capability::contains()` for attenuation. The federation tests (`federation_auth_test.rs`) prove the token→credential→verify pipeline works. We just need to thread credentials through the sync path and check them properly.

## Goals / Non-Goals

**Goals:**

- Sync client presents stored credentials during federation handshake
- Push and pull handlers verify credential capabilities match the operation and target resource
- New `FederationPull` / `FederationPush` capability variants scoped by repo prefix
- CLI `federation grant` issues tokens; `federation token list` / `federation token inspect` for management
- Credentials stored in KV (`_sys:fed:token:<audience_key>`) and loaded automatically for outbound connections

**Non-Goals:**

- Automatic token exchange protocol (clusters negotiate trust automatically) — future work
- Gossip-based revocation propagation — existing `RevocationStore` covers local revocation
- Per-ref granularity in capabilities — repo-prefix level is sufficient for now
- Mutual authentication (both sides present credentials) — one-directional is enough; the receiver's identity is already verified via Iroh's QUIC handshake

## Decisions

### 1. New capability variants over reusing KV prefix capabilities

Add `FederationPull { repo_prefix }` and `FederationPush { repo_prefix }` to `Capability` enum rather than encoding federation operations as KV `Read`/`Write` on `_sys:fed:` prefixes.

**Rationale**: KV prefix encoding is indirect — `Read { prefix: "_sys:forge:repo:" }` doesn't obviously mean "pull git objects." Explicit variants are self-documenting, enforce correct authorization checks in the handler, and allow `contains()` attenuation to work naturally (a `FederationPull { repo_prefix: "" }` contains `FederationPull { repo_prefix: "my-org/" }`).

**Alternative**: Reuse existing `Read`/`Write` capabilities. Rejected because it conflates KV data access with federation sync semantics.

### 2. Credential stored per-audience in KV

Store issued and received credentials in KV at `_sys:fed:token:issued:<audience_hex>` and `_sys:fed:token:received:<issuer_hex>`. The sync orchestrator loads the received credential for the target cluster before connecting.

**Rationale**: KV is already replicated via Raft, so credentials survive node restarts and leader changes. The audience/issuer key structure makes lookups O(1).

**Alternative**: Store in TrustManager memory. Rejected because TrustManager is in-memory only and loses state on restart.

### 3. Client sends credential in existing Handshake message

The `FederationRequest::Handshake` already has `credential: Option<Credential>`. The client just needs to populate it. No wire protocol changes required.

### 4. Handler checks capabilities per-operation, not just presence

`handle_push_objects` currently checks `has_credential` as a boolean. Change to: extract the credential, check it has a `FederationPush` capability whose `repo_prefix` matches the target `FederatedId`. Same pattern for pull via `check_resource_access`.

### 5. `connect_to_cluster` takes optional Credential parameter

Add `credential: Option<Credential>` to the function signature. The orchestrator passes it; tests can pass `None` for backwards compatibility.

## Risks / Trade-offs

- **[Clock skew]** → Token expiry already accounts for clock skew tolerance in `aspen-auth`. No new risk.
- **[Credential size in handshake]** → Delegation chains up to depth 8 could add ~4KB to handshake. Acceptable given QUIC framing. → `MAX_DELEGATION_DEPTH = 8` bounds this.
- **[Breaking change to `connect_to_cluster`]** → Adding a parameter is a breaking API change. → Internal API, no external consumers. Callers are in `orchestrator.rs` and tests.
- **[Stale credentials in KV]** → Expired tokens sit in KV. → Add a cleanup scan on node startup or periodic timer. Non-blocking for initial implementation.
