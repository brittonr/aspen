# Aspen auth call-site inventory

Status: captured
Verification-IDs: ucan-auth-integration.sibling-source-of-truth, ucan-auth-integration.adapter-preserves-aspen-boundary, ucan-auth-integration.dependency-boundary-evidenced
Captured: 2026-05-06T23:21:13Z

## Commands / Oracles

- command: `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py drain-plan`
- command: content search for `CapabilityToken|TokenBuilder|TokenVerifier|Credential|authorize|auth` across `src/`, `crates/`, and auth/federation/RPC modules
- oracle: source anchors in current tree under `crates/aspen-auth-core`, `crates/aspen-auth`, `src/bin/aspen-token.rs`, `crates/aspen-rpc-handlers`, `crates/aspen-federation`, and service wrappers

## Outcomes

- result: pass — current Aspen auth boundary is centralized enough for a UCAN adapter: portable capability/token structs live in `aspen-auth-core`, runtime issuance/verification lives in `aspen-auth`, operator token behavior lives in `aspen-token`, RPC admission calls `TokenVerifier::authorize`, and federation handshakes use `Credential::verify`.
- result: pass — no source files were changed for this inventory task; this evidence is an implementation-planning artifact for later adapter and mapping tasks.

## Portable auth core inventory

`crates/aspen-auth-core` is the alloc-focused, Aspen-facing compatibility boundary:

- `crates/aspen-auth-core/src/lib.rs:17-21` re-exports `Capability`, `Operation`, `AuthError`, `Audience`, and `CapabilityToken`.
- `crates/aspen-auth-core/src/token.rs:19-81` defines `Audience::{Key, Bearer}` and `CapabilityToken` fields: `issuer`, `audience`, `capabilities`, `issued_at`, `expires_at`, `nonce`, `proof`, `delegation_depth`, `facts`, and `signature`.
- `crates/aspen-auth-core/src/token.rs:117-165` owns portable token serialization, base64 encoding/decoding, and BLAKE3 token hashing.
- `crates/aspen-auth-core/src/capability.rs:116-440` defines 51 Aspen `Capability` variants. Families: KV/read-write-delete/watch/full, `ClusterAdmin`, `Delegate`, shell execution, secrets/transit/PKI, service mesh, CI/jobs, blob/docs/hooks, KV metadata, coordination, SQL, observability, Automerge, federation, cache, and snix.
- `crates/aspen-auth-core/src/capability.rs:499-518` dispatches `Capability::authorizes(&Operation)` through family-specific authorization helpers.
- `crates/aspen-auth-core/src/capability.rs:1328-1639` defines matching `Operation` variants for the same capability families.
- `crates/aspen-auth-core/src/verified_credential.rs:53-104` contains pure credential-chain and prefix authorization helpers that overlap with UCAN attenuation/proof-chain semantics and should be classified as either replaced-by-UCAN or Aspen-local compatibility helpers in later tasks.

## Runtime auth shell inventory

`crates/aspen-auth` owns impure/runtime behavior and is the natural place for the std UCAN shell if needed:

- `crates/aspen-auth/src/lib.rs:47-61` re-exports portable core types plus runtime `TokenBuilder`, `TokenVerifier`, revocation helpers, `Credential`, and HMAC auth.
- `crates/aspen-auth/src/builder.rs:30-194` defines `TokenBuilder`; it sets audience, capabilities, lifetime, nonce/facts, delegation parent, validates capability count/depth/attenuation, requires `Capability::Delegate`, constructs `CapabilityToken`, and signs `bytes_to_sign` with `iroh_base::SecretKey`.
- `crates/aspen-auth/src/builder.rs:229-354` defines `generate_root_token`, which grants full cluster/bootstrap capabilities and is used by node bootstrap and token CLI.
- `crates/aspen-auth/src/verifier.rs:34-44` defines `TokenVerifier` with in-memory revocation set, parent-token cache, trusted roots, and clock-skew tolerance.
- `crates/aspen-auth/src/verifier.rs:126-227` verifies signature, expiry/future issue time, presenter audience, revocation, trusted roots, and parent-cache delegation chain.
- `crates/aspen-auth/src/verifier.rs:246-358` verifies explicit proof chains via `verify_with_chain`.
- `crates/aspen-auth/src/verifier.rs:364-383` implements `authorize` by first verifying the token and then checking `cap.authorizes(operation)`.
- `crates/aspen-auth/src/verifier.rs:389-467` owns runtime revocation cache operations.
- `crates/aspen-auth/src/credential.rs:37-149` defines `Credential { token, proofs }`, `Credential::verify`, `Credential::delegate`, bounded postcard/base64 encoding/decoding, and is the current federation credential carrier.
- `crates/aspen-auth/src/hmac_auth.rs` is Raft-cookie HMAC challenge/response and is not a UCAN token path; keep it out of the UCAN adapter except for docs that distinguish node-to-node Raft auth from capability tokens.
- `crates/aspen-auth/src/revocation.rs` persists token revocation hashes under `_system:auth:revoked:<hash_hex>`.

## Operator CLI inventory

`src/bin/aspen-token.rs` is the main operator-facing token surface and must retain compatibility or emit migration receipts:

- `src/bin/aspen-token.rs:50` defines CLI commands.
- `src/bin/aspen-token.rs:160` `generate_root_cmd` generates root bearer tokens and prints JSON/text summaries.
- `src/bin/aspen-token.rs:224-307` `delegate_cmd` decodes a parent `CapabilityToken`, creates a delegated token with `TokenBuilder::delegated_from`, optionally binds audience, enforces federation-proxy bearer/lifetime/capability/fact rules, and emits JSON/text output.
- `src/bin/aspen-token.rs:310-365` `verify_cmd` decodes a token, configures trusted roots, calls `TokenVerifier::verify`, prints validity/error/details, and exits non-zero on invalid tokens.
- `src/bin/aspen-token.rs:368-425` `inspect_cmd` decodes without verification and prints raw token details including signature, nonce, and proof hash.
- `src/bin/aspen-token.rs:693+` parses capability strings into Aspen `Capability` variants; this parser is a required input to the later Aspen-capability-to-UCAN-ability mapping table.

## Client RPC admission inventory

Client RPC auth currently gates serialized `AuthenticatedRequest` tokens against request-derived operations:

- `crates/aspen-rpc-core/src/context_runtime.rs:149-152` stores `token_verifier: Option<Arc<TokenVerifier>>` and `require_auth` in `ClientProtocolContext`.
- `crates/aspen-rpc-core/src/context_runtime.rs:340-367` logs production warnings when auth is disabled or missing a verifier.
- `crates/aspen-rpc-handlers/src/client.rs:348-363` maps `ClientRpcRequest::to_operation()` to optional `aspen_auth::Operation` and fails closed when `require_auth` is true but no verifier is configured.
- `crates/aspen-rpc-handlers/src/client.rs:365-417` binds presenter to the Iroh connection remote ID, calls `verifier.authorize(cap_token, &operation, Some(client_id))`, rejects missing tokens when required, and permits migration-mode unauthenticated requests only when `require_auth` is false.
- `crates/aspen-rpc-handlers/src/client.rs:419-427` parses either `AuthenticatedRequest` with a token/proxy hop count or a legacy unauthenticated request.
- `crates/aspen-rpc-handlers/src/proxy.rs` contains proxy-specific bearer/presenter policy for cross-cluster proxying and should be included in the migration compatibility tests.

## Federation credential inventory

Federation currently uses Aspen `Credential` and direct capability authorization:

- `crates/aspen-federation/src/sync/types.rs` carries optional `aspen_auth::Credential` in handshake/refresh messages and returns `aspen_auth::CapabilityToken` in refresh responses.
- `crates/aspen-federation/src/sync/client.rs:105-186` sends optional credentials during outbound federation handshake.
- `crates/aspen-federation/src/sync/handler.rs:336-407` checks blocked peers first, then accepts session credentials with `FederationPull` authorization for the resource, then legacy `Read` authorization for backward compatibility, then falls back to trust/resource settings.
- `crates/aspen-federation/src/sync/handler.rs:410-479` verifies presented credentials against this cluster key with presenter bound to peer key, stores verified credentials in the session, and updates trust from the credential.
- `crates/aspen-federation/src/token_store.rs:38-109` stores issued/received credentials in KV and loads non-expired received credentials for peers.
- `crates/aspen-federation/src/subscription.rs:238-256` validates subscription credentials by checking capabilities for `Operation::Read` over a prefix and serializes credentials into subscription records.
- `crates/aspen-federation/src/trust.rs:334-388` derives, expires, and revokes trust manager entries from verified credentials.

## Other capability-token consumers

- `crates/aspen-net/src/auth.rs:32-111` wraps `CapabilityToken` + `TokenVerifier` for service-mesh `NetConnect`, `NetPublish`, `NetUnpublish`, and admin checks.
- `src/bin/aspen_node/setup/client.rs:517-571` builds the node `TokenVerifier` from CLI/secrets mode and installs it into client protocol context.
- `src/bin/aspen_node/node_mode.rs:244-246` generates and writes a bootstrap root token.
- `src/bin/aspen_node/setup/router.rs:395-422` prints a cluster ticket and Automerge sync token via `TokenBuilder`; this is operator-facing and must remain redacted/compatible.
- `crates/aspen-client/src/ticket.rs` and `crates/aspen-hooks-ticket/src/lib.rs` store opaque `[u8; 32]` auth token fields with debug redaction. These are not `CapabilityToken` carriers today but should be checked before changing ticket auth naming or receipt redaction.

## Migration implications for next tasks

- The adapter cannot simply expose sibling UCAN structs everywhere; `Capability`, `Operation`, CLI strings, federation compatibility, and RPC request-to-operation mapping remain Aspen-owned.
- Generic token semantics currently duplicated in Aspen and likely replaceable by sibling UCAN APIs: signing bytes, signature verification, expiration/future issue checks, audience/presenter binding, proof-chain traversal, attenuation/delegation depth, facts, and token/proof hashing/encoding.
- Aspen-local compatibility shims likely remain: capability/operation vocabulary, prefix/path/glob containment rules, revocation storage key layout, redacted CLI/receipt output, federation legacy `Read` fallback, and proxy presenter policy.
- The next inventory task should inspect `../ucan` APIs and classify which sibling APIs map to `CapabilityToken`, `TokenBuilder`, `TokenVerifier`, `Credential`, and runtime revocation boundaries.
