## 1. Credential Type and Token Extensions

- [x] 1.1 Add `facts: Vec<(String, Vec<u8>)>` field to `CapabilityToken` in `aspen-auth/src/token.rs` with `#[serde(default)]`. Update `bytes_to_sign()` to include facts in signature computation. Add tests for backward-compatible deserialization of tokens without facts.
- [x] 1.2 Create `Credential` type in `aspen-auth/src/credential.rs` — struct with `token: CapabilityToken` and `proofs: Vec<CapabilityToken>`. Implement postcard `Serialize`/`Deserialize`, `encode()`/`decode()`, `to_base64()`/`from_base64()`. Add size bound: `MAX_DELEGATION_DEPTH * MAX_TOKEN_SIZE`.
- [x] 1.3 Add `Credential::verify()` method that calls `TokenVerifier::verify_with_chain()` internally, extracting the proofs slice from self. Add `Credential::from_root(token)` constructor for depth-0 tokens with empty proofs.
- [x] 1.4 Add `Credential::delegate()` method that creates a child credential: takes a `SecretKey` (new issuer), `audience`, `capabilities` (must be subset), and `lifetime`. Returns a new `Credential` with the child token + parent chain appended to proofs.
- [x] 1.5 Re-export `Credential` from `aspen-auth/src/lib.rs`. Add unit tests: root credential verification, 2-level chain, 3-level chain, max depth chain, broken chain rejection, capability escalation rejection, expired token in chain rejection.

## 2. Federation Subscription Types

- [x] 2.1 Create `aspen-federation/src/subscription.rs` with core types: `PublishedPrefix { prefix, access_policy, resource_policy, announced_at_hlc }`, `AccessPolicy` enum (`Public`, `TokenRequired`), `Subscription { source, prefix, credential, sync_mode, last_sync_hlc, cursor }`, `SyncMode` enum (`Periodic { interval_secs: u32 }`, `OnGossip`).
- [x] 2.2 Add KV key derivation functions: `pub_key(prefix) -> "_sys:fed:pub:{blake3(prefix)[..16]_hex}"` and `sub_key(source, prefix) -> "_sys:fed:sub:{source_key_hex[..16]}:{blake3(prefix)[..16]_hex}"`. Add constants for key prefixes. Add tests for key derivation determinism.
- [x] 2.3 Add `PublishedPrefix` and `Subscription` serialization via serde_json for KV storage. Add store/load helper functions that read/write from a `KeyValueStore` trait object. Add tests for round-trip serialization.
- [x] 2.4 Re-export subscription types from `aspen-federation/src/lib.rs`.

## 3. Federation Handshake with Credentials

- [x] 3.1 Add `credential: Option<Credential>` field to `FederationRequest::Handshake` in `aspen-federation/src/sync/types.rs`. Use `#[serde(default)]` for backward compatibility.
- [x] 3.2 Add `credential: Option<Credential>` to `FederationProtocolContext` (or equivalent session state) so verified capabilities persist across requests on the same connection.
- [x] 3.3 Update `FederationProtocolHandler` handshake processing in `aspen-federation/src/sync/handler.rs`: if credential present, verify via `Credential::verify()` with `trusted_roots = [local_cluster_key]`. Store verified capabilities in session. If no credential, fall back to `TrustManager` check. Log deprecation warning for legacy handshakes.
- [x] 3.4 Add tests: handshake with valid credential, handshake with expired credential (rejected), handshake with escalated chain (rejected), handshake without credential (legacy fallback).

## 4. Authorization Gate on Sync Requests

- [x] 4.1 Add authorization check in `handle_list_resources()` in `aspen-federation/src/sync/handler.rs`: extract session credential, check `TokenVerifier.authorize(token, Operation::Read{key: requested_prefix}, presenter)`. Reject with `FederationResponse::Error` if unauthorized.
- [x] 4.2 Add authorization check in `handle_get_resource_state()`: verify credential authorizes `Read` for the resource's prefix.
- [x] 4.3 Add authorization check in `handle_sync_objects()`: verify credential authorizes `Read` for the federated resource's prefix.
- [x] 4.4 Add authorization check on the publish side: when serving a `ListResources` response, filter results to only include resources whose prefix falls within the credential's authorized scope.
- [x] 4.5 Add tests: authorized sync request succeeds, unauthorized prefix rejected, partially authorized list returns only accessible resources.

## 5. Token Lifecycle: Issuance, Refresh, Revocation

- [x] 5.1 Add `FederationRequest::RefreshToken { credential: Credential }` and `FederationResponse::TokenRefreshed { token: CapabilityToken }` to sync protocol types.
- [x] 5.2 Implement refresh handler in `aspen-federation/src/sync/handler.rs`: verify presented credential, check issuer == local cluster key, check not revoked, issue fresh token with same capabilities and new expiry. Reject if audience doesn't match presenter.
- [x] 5.3 Add revocation gossip: new `FederationGossipMessage::TokenRevoked { token_hash: [u8; 32], revoker: PublicKey, timestamp_ms: u64 }` variant. Receiver adds hash to local `RevocationStore`. Verify revoker signature.
- [x] 5.4 Add auto-refresh logic to subscription sync loop: before each sync, check if credential expires within 20% of its lifetime. If so, send `RefreshToken` request. On success, update stored credential. On failure, mark subscription `NeedsRefresh`.
- [x] 5.5 Add tests: refresh valid token, refresh revoked token (rejected), refresh wrong audience (rejected), auto-refresh trigger timing, revocation gossip propagation.

## 6. TrustManager as Derived State

- [x] 6.1 Add method `TrustManager::update_from_credential(cluster_key, credential)` that sets trust level to `Trusted` for clusters with valid credentials.
- [x] 6.2 Add method `TrustManager::expire_credential(cluster_key)` that transitions trust level to `Public` when a credential expires without refresh.
- [x] 6.3 Add method `TrustManager::revoke_credential(cluster_key)` that transitions trust level to `Blocked`.
- [x] 6.4 Ensure existing `TrustManager` API (`trust_level()`, `can_access_resource()`, `add_trusted()`) continues to work for backward compatibility — manual trust additions coexist with credential-derived trust.
- [x] 6.5 Add tests: trust derived from credential, trust expires with credential, trust blocked on revocation, manual trust coexists with credential trust.

## 7. Publish and Subscribe Operations

- [x] 7.1 Add `publish_prefix()` function in `aspen-federation/src/subscription.rs`: validates prefix, stores `PublishedPrefix` in KV, emits `ResourceAvailable` gossip event.
- [x] 7.2 Add `unpublish_prefix()` function: removes publication from KV, emits `ResourceRemoved` gossip event.
- [x] 7.3 Add `subscribe()` function: validates credential authorizes the prefix, stores `Subscription` in KV, starts sync loop (periodic or gossip-triggered).
- [x] 7.4 Add `unsubscribe()` function: stops sync loop, removes subscription from KV, leaves synced data in place.
- [x] 7.5 Add subscription resume on startup: scan `_sys:fed:sub:*` from KV, re-validate credentials (check expiry), resume active subscriptions, mark expired ones `NeedsRefresh`.
- [x] 7.6 Add tests: publish/unpublish round-trip, subscribe/unsubscribe round-trip, subscription resume after restart, expired credential on resume.

## 8. Client API and CLI

- [x] 8.1 Add RPC variants to `ClientRpcRequest`/`ClientRpcResponse` in `aspen-client-api`: `FederationGrant { audience, capabilities, lifetime_secs, allow_delegate }`, `FederationRevoke { token_hash }`, `FederationListTokens`, `FederationPublish { prefix, access_policy }`, `FederationSubscribe { source, prefix, sync_mode }`, `FederationListSubscriptions`, `FederationUnsubscribe { source, prefix }`.
- [x] 8.2 Add handler implementations in `aspen-rpc-handlers` or `aspen-federation` handler: dispatch RPC variants to underlying publish/subscribe/token functions.
- [x] 8.3 Add CLI commands in `aspen-cli`: `federation grant`, `federation delegate`, `federation revoke`, `federation tokens list`, `federation publish`, `federation subscribe`, `federation unsubscribe`, `federation subscriptions list`, `federation status`.
- [x] 8.4 Add CLI integration tests: issue token → subscribe → verify sync → unsubscribe.

## 9. Verified Functions and Specs

- [x] 9.1 Add verified pure functions in `aspen-auth/src/verified/` or `aspen-federation/src/verified/`: `is_credential_chain_valid(proofs, trusted_root) -> bool`, `credential_authorized_for_prefix(caps, prefix) -> bool`. Deterministic, no I/O, no async.
- [x] 9.2 Add Verus specs in `aspen-auth/verus/` or `aspen-federation/verus/`: chain validity invariant (each level attenuates), authorization correctness (prefix matching), delegation depth bound.
- [x] 9.3 Run `nix run .#verify-verus` and fix any verification failures.

## 10. Integration Testing

- [x] 10.1 Add federation auth unit tests in `aspen-federation/tests/`: two-cluster scenario with token issuance → handshake → sync → verify data transferred. Three-cluster delegation scenario.
- [x] 10.2 Add test for unauthorized access: Cluster B subscribes, attempts to read prefix outside token scope, verify rejection.
- [x] 10.3 Add test for token expiry: issue short-lived token, sync succeeds, wait for expiry, sync fails, refresh, sync succeeds again.
- [x] 10.4 Add test for revocation: issue token, sync succeeds, revoke, verify sync fails on next attempt.
- [x] 10.5 Add NixOS VM test: two-node cluster with federation — node A publishes nix cache prefix, node B subscribes with token, verify narinfo entries appear on node B.
