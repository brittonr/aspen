## 1. Define the timeout policy

- [x] 1.1 Inventory the client-side QUIC RPC helpers that currently bound only connect or only response read
- [x] 1.2 Choose the shared helper shape for bounded post-connect exchanges and document the stage-specific error messages
- [x] 1.3 Decide which call sites need a dedicated read budget for large responses instead of the default RPC budget

## 2. Apply the bounded exchange helper to common clients

- [x] 2.1 Update `crates/aspen-client/src/client.rs` to bound `open_bi()`, `write_all()`, `finish()`, and response read
- [x] 2.2 Update `crates/aspen-cli/src/bin/aspen-cli/client.rs` to use the same timeout coverage for cached RPC requests
- [x] 2.3 Update `send_get_blob()` in the CLI to bound `open_bi()`, `write_all()`, `finish()`, and the 256 MiB `read_to_end()` with a dedicated large-response read budget
- [x] 2.4 Ensure CLI timeout paths discard cached connections instead of returning them to the pool

## 3. Apply the same policy to federation and proxy paths

- [x] 3.1 Update `crates/aspen-federation/src/sync/client.rs` handshake, list, state, sync, and push helpers
- [x] 3.2 Update `crates/aspen-rpc-handlers/src/proxy.rs` remote-cluster forwarding to bound stream open, request write, finish, and response read
- [x] 3.3 Review other QUIC request/response helpers that open streams directly and either convert them or explicitly document why they differ

## 4. Add regression coverage

- [x] 4.1 Add tests that simulate a peer accepting a connection but never opening a bidirectional stream
- [x] 4.2 Add tests that simulate request-body backpressure or a peer that never drains bytes
- [x] 4.3 Add tests that simulate a peer that never replies after request flush
- [x] 4.4 Assert that timed-out cached connections are not reused by subsequent requests

## 5. Verify the rollout

- [x] 5.1 Run targeted crate tests for the updated client, federation, and proxy modules
- [x] 5.2 Run a quick end-to-end command path that exercises CLI or client RPCs against a real node
  - Verified `target/debug/aspen-cli --ticket ... cluster health` and `cluster init` against a live local `target/debug/aspen-node`; a follow-up `kv set/get` attempt hit the node's disk-pressure guard rather than any transport timeout path.
- [x] 5.3 Record any remaining direct `open_bi()` call sites that need follow-up in a separate audit issue if they are out of scope
  - Created follow-up change: `openspec/changes/audit-remaining-iroh-open-bi-clients`.
