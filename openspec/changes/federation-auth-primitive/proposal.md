## Why

Federation exists as infrastructure (~8,600 LOC in `aspen-federation`) but is consumed only by Forge. The auth model is split: capability tokens (UCAN-inspired, prefix-scoped) handle intra-cluster authorization while a separate trust manager (Trusted/Public/Blocked) handles inter-cluster access with no fine-grained control. A "Trusted" cluster gets the same access as any other trusted cluster — there's no way to grant read access to your nix cache without also granting access to your forge repos.

This change unifies federation and auth into a single system-wide primitive. Federation becomes a publish/subscribe model over KV prefixes with UCAN-semantics capability tokens as the authorization mechanism — same token format for intra-cluster and inter-cluster auth, same verification logic, same delegation chains.

## What Changes

- **Federation subscriptions as the core primitive**: Any KV prefix can be published for federation. Remote clusters subscribe to specific prefixes. The federation layer syncs matching entries + associated blobs. Apps don't implement federation logic — they store data in KV and declare which prefixes are federable.
- **Unified auth via UCAN-semantics tokens**: `CapabilityToken` becomes the single auth mechanism for both local clients and federated clusters. Cluster keys (Ed25519) issue tokens to other clusters, scoped to specific KV prefixes. Same `TokenBuilder`, `TokenVerifier`, `Capability::contains()` — same verification path.
- **Self-contained credential presentation**: New `Credential` type bundles a token with its full proof chain (parent tokens ordered leaf-to-root). Enables offline verification of delegation chains across clusters without server-side cache lookups. This is the UCAN invocation model with Aspen's native encoding (postcard + BLAKE3, not JWT + CID).
- **Delegation chains across clusters**: Cluster A issues token to Cluster B. Cluster B delegates a narrower subset to Cluster C. Cluster C presents the full chain to Cluster A for verification. Capability attenuation enforced cryptographically at each delegation level.
- **Token-based federation lifecycle**: Short-lived tokens (24h default) with refresh protocol. Revocation = stop refreshing. Best-effort revocation gossip for urgent cases. `TrustManager` becomes a convenience layer over token state, not a separate auth system.
- **Federation handshake carries credentials**: `FederationRequest::Handshake` includes the `Credential`. Sync requests authorized against the credential's capabilities. Prefix-level access control replaces the coarse Trusted/Public/Blocked model.
- **`facts` field on CapabilityToken**: Arbitrary key-value metadata (UCAN "facts") for federation context — sync preferences, cluster metadata, subscription configuration.

## Capabilities

### New Capabilities

- `federation-subscription`: Publish/subscribe model for federating KV prefix ranges between clusters. Covers publishing prefixes with access settings, subscribing to remote prefixes, sync triggers (gossip notification or periodic poll), and the lifecycle of federation relationships.
- `federation-credential`: Self-contained UCAN-semantics credential type (`Credential`) that bundles a capability token with its full delegation proof chain for offline cross-cluster verification.
- `federation-token-lifecycle`: Token issuance, refresh, and revocation protocol for federation relationships. Short-lived tokens with automatic refresh, best-effort revocation gossip, and graceful expiry.

### Modified Capabilities

- `federation`: Auth model changes from TrustManager (Trusted/Public/Blocked) to capability-token-based access control. `TrustManager` becomes a convenience layer derived from active token state. Sync protocol handshake changes to include credentials. Per-prefix access replaces per-cluster trust levels.

## Impact

- **`aspen-auth`**: Add `Credential` type (token + proof chain), add `facts` field to `CapabilityToken`, `verify_with_chain()` becomes the primary federation verification path. ~200 LOC.
- **`aspen-federation`**: Modify `FederationRequest::Handshake` to include credential. Add federation subscription types (publish/subscribe declarations). Add token refresh request/response. Sync handler checks token capabilities before serving data. `TrustManager` refactored to derive from active credentials. ~800 LOC.
- **`aspen-federation/sync`**: Authorization gate on `ListResources`, `GetResourceState`, `SyncObjects` — check credential capabilities against requested prefix. ~150 LOC.
- **`aspen-client-api`**: New RPC variants for federation token management (issue, refresh, revoke, list subscriptions). ~200 LOC.
- **`aspen-cli`**: `federation grant`, `federation delegate`, `federation subscribe`, `federation status` commands. ~300 LOC.
- **`aspen-core`**: Federation subscription types may live here if they become a core primitive referenced by multiple crates.
- **Wire compatibility**: Federation protocol version bump (handshake now carries credentials). Existing clusters without tokens fall back to TrustManager behavior during migration.
