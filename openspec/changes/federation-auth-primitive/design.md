## Context

Aspen has two parallel auth systems that don't interact. Intra-cluster: `CapabilityToken` with prefix-scoped capabilities, delegation chains, offline verification via `TokenVerifier`. Inter-cluster: `TrustManager` with three coarse levels (Trusted/Public/Blocked) plus per-resource `FederationSettings` (Public/AllowList/Disabled). The federation sync protocol (`aspen-federation/sync`) handles data transfer but authorization is a binary trust check — no prefix-level granularity.

Federation infrastructure (~8,600 LOC) is generic but only consumed by Forge. Other subsystems (nix cache, CI artifacts, automerge docs) store state in KV but have no federation path.

The existing `CapabilityToken` already implements UCAN semantics: delegation chains via `proof` hash references, capability attenuation via `Capability::contains()`, chain verification via `verify_with_chain()`, trusted roots, revocation, audience binding. The gap is a self-contained presentation format and wiring into the federation protocol.

## Goals / Non-Goals

**Goals:**

- Unify intra-cluster and inter-cluster auth into a single capability token model
- Enable any KV prefix to be federated without app-specific federation code
- Support offline verification of cross-cluster delegation chains (no server-side state needed)
- Provide prefix-level access control for federation (not just per-cluster trust)
- Maintain backward compatibility during rollout (clusters without tokens fall back to trust levels)

**Non-Goals:**

- Full UCAN wire format (JWT + DIDs + CIDs) — adopt semantics, keep postcard + Ed25519 + BLAKE3
- Federated writes (remote clusters pushing data) — federation remains pull-based / read-only
- CRDT/automerge doc federation — iroh-docs handles its own sync; this covers KV + blob federation
- Real-time streaming sync — poll-based and gossip-triggered, not continuous replication
- Cross-cluster coordination primitives (locks, barriers, elections) — these are cluster-local by nature

## Decisions

### D1: Self-contained `Credential` type for federation auth

**Decision**: Introduce `Credential { token: CapabilityToken, proofs: Vec<CapabilityToken> }` as the presentation format for cross-cluster auth.

**Rationale**: The existing `verify_with_chain()` already accepts a token + chain slice. The missing piece is a wire type that carries the chain alongside the token. This is the UCAN invocation model. The presenter bundles the leaf token with all parent tokens back to the root. The verifier walks the chain using pure crypto — no cache lookups, no network calls.

**Alternatives considered**:

- *Server-side parent cache only*: Current `register_parent_token()` approach. Breaks for federation — Cluster A has never seen tokens issued by Cluster B to Cluster C.
- *Hash-only proofs with lazy fetch*: Token carries parent hashes, verifier fetches parents on demand. Requires network, breaks offline verification.

**Wire size**: Bounded by `MAX_DELEGATION_DEPTH` (8). Each token is ~200-400 bytes postcard-encoded. Worst case chain: ~3KB. Acceptable for handshake messages.

### D2: Federation subscriptions as publish/subscribe over KV prefixes

**Decision**: Federation is modeled as:

- **Publish side**: Cluster declares KV prefixes available for federation, each with access policy (public, allowlist, token-required).
- **Subscribe side**: Remote cluster declares which prefixes to sync from which sources, with sync trigger (periodic or gossip-driven).

**Rationale**: KV prefixes are the natural federation boundary. Every app already stores state under a known prefix (`_sys:nix-cache:`, `_forge:repos:`, `_ci:artifacts:`). Federating at the prefix level means apps get federation by declaring their prefix — no per-app sync protocol needed. The sync engine doesn't need to understand what the keys mean.

**Data model**:

```
Published prefix (stored in KV at _sys:fed:pub:{prefix_hash}):
  prefix:       "_sys:nix-cache:narinfo:"
  access:       Public | TokenRequired
  policy:       ResourcePolicy (quorum, verification, fork detection)
  announced_at: HLC timestamp

Subscription (stored in KV at _sys:fed:sub:{source_key}:{prefix_hash}):
  source:       PublicKey (cluster key)
  prefix:       "_sys:nix-cache:narinfo:"
  credential:   Credential (token + proof chain)
  sync_mode:    Periodic(interval_secs) | OnGossip
  last_sync:    HLC timestamp
  cursor:       Option<String> (pagination state)
```

**Alternatives considered**:

- *Federated namespace grouping*: Group multiple prefixes into named namespaces. Adds indirection without clear benefit — prefixes already serve as natural boundaries.
- *Per-resource FederatedId only*: Current Forge model. Requires each app to implement federation. Doesn't scale to "everything federates."

### D3: Cluster keys as token issuers for federation

**Decision**: Federation tokens are issued using the cluster's long-lived Ed25519 key (same `ClusterIdentity` keypair used for signing announcements). The `TokenBuilder` already accepts any `SecretKey`. The `TokenVerifier` uses `trusted_roots` = [our cluster key] for federation verification.

**Rationale**: Node keys are ephemeral (per-node, change on restart). Cluster keys are long-lived and shared across all nodes. Federation relationships outlive individual nodes. The issuer identity must be stable.

**Key hierarchy**:

```
Cluster key (long-lived, shared)
  └── issues federation tokens (audience: remote cluster key)
       └── remote cluster delegates subset (audience: third cluster key)

Node key (ephemeral, per-node)
  └── issues local client tokens (audience: client node key or bearer)
```

Both key types are Ed25519 / iroh PublicKey. Same `TokenBuilder`, same `TokenVerifier`. Different issuer scope.

### D4: Short-lived tokens with refresh protocol

**Decision**: Federation tokens have a default 24h expiry. Active federation relationships auto-refresh tokens during sync sessions. Revocation = stop refreshing. Best-effort revocation gossip for urgent cases via existing gossip infrastructure.

**Rationale**: Distributed revocation is a famously hard problem (CRLs, OCSP). Short-lived tokens sidestep it — a revoked token becomes useless within 24h without any distributed coordination. The `RevocationStore` provides faster revocation when clusters are connected, but correctness doesn't depend on it.

**Refresh protocol**:

```
FederationRequest::RefreshToken {
    credential: Credential,    // current (near-expiry) credential
}
FederationResponse::TokenRefreshed {
    token: CapabilityToken,    // fresh token, same capabilities
}
```

Issuing cluster validates the current credential, issues a fresh token with the same capabilities and a new expiry. Refuses if the relationship has been revoked.

**Alternatives considered**:

- *Long-lived tokens + CRL*: Simpler issuance but requires distributed revocation list. Propagation delay = security window.
- *Very short tokens (1h) + constant refresh*: More secure but generates constant traffic. 24h balances security and efficiency.

### D5: TrustManager as convenience layer, not auth authority

**Decision**: `TrustManager` remains but becomes derived from active credential state. "Trusted" = "has a valid, non-expired credential for this cluster." "Blocked" = "revoked or explicitly blocked." "Public" = "unknown, no credential."

**Rationale**: The trust manager provides useful UX — operators want to see "who do I trust" as a list, not "what tokens are active." But authorization decisions flow through `TokenVerifier.authorize()`, not `TrustManager.can_access_resource()`.

**Migration**: During rollout, clusters without token support continue using the existing trust model. The federation handshake accepts either a credential or a legacy trust-based handshake. The sync handler checks credentials first, falls back to `TrustManager.can_access_resource()` if no credential is present. This allows incremental adoption.

### D6: Add `facts` field to CapabilityToken

**Decision**: Add `pub facts: Vec<(String, Vec<u8>)>` to `CapabilityToken` with `#[serde(default)]` for backward compatibility.

**Rationale**: UCAN "facts" carry arbitrary claims that aren't capabilities. For federation: sync preferences (preferred sync interval, bandwidth limits), cluster metadata (region, available storage), subscription context. These travel with the token but don't affect authorization — they're informational.

**Wire compatibility**: `#[serde(default)]` means old tokens without `facts` deserialize with an empty vec. No version bump needed for the token format.

### D7: Authorization gate on sync requests

**Decision**: Every sync request (`ListResources`, `GetResourceState`, `SyncObjects`) is authorized against the credential established during handshake. The handler extracts the `Credential` from the session, calls `TokenVerifier.authorize(token, Operation::Read{key: requested_prefix}, presenter)`.

**Rationale**: The existing `Capability::authorizes()` and `Operation` enum already support prefix matching for Read operations. A `SyncObjects` request for `_sys:nix-cache:narinfo:` maps to `Operation::Read{key: "_sys:nix-cache:narinfo:"}`. The token's `Read{prefix: "_sys:nix-cache:"}` authorizes it via `starts_with`. No new authorization logic needed.

## Risks / Trade-offs

**[Risk] 24h revocation window** → Acceptable for federation use cases. If urgent revocation is needed, gossip propagates revocation hashes to connected clusters within seconds. The 24h window only applies to disconnected clusters. For high-security scenarios, operators can configure shorter token lifetimes.

**[Risk] Credential size in handshake** → Bounded by `MAX_DELEGATION_DEPTH` (8). Worst case ~3KB. Handshakes happen once per sync session, not per request. Negligible compared to actual data transfer.

**[Risk] Clock skew across clusters** → Token expiry verification uses `TOKEN_CLOCK_SKEW_SECS` tolerance (existing). Federation between clusters with wildly different clocks will have auth failures. Mitigation: HLC timestamps for ordering, clock skew tolerance for token verification.

**[Risk] Backward compatibility during migration** → Clusters without token support must still federate. The handshake fallback (credential → trust manager) handles this, but creates a period where both auth paths are active. Mitigation: Feature flag `federation-tokens` controls whether tokens are required or optional. Once all clusters upgrade, flip to required.

**[Trade-off] UCAN semantics without UCAN wire format** → Gives up interoperability with UCAN-native systems (IPFS ecosystem, Fission). Gains: postcard compactness, no DID/CID dependencies, consistent with Aspen's iroh-native encoding. A translation layer can be added later if interop is needed.

**[Trade-off] Prefix-level federation vs. resource-level** → Prefix-level is coarser than per-resource `FederatedId`. A token granting `Read{prefix: "_forge:repos:"}` grants access to ALL repos, not a specific one. Mitigation: Issue narrower tokens (`Read{prefix: "_forge:repos:aspen:"}`) for fine-grained control. The `Capability::contains()` attenuation model supports arbitrary prefix specificity.

## Open Questions

- **Subscription persistence across restarts**: Should subscription state survive node restarts? If stored in KV (proposed), it's Raft-replicated and survives. But the credential (containing tokens) would also be in KV — is that an acceptable security posture for token storage?
- **Multi-prefix tokens**: A single token with `[Read{prefix:"_nix:"}, Read{prefix:"_forge:public:"}]` — should this create one subscription or two? Likely two (different sync configurations per prefix), but the credential is shared.
- **Gossip topic for revocations**: Use the existing federation gossip topic or a dedicated revocation topic? Dedicated topic avoids rate limiting interference with normal federation gossip.
