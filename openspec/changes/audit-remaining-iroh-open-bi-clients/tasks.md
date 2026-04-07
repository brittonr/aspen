## 1. Confirm the follow-up inventory

- [ ] 1.1 Re-run the direct `open_bi()` audit and confirm the one-request/one-response clients still lacking full timeout coverage
- [ ] 1.2 Separate long-lived/session protocols and test-only call sites from the request/response follow-up list
- [ ] 1.3 Decide which bridge helpers need dedicated large-response read budgets

## 2. Harden the remaining user-facing clients

- [ ] 2.1 Update `src/bin/git-remote-aspen/main.rs`
- [ ] 2.2 Update `crates/aspen-tui/src/iroh_client/rpc.rs` and `multi_node.rs`
- [ ] 2.3 Update `crates/aspen-fuse/src/client.rs`, `crates/aspen-hooks/src/client.rs`, and `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs`

## 3. Harden the remaining bridge/service clients

- [ ] 3.1 Update `crates/aspen-snix/src/rpc_blob_service.rs`, `rpc_directory_service.rs`, and `rpc_pathinfo_service.rs`
- [ ] 3.2 Update `crates/aspen-blob/src/replication/adapters.rs`
- [ ] 3.3 Decide whether `crates/aspen-castore/src/client.rs` should own stream-open timeouts or delegate them to the IRPC layer

## 4. Verify the follow-up rollout

- [ ] 4.1 Add targeted timeout regression tests for at least one representative client from each bucket
- [ ] 4.2 Run targeted crate tests for the touched clients
- [ ] 4.3 Run one quick live command path through a hardened auxiliary client
