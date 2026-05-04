# Authorization Matrix: ClientRpcRequest and CiRequest

Status: captured (Phase 3 request-classification slice).

## Collection policy
- Static source classification plus focused Cargo regressions.
- No live cluster tickets, auth tokens, root tokens, bearer strings, private keys, passwords, or connection strings were read.
- Detailed per-variant source handles are in `authorization-matrix.json`.

## Summary
- `ClientRpcRequest` variants: 350; source-classified: 350; missing: 0.
- `CiRequest` variants: 25; source-classified: 25; missing: 0.
- Exact `Some(None)` public/no-auth `ClientRpcRequest` variants: 25.
  - `Ping`, `GetHealth`, `GetNodeInfo`, `GetRaftMetrics`, `GetLeader`, `GetClusterTicket`, `GetClusterState`, `GetClusterTicketCombined`, `GetMetrics`, `GetNetworkMetrics`, `ListVaults`, `GetFederationStatus`, `ListDiscoveredClusters`, `GetDiscoveredCluster`, `ListFederatedRepositories`, `GetTopology`, `ListPeerClusters`, `GetPeerClusterStatus`, `GetKeyOrigin`, `GetClientTicket`, `GetDocsTicket`, `WatchCancel`, `WatchStatus`, `NostrAuthChallenge`, `NostrAuthVerify`

## Matrix findings remediated in this slice
- **Finding:** CiRequest::CiGetRefStatus existed but CiRequest::to_operation omitted it, while ClientRpcRequest already mapped the equivalent request to _ci:ref-status:<repo>:<ref> read authority.
  - **Fix:** Added CiRequest::CiGetRefStatus -> Operation::Read and regression test ci_ref_status_requires_read_authorization.
  - **Source:** `crates/aspen-client-api/src/messages/ci.rs`
- **Finding:** ClientRpcRequest classification drift test stopped at MAX_CLIENT_REQUEST_VARIANTS=256 while the enum has 350 variants.
  - **Fix:** Raised MAX_CLIENT_REQUEST_VARIANTS to 512 and reran every_client_request_variant_has_authorization_classification.
  - **Source:** `crates/aspen-client-api/src/messages/to_operation/mod.rs`

## Matrix acceptance
- Every `ClientRpcRequest` variant is represented in `messages/to_operation/*.rs`; public/no-auth rows are explicit `Some(None)` arms.
- Every `CiRequest` variant is represented in `CiRequest::to_operation`; this slice added the missing `CiGetRefStatus` mapping.
- The audit matrix intentionally records source handles and classification families, not live credential-bearing requests.

## Focused verification commands
- `cargo test -p aspen-client-api --features auth every_client_request_variant_has_authorization_classification -- --nocapture`
- `cargo test -p aspen-client-api --features auth every_ci_request_variant_has_authorization_classification -- --nocapture`
- `cargo test -p aspen-client-api --features auth ci_ref_status_requires_read_authorization -- --nocapture`

## Residual Phase 3 work
- Remaining Phase 3 tasks still need source-order auth-before-dispatch evidence, reserved-prefix bypass evidence, domain-specific capability review, and additional negative tests for high-risk bypass classes.
