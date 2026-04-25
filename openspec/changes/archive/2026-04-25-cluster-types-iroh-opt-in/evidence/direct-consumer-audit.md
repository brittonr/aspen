Evidence-ID: cluster-types-iroh-opt-in.i1-consumer-audit
Task-ID: I1
Artifact-Type: audit
Covers: architecture.modularity.feature-bundles-are-explicit-and-bounded.cluster-types-iroh-opt-in-is-explicit

# Direct consumer audit

## Inventory command
```
rg -n 'aspen-cluster-types\s*=\s*\{' . -g 'Cargo.toml'
./Cargo.toml:258:aspen-cluster-types = { path = "crates/aspen-cluster-types", default-features = false }
./crates/aspen-traits/Cargo.toml:13:aspen-cluster-types = { path = "../aspen-cluster-types", default-features = false }
./crates/aspen-cluster-handler/Cargo.toml:56:aspen-cluster-types = { path = "../aspen-cluster-types", default-features = false, features = ["iroh"] }
./crates/aspen-testing/Cargo.toml:27:aspen-cluster-types = { version = "0.1.0", path = "../aspen-cluster-types", default-features = false }
./crates/aspen-testing-fixtures/Cargo.toml:12:aspen-cluster-types = { version = "0.1.0", path = "../aspen-cluster-types", default-features = false }
./crates/aspen-jobs/Cargo.toml:28:aspen-cluster-types = { path = "../aspen-cluster-types", default-features = false }
./crates/aspen-testing-patchbay/Cargo.toml:21:aspen-cluster-types = { path = "../aspen-cluster-types", default-features = false, features = ["iroh"] }
./crates/aspen-testing-madsim/Cargo.toml:14:aspen-cluster-types = { version = "0.1.0", path = "../aspen-cluster-types", default-features = false }
./crates/aspen-raft/Cargo.toml:19:aspen-cluster-types = { workspace = true, features = ["iroh"] }
./crates/aspen-testing-core/Cargo.toml:11:aspen-cluster-types = { version = "0.1.0", path = "../aspen-cluster-types", default-features = false }
./crates/aspen-core/Cargo.toml:16:aspen-cluster-types = { path = "../aspen-cluster-types", default-features = false }
```

## Helper-usage command (direct consumer crates only)
```
rg -n 'NodeAddress::new|ClusterNode::with_iroh_addr|\.iroh_addr\(|try_into_iroh\(' crates/aspen-core crates/aspen-traits crates/aspen-jobs crates/aspen-testing-core crates/aspen-testing-fixtures crates/aspen-testing crates/aspen-testing-madsim crates/aspen-raft crates/aspen-cluster-handler crates/aspen-testing-patchbay -g '*.rs'
crates/aspen-testing-patchbay/src/harness.rs:150:                node_addr: Some(NodeAddress::new(addr.clone())),
crates/aspen-testing-madsim/src/madsim_tester/node.rs:35:    RaftMemberInfo::new(aspen_core::NodeAddress::new(endpoint_addr))
crates/aspen-testing/src/router/mod.rs:503:    RaftMemberInfo::new(aspen_core::NodeAddress::new(endpoint_addr))
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:137:                if let Some(addr) = node.iroh_addr() {
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:293:    let target_node = ClusterNode::with_iroh_addr(2, target_addr);
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:382:    let target_node = ClusterNode::with_iroh_addr(2, target_addr);
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:405:    let target_node = ClusterNode::with_iroh_addr(2, target_addr);
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:485:    let node1 = ClusterNode::with_iroh_addr(1, coord_addr);
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:486:    let node2 = ClusterNode::with_iroh_addr(2, addr2);
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:487:    let node3 = ClusterNode::with_iroh_addr(3, addr3);
crates/aspen-cluster-handler/src/handler/state.rs:38:            node.iroh_addr().map(|a| format!("{:?}", a)).unwrap_or_default()
crates/aspen-cluster-handler/src/handler/state.rs:58:        let endpoint_addr = learner.iroh_addr().map(|a| format!("{:?}", a)).unwrap_or_default();
crates/aspen-cluster-handler/src/handler/membership.rs:55:                    learner: ClusterNode::with_iroh_addr(node_id, iroh_addr),
crates/aspen-cluster-handler/src/handler/tickets.rs:37:            if let Some(iroh_addr) = node.iroh_addr() {
crates/aspen-cluster-handler/src/handler/tickets.rs:109:            if let Some(iroh_addr) = node.iroh_addr() {
crates/aspen-testing/src/lib.rs:289:    RaftMemberInfo::new(aspen_core::NodeAddress::new(endpoint_addr))
crates/aspen-cluster-handler/src/handler/init.rs:23:    let this_node = ClusterNode::with_iroh_addr(ctx.node_id, endpoint_addr.clone());
crates/aspen-raft/src/types.rs:37:        .try_into_iroh()
crates/aspen-raft/src/types.rs:170:        let member_info = RaftMemberInfo::new(aspen_core::NodeAddress::new(endpoint_addr.clone()));
crates/aspen-raft/src/types.rs:403:        let info = RaftMemberInfo::new(aspen_core::NodeAddress::new(addr.clone()));
crates/aspen-raft/src/node/trust.rs:282:        .try_into_iroh()
crates/aspen-raft/src/node/trust.rs:921:            initial_members: ids.iter().map(|id| ClusterNode::with_iroh_addr(*id, endpoint_addr())).collect(),
crates/aspen-raft/src/node/trust.rs:1077:        addresses.insert(2, NodeAddress::new(remote_addr.clone()));
crates/aspen-raft/src/node/tests.rs:159:    node.node_addr = Some(aspen_cluster_types::NodeAddress::new(endpoint));
crates/aspen-raft/src/node/tests.rs:627:            node.node_addr = Some(aspen_cluster_types::NodeAddress::new(iroh::EndpointAddr::from_parts(
crates/aspen-raft/src/node/tests.rs:2183:        cn.node_addr = Some(aspen_cluster_types::NodeAddress::new(iroh::EndpointAddr::from_parts(
crates/aspen-raft/src/node/membership_refresh.rs:170:        let addr_hash = hash_addrs(&aspen_core::NodeAddress::new(new_addr.clone()));
crates/aspen-raft/src/node/membership_refresh.rs:183:        let mut updated_info = RaftMemberInfo::new(aspen_core::NodeAddress::new(new_addr.clone()));
crates/aspen-raft/src/node/membership_refresh.rs:265:        let node_addr = aspen_core::NodeAddress::new(addr);
crates/aspen-raft/src/node/membership_refresh.rs:283:        assert_ne!(hash_addrs(&aspen_core::NodeAddress::new(addr1)), hash_addrs(&aspen_core::NodeAddress::new(addr2)));
crates/aspen-raft/src/storage_shared/trust.rs:459:        NodeAddress::new(EndpointAddr::new(key.public()))
crates/aspen-raft/src/network/tests.rs:184:    let member_info = RaftMemberInfo::new(aspen_core::NodeAddress::new(endpoint_addr.clone()));
crates/aspen-raft/src/network/tests.rs:207:    let member_info = RaftMemberInfo::new(aspen_core::NodeAddress::new(endpoint_addr));
```

## Classification

| Consumer stanza | Classification | Justification | Evidence |
| --- | --- | --- | --- |
| `Cargo.toml` workspace dependency | alloc-safe | Root workspace stanza keeps `default-features = false` and no `features = ["iroh"]`. | `evidence/workspace-dependency-proof.txt` |
| `crates/aspen-core/Cargo.toml` | alloc-safe | Keeps `default-features = false`; direct crate sources do not call `NodeAddress::new`, `ClusterNode::with_iroh_addr`, `.iroh_addr()`, or `try_into_iroh()`. | Inventory + helper-usage commands above |
| `crates/aspen-traits/Cargo.toml` | alloc-safe | Keeps `default-features = false`; no direct helper usage in crate sources. | Inventory + helper-usage commands above; `evidence/cluster-types-validation.md` (`cargo tree -p aspen-traits -e normal --depth 2`) |
| `crates/aspen-jobs/Cargo.toml` | alloc-safe | Now sets `default-features = false`; no direct helper usage in crate sources. | Inventory + helper-usage commands above; `evidence/runtime-consumers.md` |
| `crates/aspen-testing-core/Cargo.toml` | alloc-safe | Now sets `default-features = false`; no direct helper usage in crate sources. | Inventory + helper-usage commands above; `evidence/runtime-consumers.md` |
| `crates/aspen-testing-fixtures/Cargo.toml` | alloc-safe | Now sets `default-features = false`; no direct helper usage in crate sources. | Inventory + helper-usage commands above; `evidence/runtime-consumers.md` |
| `crates/aspen-testing/Cargo.toml` | alloc-safe | Now sets `default-features = false`; helper-style calls in this crate go through `aspen_core::NodeAddress::new`, not direct `aspen-cluster-types` helper use. | Inventory + helper-usage commands above; `evidence/runtime-consumers.md` |
| `crates/aspen-testing-madsim/Cargo.toml` | alloc-safe | Now sets `default-features = false`; helper-style calls in this crate go through `aspen_core::NodeAddress::new`, not direct `aspen-cluster-types` helper use. | Inventory + helper-usage commands above; `evidence/runtime-consumers.md` |
| `crates/aspen-raft/Cargo.toml` | iroh-opt-in | Direct crate sources call `NodeAddress::new` and `try_into_iroh()`, so this crate now opts into `features = ["iroh"]` explicitly. | Inventory + helper-usage commands above; `evidence/runtime-consumers.md` |
| `crates/aspen-cluster-handler/Cargo.toml` | iroh-opt-in | Direct crate sources/tests call `ClusterNode::with_iroh_addr` and `.iroh_addr()`, so this dev-dependency stanza keeps explicit `features = ["iroh"]`. | Inventory + helper-usage commands above; `evidence/runtime-consumers.md` |
| `crates/aspen-testing-patchbay/Cargo.toml` | iroh-opt-in | Direct crate sources call `NodeAddress::new`, so this crate keeps explicit `features = ["iroh"]`. | Inventory + helper-usage commands above; `evidence/runtime-consumers.md` |
