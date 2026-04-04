## 1. Refactor connect_to_cluster to return ConnectResult

- [x] 1.1 Change `connect_to_cluster()` in `crates/aspen-federation/src/sync/client.rs` to return `Result<ConnectResult>` — extract `capabilities` from the `FederationResponse::Handshake` variant
- [x] 1.2 Remove `connect_to_cluster_full()` from `client.rs`
- [x] 1.3 Update re-exports in `crates/aspen-federation/src/sync/mod.rs` — remove `connect_to_cluster_full`, export `ConnectResult`

## 2. Update orchestrator call sites

- [x] 2.1 Update `crates/aspen-federation/src/sync/orchestrator.rs` line ~270 to destructure `ConnectResult`
- [x] 2.2 Update `crates/aspen-federation/src/sync/orchestrator.rs` line ~357 to destructure `ConnectResult`
- [x] 2.3 Add capability gating in orchestrator — check `has_capability("streaming-sync")` before streaming sync paths

## 3. Update forge handler call sites

- [x] 3.1 Update 5 call sites in `crates/aspen-forge-handler/src/handler/handlers/federation.rs` to destructure `ConnectResult`
- [x] 3.2 Update 3 call sites in `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs` to destructure `ConnectResult`

## 4. Thread credentials through connections

- [x] 4.1 Add credential lookup helper: `async fn lookup_federation_credential(store, cluster_key) -> Option<Credential>` that reads from KV key `_sys:fed:token:received:<hex_key>`, returns `None` on missing or error (with warning log)
- [x] 4.2 Wire credential lookup into orchestrator `connect_to_cluster` calls
- [x] 4.3 Wire credential lookup into forge federation handler calls (federation.rs)
- [x] 4.4 Wire credential lookup into forge git handler calls (federation_git.rs)

## 5. Tests

- [x] 5.1 Unit test: `connect_to_cluster` returns capabilities from handshake response (mock a peer that sends specific capabilities)
- [x] 5.2 Unit test: empty capabilities from old peer handled correctly
- [x] 5.3 Unit test: credential lookup returns `None` when key missing, logs warning on error
- [x] 5.4 Integration test: handshake with credential flows through to `session_credential` on handler side
- [x] 5.5 Verify existing federation tests still pass (`cargo nextest run -p aspen-federation` — 169 passed, 41 skipped)
