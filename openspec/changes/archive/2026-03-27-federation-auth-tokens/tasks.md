## 1. Capability Variants

- [x] 1.1 Add `FederationPull { repo_prefix: String }` and `FederationPush { repo_prefix: String }` to `Capability` enum in `crates/aspen-auth/src/capability.rs`
- [x] 1.2 Add `FederationPull` and `FederationPush` to `Operation` enum with matching `authorizes()` logic
- [x] 1.3 Implement `contains()` for the new variants (prefix attenuation)
- [x] 1.4 Unit tests for `authorizes()` and `contains()` on the new variants

## 2. Client Credential Plumbing

- [x] 2.1 Add `credential: Option<Credential>` parameter to `connect_to_cluster` in `crates/aspen-federation/src/sync/client.rs`
- [x] 2.2 Pass the credential into the `Handshake` request (replace hardcoded `credential: None`)
- [x] 2.3 Update all callers of `connect_to_cluster` (orchestrator, tests) to pass the new parameter

## 3. Handler Enforcement

- [x] 3.1 Update `check_resource_access` to check session credential for `FederationPull` capability matching the target resource
- [x] 3.2 Update `handle_push_objects` to verify session credential contains `FederationPush` matching the target FederatedId (not just `has_credential`)
- [x] 3.3 Add blocked-peer check before credential check in both paths (blocked always denied)

## 4. Token Storage

- [x] 4.1 Add KV storage helpers: `store_issued_token(audience, credential)` and `store_received_token(issuer, credential)` writing to `_sys:fed:token:{issued|received}:<hex>`
- [x] 4.2 Add `load_credential_for_peer(peer_key)` that reads from KV and skips expired tokens
- [x] 4.3 Wire orchestrator to call `load_credential_for_peer` before `connect_to_cluster`

## 5. CLI Commands

- [x] 5.1 Implement `federation grant` CLI command: issue a token with `FederationPull`/`FederationPush` capabilities, print base64 token, store in KV
- [x] 5.2 Implement `federation token list` CLI command: scan `_sys:fed:token:issued:*` and display audience, capabilities, expiry
- [x] 5.3 Implement `federation token inspect <base64>` CLI command: decode and display token fields without verification

## 6. Tests

- [x] 6.1 Integration test: two-cluster push with `FederationPush` credential succeeds, without credential fails
- [x] 6.2 Integration test: pull from AllowList resource succeeds with `FederationPull` credential, fails without
- [x] 6.3 Integration test: blocked peer denied despite valid credential
- [x] 6.4 Integration test: credential with wrong repo prefix denied
- [x] 6.5 Unit test: delegation attenuation — parent `FederationPull { "" }` → child `FederationPull { "org-a/" }` succeeds, reverse fails
