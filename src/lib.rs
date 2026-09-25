#![feature(register_tool)]
#![register_tool(tigerstyle)]

mod bounded;

pub mod addressable_actor;

#[doc(hidden)]
#[path = "effects/mod.rs"]
pub mod actions;
#[doc(hidden)]
#[path = "audit/ast_grep.rs"]
pub mod ast_grep_runtime_authority_core;
#[doc(hidden)]
#[path = "chunk/store.rs"]
pub mod blocks;
#[doc(hidden)]
#[path = "prod/soak.rs"]
pub mod burnin;
#[doc(hidden)]
#[path = "capability/mod.rs"]
pub mod capabilities_core;
#[doc(hidden)]
#[path = "typed/storage.rs"]
pub mod cells;
#[path = "cluster.rs"]
pub mod cluster;
pub mod cluster_harness;
#[doc(hidden)]
#[path = "preserves/rail.rs"]
pub mod codec;
#[doc(hidden)]
#[path = "project/config/portability.rs"]
pub mod config_portability_core;
pub mod content_replication;
pub mod content_store_adapter;
#[doc(hidden)]
#[path = "operator/context/profile.rs"]
pub mod context_profile_core;
#[doc(hidden)]
#[path = "protocol/session.rs"]
pub mod conversation;
pub mod coordination_delivery;
#[doc(hidden)]
#[path = "node/identity.rs"]
pub mod credential;
#[doc(hidden)]
#[path = "retention/mod.rs"]
pub mod custody;
#[doc(hidden)]
#[path = "node/daemon.rs"]
pub mod daemon_core;
pub mod dag_sync;
#[doc(hidden)]
#[path = "delivery/idempotency.rs"]
pub mod dedupe;
#[doc(hidden)]
#[path = "authority/mod.rs"]
pub mod delegation;
#[doc(hidden)]
#[path = "schema/identity.rs"]
pub mod descriptor;
#[doc(hidden)]
#[path = "testing/distributed.rs"]
pub mod distributed_core;
#[doc(hidden)]
#[path = "testing/drift.rs"]
pub mod drift_core;
#[doc(hidden)]
#[path = "operator/gateway.rs"]
pub mod edgeway;
#[doc(hidden)]
#[path = "project/effective/config.rs"]
pub mod effective_config_core;
#[doc(hidden)]
#[path = "runtime/mod.rs"]
pub mod engine;
#[cfg(feature = "executable-extents")]
pub mod executable_extent;
#[doc(hidden)]
#[path = "plugin/host.rs"]
pub mod extension;
pub mod fabric;
pub mod fabric_consistency;
pub mod fabric_crypto_identity;
pub mod fabric_durability;
pub mod fabric_execution;
pub mod fabric_membership;
pub mod fabric_observability;
pub mod fabric_simulation;
pub mod fabric_time;
pub mod fabric_transport;
#[doc(hidden)]
#[path = "error/mod.rs"]
pub mod failures;
#[doc(hidden)]
#[path = "testing/hardening.rs"]
pub mod hardening_core;
#[doc(hidden)]
#[path = "catalog/mod.rs"]
pub mod inventory;
#[doc(hidden)]
#[path = "catalog/mcp.rs"]
pub mod inventory_api;
#[doc(hidden)]
#[path = "ledger/mod.rs"]
pub mod journal;
#[doc(hidden)]
#[path = "node/runtime.rs"]
pub mod kernel;
#[doc(hidden)]
#[path = "prod/readiness.rs"]
pub mod launch;
#[doc(hidden)]
#[path = "evidence/chain.rs"]
pub mod lineage;
#[doc(hidden)]
#[path = "provenance/mod.rs"]
pub mod lineage_meta;
pub mod live_binding_adoption;
pub mod local_store;
#[doc(hidden)]
#[path = "nixos/vm.rs"]
pub mod machine;
#[path = "materialization.rs"]
pub mod materialization;
#[doc(hidden)]
#[path = "eval/cache.rs"]
pub mod memo;
#[doc(hidden)]
#[path = "federation/mod.rs"]
pub mod mesh;
#[doc(hidden)]
#[path = "remote/dataspace.rs"]
pub mod meshspace;
#[doc(hidden)]
#[path = "upgrades/mod.rs"]
pub mod migrations;
#[doc(hidden)]
#[path = "testing/multinode.rs"]
pub mod multinode_core;
#[doc(hidden)]
#[path = "transcripts/mod.rs"]
pub mod narratives;
#[path = "node/nativesystemextension.rs"]
pub mod nativehostnode;
#[doc(hidden)]
#[path = "iroh/exchange.rs"]
pub mod netlink;
#[doc(hidden)]
#[path = "node/profile/config.rs"]
pub mod node_profile_config_core;
#[doc(hidden)]
#[path = "node/service/fsm.rs"]
pub mod node_service_fsm_core;
#[path = "node/state.rs"]
pub mod node_state;
#[doc(hidden)]
#[path = "artifacts/mod.rs"]
pub mod objects;
#[doc(hidden)]
#[path = "coordination/mod.rs"]
pub mod orchestration;
#[doc(hidden)]
#[path = "peer/bootstrap.rs"]
pub mod peering;
#[doc(hidden)]
#[path = "lifecycle/mod.rs"]
pub mod phases;
#[doc(hidden)]
#[path = "operator/dogfood.rs"]
pub mod pilot;
#[doc(hidden)]
#[path = "prod/pilot.rs"]
pub mod pilot_readiness;
#[doc(hidden)]
#[path = "deterministic/replay.rs"]
pub mod playback;
pub mod profiling;
pub mod prolly_map;
#[doc(hidden)]
#[path = "testing/prooftrace.rs"]
pub mod proof_trace_core;
#[doc(hidden)]
#[path = "evidence/mod.rs"]
pub mod proofs;
#[doc(hidden)]
#[path = "propagation/mod.rs"]
pub mod propagation_core;
#[doc(hidden)]
#[path = "octet/gate.rs"]
pub mod quality;
#[doc(hidden)]
#[path = "raft/control/plane.rs"]
pub mod quorum;
#[doc(hidden)]
#[path = "raft/membership.rs"]
pub mod raft_membership_core;
#[doc(hidden)]
#[path = "service/records.rs"]
pub mod registry;
#[doc(hidden)]
#[path = "prod/release/profile.rs"]
pub mod release_profile_core;
#[doc(hidden)]
#[path = "octet/remediation.rs"]
pub mod remediator;
#[doc(hidden)]
#[path = "protocol/sans/io.rs"]
pub mod sans_io_protocol_core;
pub mod schema_identity_core_pilot;
#[doc(hidden)]
#[path = "resources/mod.rs"]
pub mod supplies;
pub mod system_extension;
#[doc(hidden)]
#[path = "harness/mod.rs"]
pub mod testbed;
#[doc(hidden)]
#[path = "testing/traceability.rs"]
pub mod trace_core;
#[doc(hidden)]
#[path = "rewrites/mod.rs"]
pub mod transforms;
#[doc(hidden)]
#[path = "node/iroh.rs"]
pub mod transport;
#[doc(hidden)]
#[path = "secrets/mod.rs"]
pub mod vault;
#[path = "wasm/component/mod.rs"]
pub mod wasm_component;
#[path = "wasm/performance/mod.rs"]
pub mod wasm_performance;
#[doc(hidden)]
#[path = "service/supervision.rs"]
pub mod watchdog;
#[doc(hidden)]
#[path = "service/runtime.rs"]
pub mod worker_core;
#[doc(hidden)]
#[path = "job/dag.rs"]
pub mod workload;
pub mod world_benchmark;
pub mod world_branch_authority;
#[path = "worldcommit/mod.rs"]
pub mod world_commit;
pub mod world_distribution;
pub mod world_faults;
pub mod world_head;
pub mod world_merge;
pub mod world_operator;
pub mod world_promotion;
pub mod world_replay;
pub mod world_snapshot;
#[cfg(feature = "doltlite-oracle")]
pub mod world_state_oracle;

#[cfg(test)]
#[path = "test/support.rs"]
pub(crate) mod test_support;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/parts/lib/p000/body.rs"));
