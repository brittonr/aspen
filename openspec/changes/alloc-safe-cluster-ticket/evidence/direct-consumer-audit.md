Evidence-ID: alloc-safe-cluster-ticket.v1-direct-consumer-audit
Task-ID: V4
Artifact-Type: command-transcript
Covers: architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.iroh-conversion-happens-at-the-shell-boundary, architecture.modularity.cluster-ticket-runtime-helpers-require-explicit-shell-opt-in.cluster-ticket-seam-proof-is-reviewable

## Baseline direct-consumer discovery

### `python3 - <<\PY\ ... cargo metadata --format-version 1 --no-deps ... PY`

aspen-ci	crates/aspen-ci/Cargo.toml
aspen-ci-executor-vm	crates/aspen-ci-executor-vm/Cargo.toml
aspen-client	crates/aspen-client/Cargo.toml
aspen-cluster	crates/aspen-cluster/Cargo.toml
aspen-cluster-handler	crates/aspen-cluster-handler/Cargo.toml
aspen-rpc-handlers	crates/aspen-rpc-handlers/Cargo.toml

## Baseline helper-usage classification

### `rg -n 'SignedAspenClusterTicket|parse_ticket_to_addrs|with_bootstrap_addr|with_bootstrap\(|endpoint_addrs\(|endpoint_ids\(|AspenClusterTicket::deserialize|AspenClusterTicket::new|iroh::EndpointAddr|iroh::EndpointId|iroh_gossip::proto::TopicId|ClusterTopicId|try_into_iroh|to_topic_id|from_topic_id' crates/aspen-ci-executor-vm crates/aspen-cluster-handler crates/aspen-cluster crates/aspen-rpc-handlers crates/aspen-client crates/aspen-ci -g '*.rs'`

crates/aspen-cluster/src/cluster_discovery.rs:14:use iroh::EndpointAddr;
crates/aspen-rpc-handlers/src/proxy.rs:229:        let target_addr = iroh::EndpointAddr::new(*node_id);
crates/aspen-cluster-handler/src/handler/tickets.rs:11:use iroh::EndpointId;
crates/aspen-cluster-handler/src/handler/tickets.rs:12:use iroh_gossip::proto::TopicId;
crates/aspen-cluster-handler/src/handler/tickets.rs:26:    let mut ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, ctx.cluster_cookie.clone(), &endpoint_addr);
crates/aspen-cluster-handler/src/handler/tickets.rs:71:    let mut ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, ctx.cluster_cookie.clone(), &endpoint_addr);
crates/aspen-ci-executor-vm/src/pool.rs:206:                match aspen_ticket::AspenClusterTicket::deserialize(&ticket_str) {
crates/aspen-client/src/client.rs:17:use iroh::EndpointAddr;
crates/aspen-client/src/client.rs:131:        let ticket = AspenClusterTicket::deserialize(ticket_str).context("failed to parse cluster ticket")?;
crates/aspen-client/src/client.rs:334:    pub fn endpoint_id(&self) -> iroh::EndpointId {
crates/aspen-cluster/src/endpoint_manager.rs:10:use iroh::EndpointAddr;
crates/aspen-cluster/src/metrics_init.rs:131:where T: aspen_core::NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static {
crates/aspen-cluster-handler/src/handler/mod.rs:22:use iroh::EndpointAddr;
crates/aspen-cluster-handler/src/handler/membership.rs:14:use iroh::EndpointAddr;
crates/aspen-rpc-handlers/src/test_mocks.rs:23:use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/types.rs:7:use iroh::EndpointAddr;
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:129:    async fn resolve_node_addr(&self, node_id: u64) -> Result<iroh::EndpointAddr, RpcError> {
crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:261:async fn setup_target_node(node_id: u64, seed: u64) -> (Endpoint, iroh::EndpointAddr, Router) {
crates/aspen-cluster/src/gossip/mod.rs:22://! use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip/mod.rs:28://! #     endpoint_addr: iroh::EndpointAddr,
crates/aspen-ci-executor-vm/src/vm/lifecycle.rs:27:    match aspen_ticket::AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-cluster/src/gossip/discovery/trait_impl.rs:10:use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/discovery/trait_impl.rs:11:use iroh_gossip::proto::TopicId;
crates/aspen-ci-executor-vm/src/vm/restore.rs:26:    match aspen_ticket::AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:17:use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:24:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:68:    bootstrap_peers: Vec<iroh::EndpointId>,
crates/aspen-cluster/src/gossip/discovery/lifecycle.rs:126:    pub fn set_bootstrap_peers(&mut self, peers: Vec<iroh::EndpointId>) {
crates/aspen-cluster/src/gossip/discovery/mod.rs:33:    use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/discovery/mod.rs:35:    use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip/discovery/blob.rs:6:use iroh::EndpointAddr;
crates/aspen-cluster/src/gossip/discovery/blob.rs:9:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/resources.rs:19:use iroh_gossip::proto::TopicId;
crates/aspen-client/src/ticket.rs:8:use iroh::EndpointAddr;
crates/aspen-client/src/ticket.rs:116:    use iroh::EndpointId;
crates/aspen-client/src/lib.rs:161:pub use iroh::EndpointAddr;
crates/aspen-client/src/lib.rs:162:pub use iroh::EndpointId;
crates/aspen-client/src/watch.rs:63:use iroh::EndpointAddr;
crates/aspen-cluster/src/bootstrap/node/sharding_init.rs:37:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/node/sharding_init.rs:530:        match AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-cluster/src/bootstrap/node/sharding_init.rs:537:                return ticket.topic_id.to_topic_id();
crates/aspen-cluster/src/bootstrap/traits.rs:102:    pub peer_addrs: std::collections::HashMap<aspen_raft::types::NodeId, iroh::EndpointAddr>,
crates/aspen-cluster/src/bootstrap/traits.rs:112:    pub gossip_topic_id: iroh_gossip::proto::TopicId,
crates/aspen-cluster/src/bootstrap/node/network_init.rs:12:use iroh::EndpointAddr;
crates/aspen-cluster/src/endpoint_config.rs:6:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/node/discovery_init.rs:8:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/node/discovery_init.rs:39:        match AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-cluster/src/bootstrap/node/discovery_init.rs:46:                return ticket.topic_id.to_topic_id();
crates/aspen-cluster/src/gossip_discovery.rs:16:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/gossip_discovery.rs:95:        let bootstrap_peers = factory.get_peer_endpoint_ids().await;
crates/aspen-cluster/src/gossip_discovery.rs:103:        let callback: PeerDiscoveredCallback<iroh::EndpointAddr> =
crates/aspen-cluster/src/gossip_discovery.rs:104:            Box::new(move |peer: DiscoveredPeer<iroh::EndpointAddr>| {
crates/aspen-cluster/src/gossip_discovery.rs:165:    fn endpoint_addr_from_secret_key(secret_key: &SecretKey) -> iroh::EndpointAddr {
crates/aspen-cluster/src/gossip_discovery.rs:166:        iroh::EndpointAddr::new(secret_key.public())
crates/aspen-cluster/src/gossip_discovery.rs:180:            iroh::EndpointAddr,
crates/aspen-cluster/src/gossip_discovery.rs:231:        let callback: PeerDiscoveredCallback<iroh::EndpointAddr> =
crates/aspen-cluster/src/gossip_discovery.rs:232:            Box::new(move |peer: DiscoveredPeer<iroh::EndpointAddr>| {
crates/aspen-cluster/src/bootstrap/node/mod.rs:72:use iroh_gossip::proto::TopicId;
crates/aspen-cluster/src/bootstrap/node/mod.rs:395:                    match member_info.node_addr.try_into_iroh() {
crates/aspen-cluster/src/bootstrap/node/mod.rs:1398:            let _: &iroh_gossip::proto::TopicId = &handle.discovery.gossip_topic_id;
crates/aspen-cluster/src/bootstrap/node/mod.rs:1478:            let _: &iroh_gossip::proto::TopicId = &base.discovery.gossip_topic_id;
crates/aspen-cluster/src/ticket.rs:8:pub use aspen_ticket::SignedAspenClusterTicket;
crates/aspen-cluster/src/ticket.rs:9:pub use aspen_ticket::parse_ticket_to_addrs;
crates/aspen-cluster/src/relay_server.rs:20://! - `extract_endpoint_ids()`: Extracts endpoint IDs for access control
crates/aspen-cluster/src/relay_server.rs:38:use iroh::EndpointId;
crates/aspen-cluster/src/relay_server.rs:328:    use iroh::EndpointAddr;
crates/aspen-cluster/src/relay_server.rs:381:    fn test_extract_endpoint_ids() {
crates/aspen-cluster/src/relay_server.rs:386:        let ids = extract_endpoint_ids(vec![m1, m2]);
crates/aspen-cluster/src/relay_server.rs:400:        let ids = extract_endpoint_ids(vec![valid_member.clone(), invalid_member]);
crates/aspen-cluster/src/lib.rs:207:    use iroh_gossip::proto::TopicId;

## Final shell-boundary citations

### `rg -n 'with_bootstrap_addr|add_bootstrap\(|add_bootstrap_addr' crates/aspen-cluster-handler/src/handler/tickets.rs`

26:    let mut ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, ctx.cluster_cookie.clone(), &endpoint_addr);
42:                if ticket.add_bootstrap_addr(&iroh_addr).is_ok() {
71:    let mut ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, ctx.cluster_cookie.clone(), &endpoint_addr);
95:                if ticket.add_bootstrap(endpoint_id).is_ok() {
115:                if ticket.add_bootstrap_addr(&iroh_addr).is_ok() {

### `rg -n 'to_endpoint_addr\(' crates/aspen-client/src/client.rs`

261:        let target_addr = peer.to_endpoint_addr();

### `rg -n 'parse_ticket_to_addrs|SignedAspenClusterTicket' crates/aspen-cluster/src/ticket.rs`

8:pub use aspen_ticket::SignedAspenClusterTicket;
9:pub use aspen_ticket::parse_ticket_to_addrs;

### `rg -n 'topic_id\.to_topic_id\(' crates/aspen-cluster/src/bootstrap/node/discovery_init.rs crates/aspen-cluster/src/bootstrap/node/sharding_init.rs`

crates/aspen-cluster/src/bootstrap/node/sharding_init.rs:537:                return ticket.topic_id.to_topic_id();
crates/aspen-cluster/src/bootstrap/node/discovery_init.rs:46:                return ticket.topic_id.to_topic_id();

### `rg -n 'AspenClusterTicket::deserialize|inject_direct_addr' crates/aspen-ci-executor-vm/src`

crates/aspen-ci-executor-vm/src/pool.rs:206:                match aspen_ticket::AspenClusterTicket::deserialize(&ticket_str) {
crates/aspen-ci-executor-vm/src/pool.rs:208:                        ticket.inject_direct_addr(bridge_addr);
crates/aspen-ci-executor-vm/src/vm/lifecycle.rs:27:    match aspen_ticket::AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-ci-executor-vm/src/vm/lifecycle.rs:34:            ticket.inject_direct_addr(bridge_addr);
crates/aspen-ci-executor-vm/src/vm/restore.rs:26:    match aspen_ticket::AspenClusterTicket::deserialize(ticket_str) {
crates/aspen-ci-executor-vm/src/vm/restore.rs:28:            ticket.inject_direct_addr(bridge_addr);

### `rg -n 'AspenClusterTicket|parse_ticket_to_addrs|SignedAspenClusterTicket|with_bootstrap_addr|add_bootstrap_addr|add_bootstrap\(' crates/aspen-rpc-handlers crates/aspen-ci -g '*.rs'`


## Final classification

- `Cargo.toml` workspace stanza → explicit alloc-safe workspace stanza (`default-features = false`).
- `crates/aspen-ci-executor-vm` → bare/default. Uses unsigned deserialize and alloc-safe `inject_direct_addr` flow only.
- `crates/aspen-cluster-handler` → `iroh`. Generates tickets from runtime endpoint/topic types in the RPC shell.
- `crates/aspen-cluster` → `iroh`, `std`. Re-exports runtime parse helpers and signed-ticket surface, and converts `ClusterTopicId` back to runtime `TopicId` at the cluster bootstrap shell.
- `crates/aspen-rpc-handlers` → bare/default. No `aspen-ticket` helper hits in crate sources.
- `crates/aspen-client` → `iroh`. Converts alloc-safe bootstrap peers to runtime endpoint addresses in the client transport shell.
- `crates/aspen-ci` → bare/default. No `aspen-ticket` helper hits in crate sources.
- Reopen result: no newly discovered direct consumers outside the audited set; no helper hits in `crates/aspen-rpc-handlers` or `crates/aspen-ci`, so both remain bare/default.

## Runtime-surface proof for `aspen-ticket` itself

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo tree -p aspen-ticket --features iroh -e normal'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
├── aspen-cluster-types v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-types)
│   ├── iroh-base v0.97.0
│   │   ├── curve25519-dalek v5.0.0-pre.1
│   │   │   ├── cfg-if v1.0.4
│   │   │   ├── cpufeatures v0.2.17
│   │   │   ├── curve25519-dalek-derive v0.1.1 (proc-macro)
│   │   │   │   ├── proc-macro2 v1.0.106
│   │   │   │   │   └── unicode-ident v1.0.24
│   │   │   │   ├── quote v1.0.45
│   │   │   │   │   └── proc-macro2 v1.0.106 (*)
│   │   │   │   └── syn v2.0.117
│   │   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │       ├── quote v1.0.45 (*)
│   │   │   │       └── unicode-ident v1.0.24
│   │   │   ├── digest v0.11.0-rc.10
│   │   │   │   ├── block-buffer v0.11.0
│   │   │   │   │   └── hybrid-array v0.4.8
│   │   │   │   │       └── typenum v1.19.0
│   │   │   │   ├── const-oid v0.10.2
│   │   │   │   └── crypto-common v0.2.1
│   │   │   │       └── hybrid-array v0.4.8 (*)
│   │   │   ├── rand_core v0.9.5
│   │   │   │   └── getrandom v0.3.4
│   │   │   │       ├── cfg-if v1.0.4
│   │   │   │       └── libc v0.2.183
│   │   │   ├── serde v1.0.228
│   │   │   │   ├── serde_core v1.0.228
│   │   │   │   └── serde_derive v1.0.228 (proc-macro)
│   │   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │       ├── quote v1.0.45 (*)
│   │   │   │       └── syn v2.0.117 (*)
│   │   │   ├── subtle v2.6.1
│   │   │   └── zeroize v1.8.2
│   │   │       └── zeroize_derive v1.4.3 (proc-macro)
│   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │           ├── quote v1.0.45 (*)
│   │   │           └── syn v2.0.117 (*)
│   │   ├── data-encoding v2.10.0
│   │   ├── derive_more v2.1.1
│   │   │   └── derive_more-impl v2.1.1 (proc-macro)
│   │   │       ├── convert_case v0.10.0
│   │   │       │   └── unicode-segmentation v1.12.0
│   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │       ├── quote v1.0.45 (*)
│   │   │       ├── syn v2.0.117 (*)
│   │   │       └── unicode-xid v0.2.6
│   │   ├── digest v0.11.0-rc.10 (*)
│   │   ├── ed25519-dalek v3.0.0-pre.1
│   │   │   ├── curve25519-dalek v5.0.0-pre.1 (*)
│   │   │   ├── ed25519 v3.0.0-rc.4
│   │   │   │   ├── pkcs8 v0.11.0-rc.11
│   │   │   │   │   ├── der v0.8.0
│   │   │   │   │   │   ├── const-oid v0.10.2
│   │   │   │   │   │   ├── pem-rfc7468 v1.0.0
│   │   │   │   │   │   │   └── base64ct v1.8.3
│   │   │   │   │   │   └── zeroize v1.8.2 (*)
│   │   │   │   │   └── spki v0.8.0-rc.4
│   │   │   │   │       └── der v0.8.0 (*)
│   │   │   │   ├── serde v1.0.228 (*)
│   │   │   │   └── signature v3.0.0-rc.10
│   │   │   ├── rand_core v0.9.5 (*)
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── sha2 v0.11.0-rc.2
│   │   │   │   ├── cfg-if v1.0.4
│   │   │   │   ├── cpufeatures v0.2.17
│   │   │   │   └── digest v0.11.0-rc.10 (*)
│   │   │   ├── signature v3.0.0-rc.10
│   │   │   ├── subtle v2.6.1
│   │   │   └── zeroize v1.8.2 (*)
│   │   ├── n0-error v0.1.3
│   │   │   ├── anyhow v1.0.102
│   │   │   ├── n0-error-macros v0.1.3 (proc-macro)
│   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   └── syn v2.0.117 (*)
│   │   │   └── spez v0.1.2 (proc-macro)
│   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │       ├── quote v1.0.45 (*)
│   │   │       └── syn v2.0.117 (*)
│   │   ├── rand_core v0.9.5 (*)
│   │   ├── serde v1.0.228 (*)
│   │   ├── sha2 v0.11.0-rc.2 (*)
│   │   ├── url v2.5.8
│   │   │   ├── form_urlencoded v1.2.2
│   │   │   │   └── percent-encoding v2.3.2
│   │   │   ├── idna v1.1.0
│   │   │   │   ├── idna_adapter v1.2.1
│   │   │   │   │   ├── icu_normalizer v2.1.1
│   │   │   │   │   │   ├── icu_collections v2.1.1
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro)
│   │   │   │   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   │   │   │   ├── potential_utf v0.1.4
│   │   │   │   │   │   │   │   └── zerovec v0.11.5
│   │   │   │   │   │   │   │       ├── yoke v0.8.1
│   │   │   │   │   │   │   │       │   ├── stable_deref_trait v1.2.1
│   │   │   │   │   │   │   │       │   ├── yoke-derive v0.8.1 (proc-macro)
│   │   │   │   │   │   │   │       │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │   │   ├── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │   │   └── synstructure v0.13.2
│   │   │   │   │   │   │   │       │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │   │       ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │   │       └── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │   └── zerofrom v0.1.6
│   │   │   │   │   │   │   │       │       └── zerofrom-derive v0.1.6 (proc-macro)
│   │   │   │   │   │   │   │       │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │       │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │       │           ├── syn v2.0.117 (*)
│   │   │   │   │   │   │   │       │           └── synstructure v0.13.2 (*)
│   │   │   │   │   │   │   │       ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   │       └── zerovec-derive v0.11.2 (proc-macro)
│   │   │   │   │   │   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   │   │           ├── quote v1.0.45 (*)
│   │   │   │   │   │   │   │           └── syn v2.0.117 (*)
│   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   ├── icu_normalizer_data v2.1.1
│   │   │   │   │   │   ├── icu_provider v2.1.1
│   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   ├── icu_locale_core v2.1.1
│   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   ├── litemap v0.8.1
│   │   │   │   │   │   │   │   ├── tinystr v0.8.2
│   │   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   │   ├── writeable v0.6.2
│   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   ├── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   ├── zerotrie v0.2.3
│   │   │   │   │   │   │   │   ├── displaydoc v0.2.5 (proc-macro) (*)
│   │   │   │   │   │   │   │   ├── yoke v0.8.1 (*)
│   │   │   │   │   │   │   │   └── zerofrom v0.1.6 (*)
│   │   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   │   ├── smallvec v1.15.1
│   │   │   │   │   │   └── zerovec v0.11.5 (*)
│   │   │   │   │   └── icu_properties v2.1.2
│   │   │   │   │       ├── icu_collections v2.1.1 (*)
│   │   │   │   │       ├── icu_locale_core v2.1.1 (*)
│   │   │   │   │       ├── icu_properties_data v2.1.2
│   │   │   │   │       ├── icu_provider v2.1.1 (*)
│   │   │   │   │       ├── zerotrie v0.2.3 (*)
│   │   │   │   │       └── zerovec v0.11.5 (*)
│   │   │   │   ├── smallvec v1.15.1
│   │   │   │   └── utf8_iter v1.0.4
│   │   │   ├── percent-encoding v2.3.2
│   │   │   ├── serde v1.0.228 (*)
│   │   │   └── serde_derive v1.0.228 (proc-macro) (*)
│   │   ├── zeroize v1.8.2 (*)
│   │   └── zeroize_derive v1.4.3 (proc-macro) (*)
│   ├── serde v1.0.228 (*)
│   └── thiserror v2.0.18
│       └── thiserror-impl v2.0.18 (proc-macro)
│           ├── proc-macro2 v1.0.106 (*)
│           ├── quote v1.0.45 (*)
│           └── syn v2.0.117 (*)
├── iroh-base v0.97.0 (*)
├── iroh-gossip v0.97.0
│   ├── blake3 v1.8.3
│   │   ├── arrayref v0.3.9
│   │   ├── arrayvec v0.7.6
│   │   ├── cfg-if v1.0.4
│   │   ├── constant_time_eq v0.4.2
│   │   └── cpufeatures v0.2.17
│   ├── bytes v1.11.1
│   │   └── serde v1.0.228 (*)
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1 (*)
│   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   ├── futures-concurrency v7.7.1
│   │   ├── fixedbitset v0.5.7
│   │   ├── futures-core v0.3.32
│   │   ├── futures-lite v2.6.1
│   │   │   ├── fastrand v2.3.0
│   │   │   ├── futures-core v0.3.32
│   │   │   ├── futures-io v0.3.32
│   │   │   ├── parking v2.2.1
│   │   │   └── pin-project-lite v0.2.17
│   │   ├── pin-project v1.1.11
│   │   │   └── pin-project-internal v1.1.11 (proc-macro)
│   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │       ├── quote v1.0.45 (*)
│   │   │       └── syn v2.0.117 (*)
│   │   └── smallvec v1.15.1
│   ├── futures-lite v2.6.1 (*)
│   ├── futures-util v0.3.32
│   │   ├── futures-channel v0.3.32
│   │   │   ├── futures-core v0.3.32
│   │   │   └── futures-sink v0.3.32
│   │   ├── futures-core v0.3.32
│   │   ├── futures-io v0.3.32
│   │   ├── futures-macro v0.3.32 (proc-macro)
│   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   ├── quote v1.0.45 (*)
│   │   │   └── syn v2.0.117 (*)
│   │   ├── futures-sink v0.3.32
│   │   ├── futures-task v0.3.32
│   │   ├── memchr v2.8.0
│   │   ├── pin-project-lite v0.2.17
│   │   └── slab v0.4.12
│   ├── hex v0.4.3
│   ├── indexmap v2.13.0
│   │   ├── equivalent v1.0.2
│   │   └── hashbrown v0.16.1
│   │       ├── allocator-api2 v0.2.21
│   │       ├── equivalent v1.0.2
│   │       └── foldhash v0.2.0
│   ├── iroh v0.97.0
│   │   ├── backon v1.6.0
│   │   │   ├── fastrand v2.3.0
│   │   │   └── tokio v1.50.0
│   │   │       ├── bytes v1.11.1 (*)
│   │   │       ├── libc v0.2.183
│   │   │       ├── mio v1.1.1
│   │   │       │   └── libc v0.2.183
│   │   │       ├── pin-project-lite v0.2.17
│   │   │       ├── socket2 v0.6.3
│   │   │       │   └── libc v0.2.183
│   │   │       └── tokio-macros v2.6.1 (proc-macro)
│   │   │           ├── proc-macro2 v1.0.106 (*)
│   │   │           ├── quote v1.0.45 (*)
│   │   │           └── syn v2.0.117 (*)
│   │   ├── bytes v1.11.1 (*)
│   │   ├── data-encoding v2.10.0
│   │   ├── derive_more v2.1.1 (*)
│   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   │   ├── futures-util v0.3.32 (*)
│   │   ├── hickory-resolver v0.25.2
│   │   │   ├── cfg-if v1.0.4
│   │   │   ├── futures-util v0.3.32 (*)
│   │   │   ├── hickory-proto v0.25.2
│   │   │   │   ├── async-trait v0.1.89 (proc-macro)
│   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── cfg-if v1.0.4
│   │   │   │   ├── data-encoding v2.10.0
│   │   │   │   ├── enum-as-inner v0.6.1 (proc-macro)
│   │   │   │   │   ├── heck v0.5.0
│   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   ├── futures-channel v0.3.32 (*)
│   │   │   │   ├── futures-io v0.3.32
│   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   ├── h2 v0.4.13
│   │   │   │   │   ├── atomic-waker v1.1.2
│   │   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   │   ├── fnv v1.0.7
│   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   ├── futures-sink v0.3.32
│   │   │   │   │   ├── http v1.4.0
│   │   │   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   │   │   └── itoa v1.0.17
│   │   │   │   │   ├── indexmap v2.13.0 (*)
│   │   │   │   │   ├── slab v0.4.12
│   │   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   │   ├── tokio-util v0.7.18
│   │   │   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   │   ├── futures-sink v0.3.32
│   │   │   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   │   │   └── tokio v1.50.0 (*)
│   │   │   │   │   └── tracing v0.1.44
│   │   │   │   │       ├── log v0.4.29
│   │   │   │   │       ├── pin-project-lite v0.2.17
│   │   │   │   │       ├── tracing-attributes v0.1.31 (proc-macro)
│   │   │   │   │       │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │       │   ├── quote v1.0.45 (*)
│   │   │   │   │       │   └── syn v2.0.117 (*)
│   │   │   │   │       └── tracing-core v0.1.36
│   │   │   │   │           └── once_cell v1.21.4
│   │   │   │   │               ├── critical-section v1.2.0
│   │   │   │   │               └── portable-atomic v1.13.1
│   │   │   │   │                   └── serde v1.0.228 (*)
│   │   │   │   ├── http v1.4.0 (*)
│   │   │   │   ├── idna v1.1.0 (*)
│   │   │   │   ├── ipnet v2.12.0
│   │   │   │   ├── once_cell v1.21.4 (*)
│   │   │   │   ├── rand v0.9.2
│   │   │   │   │   ├── rand_chacha v0.9.0
│   │   │   │   │   │   ├── ppv-lite86 v0.2.21
│   │   │   │   │   │   │   └── zerocopy v0.8.42
│   │   │   │   │   │   └── rand_core v0.9.5 (*)
│   │   │   │   │   └── rand_core v0.9.5 (*)
│   │   │   │   ├── rustls v0.23.37
│   │   │   │   │   ├── log v0.4.29
│   │   │   │   │   ├── once_cell v1.21.4 (*)
│   │   │   │   │   ├── ring v0.17.14
│   │   │   │   │   │   ├── cfg-if v1.0.4
│   │   │   │   │   │   ├── getrandom v0.2.17
│   │   │   │   │   │   │   ├── cfg-if v1.0.4
│   │   │   │   │   │   │   └── libc v0.2.183
│   │   │   │   │   │   └── untrusted v0.9.0
│   │   │   │   │   ├── rustls-pki-types v1.14.0
│   │   │   │   │   │   └── zeroize v1.8.2 (*)
│   │   │   │   │   ├── rustls-webpki v0.103.9
│   │   │   │   │   │   ├── ring v0.17.14 (*)
│   │   │   │   │   │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   │   └── untrusted v0.9.0
│   │   │   │   │   ├── subtle v2.6.1
│   │   │   │   │   └── zeroize v1.8.2 (*)
│   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   ├── tinyvec v1.11.0
│   │   │   │   │   └── tinyvec_macros v0.1.1
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   ├── tokio-rustls v0.26.4
│   │   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   │   └── tokio v1.50.0 (*)
│   │   │   │   ├── tracing v0.1.44 (*)
│   │   │   │   └── url v2.5.8 (*)
│   │   │   ├── moka v0.12.14
│   │   │   │   ├── crossbeam-channel v0.5.15
│   │   │   │   │   └── crossbeam-utils v0.8.21
│   │   │   │   ├── crossbeam-epoch v0.9.18
│   │   │   │   │   └── crossbeam-utils v0.8.21
│   │   │   │   ├── crossbeam-utils v0.8.21
│   │   │   │   ├── equivalent v1.0.2
│   │   │   │   ├── parking_lot v0.12.5
│   │   │   │   │   ├── lock_api v0.4.14
│   │   │   │   │   │   └── scopeguard v1.2.0
│   │   │   │   │   └── parking_lot_core v0.9.12
│   │   │   │   │       ├── cfg-if v1.0.4
│   │   │   │   │       ├── libc v0.2.183
│   │   │   │   │       └── smallvec v1.15.1
│   │   │   │   ├── portable-atomic v1.13.1 (*)
│   │   │   │   ├── smallvec v1.15.1
│   │   │   │   ├── tagptr v0.2.0
│   │   │   │   └── uuid v1.22.0
│   │   │   │       └── getrandom v0.4.2
│   │   │   │           ├── cfg-if v1.0.4
│   │   │   │           └── libc v0.2.183
│   │   │   ├── once_cell v1.21.4 (*)
│   │   │   ├── parking_lot v0.12.5 (*)
│   │   │   ├── rand v0.9.2 (*)
│   │   │   ├── resolv-conf v0.7.6
│   │   │   ├── rustls v0.23.37 (*)
│   │   │   ├── smallvec v1.15.1
│   │   │   ├── thiserror v2.0.18 (*)
│   │   │   ├── tokio v1.50.0 (*)
│   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   └── tracing v0.1.44 (*)
│   │   ├── http v1.4.0 (*)
│   │   ├── ipnet v2.12.0
│   │   ├── iroh-base v0.97.0 (*)
│   │   ├── iroh-metrics v0.38.3
│   │   │   ├── iroh-metrics-derive v0.4.1 (proc-macro)
│   │   │   │   ├── heck v0.5.0
│   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   └── syn v2.0.117 (*)
│   │   │   ├── itoa v1.0.17
│   │   │   ├── n0-error v0.1.3 (*)
│   │   │   ├── portable-atomic v1.13.1 (*)
│   │   │   ├── postcard v1.1.3
│   │   │   │   ├── cobs v0.3.0
│   │   │   │   │   └── thiserror v2.0.18 (*)
│   │   │   │   ├── heapless v0.7.17
│   │   │   │   │   ├── hash32 v0.2.1
│   │   │   │   │   │   └── byteorder v1.5.0
│   │   │   │   │   ├── serde v1.0.228 (*)
│   │   │   │   │   ├── spin v0.9.8
│   │   │   │   │   │   └── lock_api v0.4.14 (*)
│   │   │   │   │   └── stable_deref_trait v1.2.1
│   │   │   │   ├── postcard-derive v0.2.2 (proc-macro)
│   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   └── serde v1.0.228 (*)
│   │   │   ├── ryu v1.0.23
│   │   │   ├── serde v1.0.228 (*)
│   │   │   └── tracing v0.1.44 (*)
│   │   ├── iroh-relay v0.97.0
│   │   │   ├── blake3 v1.8.3 (*)
│   │   │   ├── bytes v1.11.1 (*)
│   │   │   ├── data-encoding v2.10.0
│   │   │   ├── derive_more v2.1.1 (*)
│   │   │   ├── hickory-resolver v0.25.2 (*)
│   │   │   ├── http v1.4.0 (*)
│   │   │   ├── http-body-util v0.1.3
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── futures-core v0.3.32
│   │   │   │   ├── http v1.4.0 (*)
│   │   │   │   ├── http-body v1.0.1
│   │   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   │   └── http v1.4.0 (*)
│   │   │   │   └── pin-project-lite v0.2.17
│   │   │   ├── hyper v1.8.1
│   │   │   │   ├── atomic-waker v1.1.2
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── futures-channel v0.3.32 (*)
│   │   │   │   ├── futures-core v0.3.32
│   │   │   │   ├── http v1.4.0 (*)
│   │   │   │   ├── http-body v1.0.1 (*)
│   │   │   │   ├── httparse v1.10.1
│   │   │   │   ├── httpdate v1.0.3
│   │   │   │   ├── itoa v1.0.17
│   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   ├── pin-utils v0.1.0
│   │   │   │   ├── smallvec v1.15.1
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   └── want v0.3.1
│   │   │   │       └── try-lock v0.2.5
│   │   │   ├── hyper-util v0.1.20
│   │   │   │   ├── base64 v0.22.1
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── futures-channel v0.3.32 (*)
│   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   ├── http v1.4.0 (*)
│   │   │   │   ├── http-body v1.0.1 (*)
│   │   │   │   ├── hyper v1.8.1 (*)
│   │   │   │   ├── ipnet v2.12.0
│   │   │   │   ├── libc v0.2.183
│   │   │   │   ├── percent-encoding v2.3.2
│   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   ├── socket2 v0.6.3 (*)
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   ├── tower-service v0.3.3
│   │   │   │   └── tracing v0.1.44 (*)
│   │   │   ├── iroh-base v0.97.0 (*)
│   │   │   ├── iroh-metrics v0.38.3 (*)
│   │   │   ├── lru v0.16.3
│   │   │   │   └── hashbrown v0.16.1 (*)
│   │   │   ├── n0-error v0.1.3 (*)
│   │   │   ├── n0-future v0.3.2
│   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   ├── futures-buffered v0.2.13
│   │   │   │   │   ├── cordyceps v0.3.4
│   │   │   │   │   ├── diatomic-waker v0.2.3
│   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   │   └── spin v0.10.0
│   │   │   │   ├── futures-lite v2.6.1 (*)
│   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   ├── pin-project v1.1.11 (*)
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   └── tokio-util v0.7.18 (*)
│   │   │   ├── noq v0.17.0
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── noq-proto v0.16.0
│   │   │   │   │   ├── aes-gcm v0.10.3
│   │   │   │   │   │   ├── aead v0.5.2
│   │   │   │   │   │   │   ├── crypto-common v0.1.7
│   │   │   │   │   │   │   │   ├── generic-array v0.14.7
│   │   │   │   │   │   │   │   │   └── typenum v1.19.0
│   │   │   │   │   │   │   │   └── typenum v1.19.0
│   │   │   │   │   │   │   └── generic-array v0.14.7 (*)
│   │   │   │   │   │   ├── aes v0.8.4
│   │   │   │   │   │   │   ├── cfg-if v1.0.4
│   │   │   │   │   │   │   ├── cipher v0.4.4
│   │   │   │   │   │   │   │   ├── crypto-common v0.1.7 (*)
│   │   │   │   │   │   │   │   └── inout v0.1.4
│   │   │   │   │   │   │   │       └── generic-array v0.14.7 (*)
│   │   │   │   │   │   │   └── cpufeatures v0.2.17
│   │   │   │   │   │   ├── cipher v0.4.4 (*)
│   │   │   │   │   │   ├── ctr v0.9.2
│   │   │   │   │   │   │   └── cipher v0.4.4 (*)
│   │   │   │   │   │   ├── ghash v0.5.1
│   │   │   │   │   │   │   ├── opaque-debug v0.3.1
│   │   │   │   │   │   │   └── polyval v0.6.2
│   │   │   │   │   │   │       ├── cfg-if v1.0.4
│   │   │   │   │   │   │       ├── cpufeatures v0.2.17
│   │   │   │   │   │   │       ├── opaque-debug v0.3.1
│   │   │   │   │   │   │       └── universal-hash v0.5.1
│   │   │   │   │   │   │           ├── crypto-common v0.1.7 (*)
│   │   │   │   │   │   │           └── subtle v2.6.1
│   │   │   │   │   │   └── subtle v2.6.1
│   │   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   │   ├── derive_more v2.1.1 (*)
│   │   │   │   │   ├── enum-assoc v1.3.0 (proc-macro)
│   │   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   │   ├── fastbloom v0.14.1
│   │   │   │   │   │   ├── getrandom v0.3.4 (*)
│   │   │   │   │   │   ├── libm v0.2.16
│   │   │   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   │   │   └── siphasher v1.0.2
│   │   │   │   │   ├── identity-hash v0.1.0
│   │   │   │   │   ├── lru-slab v0.1.2
│   │   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   │   ├── ring v0.17.14 (*)
│   │   │   │   │   ├── rustc-hash v2.1.1
│   │   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   │   ├── slab v0.4.12
│   │   │   │   │   ├── sorted-index-buffer v0.2.1
│   │   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   │   ├── tinyvec v1.11.0 (*)
│   │   │   │   │   └── tracing v0.1.44 (*)
│   │   │   │   ├── noq-udp v0.9.0
│   │   │   │   │   ├── libc v0.2.183
│   │   │   │   │   ├── socket2 v0.6.3 (*)
│   │   │   │   │   └── tracing v0.1.44 (*)
│   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   ├── rustc-hash v2.1.1
│   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   ├── socket2 v0.6.3 (*)
│   │   │   │   ├── thiserror v2.0.18 (*)
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   ├── tokio-stream v0.1.18
│   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   │   └── tokio-util v0.7.18 (*)
│   │   │   │   └── tracing v0.1.44 (*)
│   │   │   ├── noq-proto v0.16.0 (*)
│   │   │   ├── num_enum v0.7.6
│   │   │   │   ├── num_enum_derive v0.7.6 (proc-macro)
│   │   │   │   │   ├── proc-macro-crate v3.5.0
│   │   │   │   │   │   └── toml_edit v0.25.5+spec-1.1.0
│   │   │   │   │   │       ├── indexmap v2.13.0 (*)
│   │   │   │   │   │       ├── toml_datetime v1.0.1+spec-1.1.0
│   │   │   │   │   │       ├── toml_parser v1.0.10+spec-1.1.0
│   │   │   │   │   │       │   └── winnow v1.0.0
│   │   │   │   │   │       └── winnow v1.0.0
│   │   │   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   │   │   ├── quote v1.0.45 (*)
│   │   │   │   │   └── syn v2.0.117 (*)
│   │   │   │   └── rustversion v1.0.22 (proc-macro)
│   │   │   ├── pin-project v1.1.11 (*)
│   │   │   ├── pkarr v5.0.2
│   │   │   │   ├── base32 v0.5.1
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── document-features v0.2.12 (proc-macro)
│   │   │   │   │   └── litrs v1.0.0
│   │   │   │   ├── ed25519-dalek v3.0.0-pre.1 (*)
│   │   │   │   ├── getrandom v0.3.4 (*)
│   │   │   │   ├── ntimestamp v1.0.0
│   │   │   │   │   ├── base32 v0.5.1
│   │   │   │   │   ├── document-features v0.2.12 (proc-macro) (*)
│   │   │   │   │   ├── getrandom v0.2.17 (*)
│   │   │   │   │   ├── httpdate v1.0.3
│   │   │   │   │   ├── once_cell v1.21.4 (*)
│   │   │   │   │   └── serde v1.0.228 (*)
│   │   │   │   ├── self_cell v1.2.2
│   │   │   │   ├── serde v1.0.228 (*)
│   │   │   │   ├── simple-dns v0.9.3
│   │   │   │   │   └── bitflags v2.11.0
│   │   │   │   └── thiserror v2.0.18 (*)
│   │   │   ├── postcard v1.1.3 (*)
│   │   │   ├── rand v0.9.2 (*)
│   │   │   ├── reqwest v0.12.28
│   │   │   │   ├── base64 v0.22.1
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── futures-core v0.3.32
│   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   ├── http v1.4.0 (*)
│   │   │   │   ├── http-body v1.0.1 (*)
│   │   │   │   ├── http-body-util v0.1.3 (*)
│   │   │   │   ├── hyper v1.8.1 (*)
│   │   │   │   ├── hyper-rustls v0.27.7
│   │   │   │   │   ├── http v1.4.0 (*)
│   │   │   │   │   ├── hyper v1.8.1 (*)
│   │   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   │   ├── tower-service v0.3.3
│   │   │   │   │   └── webpki-roots v1.0.6
│   │   │   │   │       └── rustls-pki-types v1.14.0 (*)
│   │   │   │   ├── hyper-util v0.1.20 (*)
│   │   │   │   ├── log v0.4.29
│   │   │   │   ├── percent-encoding v2.3.2
│   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   ├── rustls v0.23.37 (*)
│   │   │   │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   ├── serde v1.0.228 (*)
│   │   │   │   ├── serde_urlencoded v0.7.1
│   │   │   │   │   ├── form_urlencoded v1.2.2 (*)
│   │   │   │   │   ├── itoa v1.0.17
│   │   │   │   │   ├── ryu v1.0.23
│   │   │   │   │   └── serde v1.0.228 (*)
│   │   │   │   ├── sync_wrapper v1.0.2
│   │   │   │   │   └── futures-core v0.3.32
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   ├── tokio-util v0.7.18 (*)
│   │   │   │   ├── tower v0.5.3
│   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   │   ├── sync_wrapper v1.0.2 (*)
│   │   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   │   ├── tower-layer v0.3.3
│   │   │   │   │   └── tower-service v0.3.3
│   │   │   │   ├── tower-http v0.6.8
│   │   │   │   │   ├── bitflags v2.11.0
│   │   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   │   ├── futures-util v0.3.32 (*)
│   │   │   │   │   ├── http v1.4.0 (*)
│   │   │   │   │   ├── http-body v1.0.1 (*)
│   │   │   │   │   ├── iri-string v0.7.10
│   │   │   │   │   ├── pin-project-lite v0.2.17
│   │   │   │   │   ├── tower v0.5.3 (*)
│   │   │   │   │   ├── tower-layer v0.3.3
│   │   │   │   │   └── tower-service v0.3.3
│   │   │   │   ├── tower-service v0.3.3
│   │   │   │   ├── url v2.5.8 (*)
│   │   │   │   └── webpki-roots v1.0.6 (*)
│   │   │   ├── rustls v0.23.37 (*)
│   │   │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   ├── serde v1.0.228 (*)
│   │   │   ├── serde_bytes v0.11.19
│   │   │   │   └── serde_core v1.0.228
│   │   │   ├── strum v0.28.0
│   │   │   │   └── strum_macros v0.28.0 (proc-macro)
│   │   │   │       ├── heck v0.5.0
│   │   │   │       ├── proc-macro2 v1.0.106 (*)
│   │   │   │       ├── quote v1.0.45 (*)
│   │   │   │       └── syn v2.0.117 (*)
│   │   │   ├── tokio v1.50.0 (*)
│   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   ├── tokio-util v0.7.18 (*)
│   │   │   ├── tokio-websockets v0.12.3
│   │   │   │   ├── base64 v0.22.1
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── futures-core v0.3.32
│   │   │   │   ├── futures-sink v0.3.32
│   │   │   │   ├── getrandom v0.3.4 (*)
│   │   │   │   ├── http v1.4.0 (*)
│   │   │   │   ├── httparse v1.10.1
│   │   │   │   ├── rand v0.9.2 (*)
│   │   │   │   ├── ring v0.17.14 (*)
│   │   │   │   ├── rustls-pki-types v1.14.0 (*)
│   │   │   │   ├── simdutf8 v0.1.5
│   │   │   │   ├── tokio v1.50.0 (*)
│   │   │   │   ├── tokio-rustls v0.26.4 (*)
│   │   │   │   └── tokio-util v0.7.18 (*)
│   │   │   ├── tracing v0.1.44 (*)
│   │   │   ├── url v2.5.8 (*)
│   │   │   ├── webpki-roots v1.0.6 (*)
│   │   │   └── z32 v1.3.0
│   │   ├── n0-error v0.1.3 (*)
│   │   ├── n0-future v0.3.2 (*)
│   │   ├── n0-watcher v0.6.1
│   │   │   ├── derive_more v2.1.1 (*)
│   │   │   ├── n0-error v0.1.3 (*)
│   │   │   └── n0-future v0.3.2 (*)
│   │   ├── netwatch v0.15.0
│   │   │   ├── atomic-waker v1.1.2
│   │   │   ├── bytes v1.11.1 (*)
│   │   │   ├── libc v0.2.183
│   │   │   ├── n0-error v0.1.3 (*)
│   │   │   ├── n0-future v0.3.2 (*)
│   │   │   ├── n0-watcher v0.6.1 (*)
│   │   │   ├── netdev v0.40.1
│   │   │   │   ├── ipnet v2.12.0
│   │   │   │   ├── libc v0.2.183
│   │   │   │   ├── mac-addr v0.3.0
│   │   │   │   ├── netlink-packet-core v0.8.1
│   │   │   │   │   └── paste v1.0.15 (proc-macro)
│   │   │   │   ├── netlink-packet-route v0.29.0
│   │   │   │   │   ├── bitflags v2.11.0
│   │   │   │   │   ├── libc v0.2.183
│   │   │   │   │   ├── log v0.4.29
│   │   │   │   │   └── netlink-packet-core v0.8.1 (*)
│   │   │   │   └── netlink-sys v0.8.8
│   │   │   │       ├── bytes v1.11.1 (*)
│   │   │   │       ├── futures-util v0.3.32 (*)
│   │   │   │       ├── libc v0.2.183
│   │   │   │       ├── log v0.4.29
│   │   │   │       └── tokio v1.50.0 (*)
│   │   │   ├── netlink-packet-core v0.8.1 (*)
│   │   │   ├── netlink-packet-route v0.29.0 (*)
│   │   │   ├── netlink-proto v0.12.0
│   │   │   │   ├── bytes v1.11.1 (*)
│   │   │   │   ├── futures v0.3.32
│   │   │   │   │   ├── futures-channel v0.3.32 (*)
│   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   ├── futures-executor v0.3.32
│   │   │   │   │   │   ├── futures-core v0.3.32
│   │   │   │   │   │   ├── futures-task v0.3.32
│   │   │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │   │   ├── futures-io v0.3.32
│   │   │   │   │   ├── futures-sink v0.3.32
│   │   │   │   │   ├── futures-task v0.3.32
│   │   │   │   │   └── futures-util v0.3.32 (*)
│   │   │   │   ├── log v0.4.29
│   │   │   │   ├── netlink-packet-core v0.8.1 (*)
│   │   │   │   ├── netlink-sys v0.8.8 (*)
│   │   │   │   └── thiserror v2.0.18 (*)
│   │   │   ├── netlink-sys v0.8.8 (*)
│   │   │   ├── noq-udp v0.9.0 (*)
│   │   │   ├── pin-project-lite v0.2.17
│   │   │   ├── socket2 v0.6.3 (*)
│   │   │   ├── time v0.3.47
│   │   │   │   ├── deranged v0.5.8
│   │   │   │   │   └── powerfmt v0.2.0
│   │   │   │   ├── num-conv v0.2.0
│   │   │   │   ├── powerfmt v0.2.0
│   │   │   │   └── time-core v0.1.8
│   │   │   ├── tokio v1.50.0 (*)
│   │   │   ├── tokio-util v0.7.18 (*)
│   │   │   └── tracing v0.1.44 (*)
│   │   ├── noq v0.17.0 (*)
│   │   ├── noq-proto v0.16.0 (*)
│   │   ├── noq-udp v0.9.0 (*)
│   │   ├── papaya v0.2.3
│   │   │   ├── equivalent v1.0.2
│   │   │   └── seize v0.5.1
│   │   │       └── libc v0.2.183
│   │   ├── pin-project v1.1.11 (*)
│   │   ├── pkarr v5.0.2 (*)
│   │   ├── pkcs8 v0.11.0-rc.11 (*)
│   │   ├── portable-atomic v1.13.1 (*)
│   │   ├── rand v0.9.2 (*)
│   │   ├── reqwest v0.12.28 (*)
│   │   ├── rustc-hash v2.1.1
│   │   ├── rustls v0.23.37 (*)
│   │   ├── rustls-pki-types v1.14.0 (*)
│   │   ├── rustls-webpki v0.103.9 (*)
│   │   ├── serde v1.0.228 (*)
│   │   ├── smallvec v1.15.1
│   │   ├── strum v0.28.0 (*)
│   │   ├── sync_wrapper v1.0.2 (*)
│   │   ├── tokio v1.50.0 (*)
│   │   ├── tokio-stream v0.1.18 (*)
│   │   ├── tokio-util v0.7.18 (*)
│   │   ├── tracing v0.1.44 (*)
│   │   ├── url v2.5.8 (*)
│   │   └── webpki-roots v1.0.6 (*)
│   ├── iroh-base v0.97.0 (*)
│   ├── iroh-metrics v0.38.3 (*)
│   ├── irpc v0.13.0
│   │   ├── futures-util v0.3.32 (*)
│   │   ├── irpc-derive v0.10.0 (proc-macro)
│   │   │   ├── proc-macro2 v1.0.106 (*)
│   │   │   ├── quote v1.0.45 (*)
│   │   │   └── syn v2.0.117 (*)
│   │   ├── n0-error v0.1.3 (*)
│   │   ├── n0-future v0.3.2 (*)
│   │   ├── serde v1.0.228 (*)
│   │   ├── tokio v1.50.0 (*)
│   │   ├── tokio-util v0.7.18 (*)
│   │   └── tracing v0.1.44 (*)
│   ├── n0-error v0.1.3 (*)
│   ├── n0-future v0.3.2 (*)
│   ├── postcard v1.1.3 (*)
│   ├── rand v0.9.2 (*)
│   ├── serde v1.0.228 (*)
│   ├── tokio v1.50.0 (*)
│   ├── tokio-util v0.7.18 (*)
│   └── tracing v0.1.44 (*)
├── iroh-tickets v0.4.0
│   ├── data-encoding v2.10.0
│   ├── derive_more v2.1.1 (*)
│   ├── iroh-base v0.97.0 (*)
│   ├── n0-error v0.1.3 (*)
│   ├── postcard v1.1.3 (*)
│   └── serde v1.0.228 (*)
├── postcard v1.1.3 (*)
├── serde v1.0.228 (*)
└── thiserror v2.0.18 (*)

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-ticket --features iroh'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.54s

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo test -p aspen-ticket --test ui'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.35s
     Running tests/ui.rs (target/debug/deps/ui-cc2d6759ae145a76)

running 1 test
warning: patch `cargo-hyperlight v0.1.5 (/home/brittonr/git/aspen/vendor/cargo-hyperlight)` was not used in the crate graph
warning: patch `uhlc v0.8.2 (/home/brittonr/git/aspen/vendor/uhlc)` was not used in the crate graph
warning: patch `snix-glue v0.1.0 (/home/brittonr/git/aspen/vendor/snix-glue)` was not used in the crate graph
help: Check that the patched package version and available features are compatible
      with the dependency requirements. If the patch has a different version from
      what is locked in the Cargo.lock file, run `cargo update` to use the new
      version. This may also occur with an optional dependency that is not enabled.
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-ticket-tests v0.0.0 (/home/brittonr/git/aspen/crates/aspen-ticket/target/tests/trybuild/aspen-ticket)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.22s


test [0m[1mtests/ui/iroh_helpers_require_feature.rs[0m ... [0m[32mok
[0m

test iroh_helpers_require_feature ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.87s


## Direct-consumer compile rails

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-ci-executor-vm'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-fuse v0.1.0 (/home/brittonr/git/aspen/crates/aspen-fuse)
    Checking aspen-ci-executor-vm v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ci-executor-vm)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.60s

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-cluster-handler'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-cluster-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-handler)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.02s

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-cluster'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.37s

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-rpc-handlers'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Checking aspen-cluster-handler v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster-handler)
    Checking aspen-rpc-handlers v0.1.0 (/home/brittonr/git/aspen/crates/aspen-rpc-handlers)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.00s

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-client'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.84s

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo check -p aspen-ci'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
    Checking aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
    Checking aspen-cluster v0.1.0 (/home/brittonr/git/aspen/crates/aspen-cluster)
    Checking aspen-forge v0.1.0 (/home/brittonr/git/aspen/crates/aspen-forge)
    Checking aspen-ci v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ci)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.73s

## Representative transitive re-export leak proofs

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo tree -p aspen-fuse -e features -i aspen-ticket'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
aspen-ticket v0.1.0 (/home/brittonr/git/aspen/crates/aspen-ticket)
└── aspen-ticket feature "iroh"
    └── aspen-client v0.1.0 (/home/brittonr/git/aspen/crates/aspen-client)
        └── aspen-client feature "default"
            └── aspen-fuse v0.1.0 (/home/brittonr/git/aspen/crates/aspen-fuse)
                └── aspen-fuse feature "default" (command-line)

### `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo tree -p aspen-cli -e features -i aspen-ticket'`

warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /home/brittonr/git/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /home/brittonr/git/aspen/Cargo.toml
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

