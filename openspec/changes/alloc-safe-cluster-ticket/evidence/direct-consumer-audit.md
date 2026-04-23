Evidence-ID: alloc-safe-cluster-ticket.v1-direct-consumer-audit
Task-ID: V4
Artifact-Type: command-transcript
Covers: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.iroh-conversion-happens-at-the-shell-boundary, architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable

## Baseline discovery snapshot

### `python3 - <<\PY\ ... cargo metadata --format-version 1 --no-deps ... PY`

aspen-ci	crates/aspen-ci/Cargo.toml
aspen-ci-executor-vm	crates/aspen-ci-executor-vm/Cargo.toml
aspen-client	crates/aspen-client/Cargo.toml
aspen-cluster	crates/aspen-cluster/Cargo.toml
aspen-cluster-handler	crates/aspen-cluster-handler/Cargo.toml
aspen-rpc-handlers	crates/aspen-rpc-handlers/Cargo.toml

## Helper-usage search

### `rg -n 'SignedAspenClusterTicket|parse_ticket_to_addrs|with_bootstrap_addr|with_bootstrap\(|endpoint_addrs\(|endpoint_ids\(|AspenClusterTicket::deserialize|AspenClusterTicket::new|iroh::EndpointAddr|iroh::EndpointId|iroh_gossip::proto::TopicId|ClusterTopicId|try_into_iroh|to_topic_id|from_topic_id' crates/aspen-ci-executor-vm crates/aspen-cluster-handler crates/aspen-cluster crates/aspen-rpc-handlers crates/aspen-client crates/aspen-ci -g '*.rs'`

crates/aspen-cluster/src/cluster_discovery.rs:14:use iroh::EndpointAddr;
crates/aspen-ci-executor-vm/src/pool.rs:206:                match aspen_ticket::AspenClusterTicket::deserialize(&ticket_str) {
crates/aspen-cluster-handler/src/handler/tickets.rs:11:use iroh::EndpointId;
crates/aspen-cluster-handler/src/handler/tickets.rs:12:use iroh_gossip::proto::TopicId;
crates/aspen-cluster-handler/src/handler/tickets.rs:26:    let mut ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, ctx.cluster_cookie.clone(), &endpoint_addr);
crates/aspen-cluster-handler/src/handler/tickets.rs:71:    let mut ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, ctx.cluster_cookie.clone(), &endpoint_addr);
crates/aspen-rpc-handlers/src/proxy.rs:229:        let target_addr = iroh::EndpointAddr::new(*node_id);
crates/aspen-cluster/src/endpoint_manager.rs:10:use iroh::EndpointAddr;
crates/aspen-cluster/src/metrics_init.rs:131:where T: aspen_core::NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static {
crates/aspen-ci-executor-vm/src/vm/lifecycle.rs:27:    match aspen_ticket::AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-cluster/src/gossip/types.rs:7:use iroh::EndpointAddr;
crates/aspen-ci-executor-vm/src/vm/restore.rs:26:    match aspen_ticket::AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-client/src/client.rs:17:use iroh::EndpointAddr;
crates/aspen-client/src/client.rs:131:        let ticket = AspenClusterTicket::deserialize(ticket_str).context("failed to parse cluster ticket")?;
crates/aspen-client/src/client.rs:334:    pub fn endpoint_id(&self) -> iroh::EndpointId {
crates/aspen-cluster/src/gossip/mod.rs:22://! use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip/mod.rs:28://! #     endpoint_addr: iroh::EndpointAddr,
crates/aspen-cluster/src/gossip/discovery/trait_impl.rs:10:use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/discovery/trait_impl.rs:11:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:17:use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:24:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:68:    bootstrap_peers: Vec<iroh::EndpointId>,
crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:126:    pub fn set_bootstrap_peers(&mut self, peers: Vec<iroh::EndpointId>) {
crates/aspen-cluster/src/gossip/discovery/mod.rs:33:    use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/discovery/mod.rs:35:    use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip/discovery/blob.rs:6:use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/discovery/blob.rs:9:use iroh_gossip::proto::TopicId;
crates/aspen-client/src/lib.rs:161:pub use iroh::EndpointAddr;
crates/aspen-client/src/lib.rs:162:pub use iroh::EndpointId;
crates/aspen-client/src/ticket.rs:8:use iroh::EndpointAddr;
crates/aspen-client/src/ticket.rs:116:    use iroh::EndpointId;
crates/aspen-client/src/watch.rs:63:use iroh::EndpointAddr;
crates/aspen-rpc-handlers/src/test_mocks.rs:23:use iroh::EndpointAddr;
crates/aspen-cluster-handler/src/handler/mod.rs:22:use iroh::EndpointAddr;
crates/aspen-cluster-handler/src/handler/membership.rs:14:use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip_discovery.rs:16:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip_discovery.rs:95:        let bootstrap_peers = factory.get_peer_endpoint_ids().await;
crates/aspen-cluster/src/gossip_discovery.rs:103:        let callback: PeerDiscoveredCallback<iroh::EndpointAddr> =
crates/aspen-cluster/src/gossip_discovery.rs:104:            Box::new(move |peer: DiscoveredPeer<iroh::EndpointAddr>| {
crates/aspen-cluster/src/gossip_discovery.rs:165:    fn endpoint_addr_from_secret_key(secret_key: &SecretKey) -> iroh::EndpointAddr {
crates/aspen-cluster/src/gossip_discovery.rs:166:        iroh::EndpointAddr::new(secret_key.public())
crates/aspen-cluster/src/gossip_discovery.rs:180:            iroh::EndpointAddr,
crates/aspen-cluster/src/gossip_discovery.rs:231:        let callback: PeerDiscoveredCallback<iroh::EndpointAddr> =
crates/aspen-cluster/src/gossip_discovery.rs:232:            Box::new(move |peer: DiscoveredPeer<iroh::EndpointAddr>| {
crates/aspen-cluster/src/lib.rs:207:    use iroh_gossip::proto::TopicId;
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:129:    async fn resolve_node_addr(&self, node_id: u64) -> Result<iroh::EndpointAddr, RpcError> {
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:261:async fn setup_target_node(node_id: u64, seed: u64) -> (Endpoint, iroh::EndpointAddr, Router) {
crates/aspen-cluster/src/ticket.rs:8:pub use aspen_ticket::SignedAspenClusterTicket;
crates/aspen-cluster/src/ticket.rs:9:pub use aspen_ticket::parse_ticket_to_addrs;
crates/aspen-cluster/src/bootstrap/resources.rs:19:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/traits.rs:102:    pub peer_addrs: std::collections::HashMap<aspen_raft::types::NodeId, iroh::EndpointAddr>,
crates/aspen-cluster/src/bootstrap/traits.rs:112:    pub gossip_topic_id: iroh_gossip::proto::TopicId,
crates/aspen-cluster/src/endpoint_config.rs:6:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/relay_server.rs:20://! - `extract_endpoint_ids()`: Extracts endpoint IDs for access control
crates/aspen-cluster/src/relay_server.rs:38:use iroh::EndpointId;
crates/aspen-cluster/src/relay_server.rs:328:    use iroh::EndpointAddr;
crates/aspen-cluster/src/relay_server.rs:381:    fn test_extract_endpoint_ids() {
crates/aspen-cluster/src/relay_server.rs:386:        let ids = extract_endpoint_ids(vec![m1, m2]);
crates/aspen-cluster/src/relay_server.rs:400:        let ids = extract_endpoint_ids(vec![valid_member.clone(), invalid_member]);
crates/aspen-cluster/src/bootstrap/node/sharding_init.rs:37:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/node/sharding_init.rs:530:        match AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-cluster/src/bootstrap/node/discovery_init.rs:8:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/node/discovery_init.rs:39:        match AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-cluster/src/bootstrap/node/network_init.rs:12:use iroh::EndpointAddr;
crates/aspen-cluster/src/bootstrap/node/mod.rs:72:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/node/mod.rs:395:                    match member_info.node_addr.try_into_iroh() {
crates/aspen-cluster/src/bootstrap/node/mod.rs:1398:            let _: &iroh_gossip::proto::TopicId = &handle.discovery.gossip_topic_id;
crates/aspen-cluster/src/bootstrap/node/mod.rs:1478:            let _: &iroh_gossip::proto::TopicId = &base.discovery.gossip_topic_id;

## Final classification

- `Cargo.toml` workspace stanza → explicit alloc-safe workspace stanza (`default-features = false`).
- `crates/aspen-ci-executor-vm` → bare/default. Uses unsigned deserialize/inject flow only.
- `crates/aspen-cluster-handler` → `iroh`. Generates tickets from runtime endpoint/topic types.
- `crates/aspen-cluster` → `iroh`, `std`. Re-exports runtime parse helpers and signed-ticket surface.
- `crates/aspen-rpc-handlers` → bare/default. No `aspen-ticket` helper hits in crate sources.
- `crates/aspen-client` → `iroh`. Converts bootstrap peers to runtime endpoint addresses in the client shell.
- `crates/aspen-ci` → bare/default. No `aspen-ticket` helper hits in crate sources.
- Reopen result: no newly discovered direct consumers outside the audited set; no helper hits in `crates/aspen-rpc-handlers` or `crates/aspen-ci`, so both remain bare/default.

## Direct-consumer compile rails

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-ci-executor-vm'`


### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-cluster-handler'`


### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-cluster'`


### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-rpc-handlers'`


### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-client'`


### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-ci'`


## Representative transitive re-export leak proofs

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo tree -p aspen-fuse -e features -i aspen-ticket'`

aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
└── aspen-ticket feature "iroh"
    └── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
        └── aspen-client feature "default"
            └── aspen-fuse v0.1.0 (/home/brittonr/git/aspen/crates/aspen-fuse)
                └── aspen-fuse feature "default" (command-line)

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo tree -p aspen-cli -e features -i aspen-ticket'`

aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
├── aspen-ticket feature "iroh"
│   ├── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
│   │   └── aspen-client feature "default"
│   │       └── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli)
│   │           └── aspen-cli feature "default" (command-line)
│   └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
│       └── aspen-cluster feature "default"
│           └── aspen-cli v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cli) (*)
├── aspen-ticket feature "signed"
│   └── aspen-ticket feature "std"
│       └── aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster) (*)
└── aspen-ticket feature "std" (*)

