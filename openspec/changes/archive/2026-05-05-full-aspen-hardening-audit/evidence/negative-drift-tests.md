# Negative Drift Tests Evidence

Generated: `2026-05-05T02:08:14Z`

## Scope

Phase 3 negative drift test for public/no-auth ClientRpcRequest classification bypass risk.

## High-risk bypass class

Authorization matrix identified public/no-auth request classification as a high-risk drift class: adding `=> Some(None)` for a protected request could bypass capability checks while still satisfying the broad classification-presence test.

## Implemented regression

- `public_no_auth_request_variants_match_audited_allowlist` at `crates/aspen-client-api/src/messages/to_operation/mod.rs:288`
- Contract: Every ClientRpcRequest classified as public/no-auth through `Some(None)` must match the audited allowlist exactly.

## Public/no-auth allowlist

Count: `25`

- `GetClientTicket`
- `GetClusterState`
- `GetClusterTicket`
- `GetClusterTicketCombined`
- `GetDiscoveredCluster`
- `GetDocsTicket`
- `GetFederationStatus`
- `GetHealth`
- `GetKeyOrigin`
- `GetLeader`
- `GetMetrics`
- `GetNetworkMetrics`
- `GetNodeInfo`
- `GetPeerClusterStatus`
- `GetRaftMetrics`
- `GetTopology`
- `ListDiscoveredClusters`
- `ListFederatedRepositories`
- `ListPeerClusters`
- `ListVaults`
- `NostrAuthChallenge`
- `NostrAuthVerify`
- `Ping`
- `WatchCancel`
- `WatchStatus`

## Verification

- `rustfmt crates/aspen-client-api/src/messages/to_operation/mod.rs` — passed
- `cargo test -p aspen-client-api --features auth public_no_auth_request_variants_match_audited_allowlist -- --nocapture` — passed
- `cargo test -p aspen-client-api --features auth every_client_request_variant_has_authorization_classification -- --nocapture` — passed
- `cargo check -p aspen-client-api --features auth` — passed
- `scripts/tigerstyle-check.sh` — passed

- `cargo test -p aspen-client-api --features auth to_operation -- --nocapture` — passed

## Credential handling

No credential files or token values were read. Evidence names request variants and source paths only.
