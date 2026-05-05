# Auth-before-dispatch evidence

Generated: `2026-05-04T22:21:43Z`

## Scope

Phase 3 auth-before-dispatch evidence for client RPC, blob fast path, registry dispatch, and proxy fallback.

This evidence uses the Phase 3 authorization matrix as the request-classification source of truth, then verifies the runtime dispatch path gates protected requests before blob streaming, local native/plugin handler dispatch, or cross-cluster proxy fallback.

## Verified boundaries

| Boundary | File | Guard line | Sink line | Result | Notes |
| --- | --- | ---: | ---: | --- | --- |
| `parse-before-auth` | `crates/aspen-rpc-handlers/src/client.rs` | 541 | 554 | guard-before-sink | Request bytes are decoded into AuthenticatedRequest or legacy request before authorization classification. |
| `auth-before-blob-fast-path` | `crates/aspen-rpc-handlers/src/client.rs` | 554 | 581 | guard-before-sink | The blob streaming fast path is reached only after the shared auth gate succeeds. |
| `auth-before-registry-dispatch` | `crates/aspen-rpc-handlers/src/client.rs` | 554 | 582 | guard-before-sink | Registry dispatch for native/plugin handlers is reached only after the shared auth gate succeeds. |
| `presenter-is-iroh-remote-id` | `crates/aspen-rpc-handlers/src/client.rs` | 144 | 398 | guard-before-sink | Key-audience token presenter is copied from the authenticated Iroh connection remote_id. |
| `classifier-before-verifier` | `crates/aspen-rpc-handlers/src/client.rs` | 375 | 398 | guard-before-sink | Authorization derives an Operation through ClientRpcRequest::to_operation before verifier authorization. |
| `local-handlers-before-proxy` | `crates/aspen-rpc-handlers/src/registry.rs` | 438 | 469 | guard-before-sink | Proxy fallback runs only after local prefix and direct handler passes miss. |
| `token-not-passed-to-local-handlers` | `crates/aspen-rpc-handlers/src/registry.rs` | 385 | 425 | guard-before-sink | Direct native/plugin handlers receive request and context only; token material is retained only for proxy fallback re-wrapping. |
| `proxy-explicit-delegation` | `crates/aspen-rpc-handlers/src/proxy.rs` | 186 | 305 | guard-before-sink | Authenticated cross-cluster proxying requires an explicit delegated bearer token with federation-proxy fact; key-bound and generic bearer tokens are rejected. |

## Regression tests

- `client::tests::auth_gate_allows_public_requests_without_verifier`
- `client::tests::auth_gate_denies_authorized_requests_when_require_auth_lacks_verifier`
- `client::tests::auth_gate_requires_to_operation_before_dispatch`
- `client::tests::auth_gate_binds_key_audience_tokens_to_iroh_remote_id`
- `client::tests::client_request_path_checks_auth_before_blob_and_handler_dispatch`
- `client::tests::client_request_path_uses_connection_remote_id_as_auth_presenter`
- `registry::tests::registry_dispatch_fallbacks_run_after_local_handler_passes`
- `registry::tests::registry_dispatch_receives_already_verified_token_only_for_proxy_fallback`
- `proxy::tests::token_allows_proxying_only_for_public_or_explicit_delegated_bearer_presenters`
- `proxy::tests::proxied_authenticated_request_preserves_client_token`

## Finding

No unauthenticated protected request dispatch path found in audited client RPC path. Public/no-auth requests remain governed by to_operation()==None classification captured in authorization-matrix evidence.
