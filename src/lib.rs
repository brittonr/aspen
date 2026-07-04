mod bounded;

macro_rules! compat_module {
    ($name:ident, $target:ident) => {
        pub mod $name {
            pub use super::$target::*;
        }
    };
}

#[doc(hidden)]
#[path = "artifacts/mod.rs"]
pub mod objects;
compat_module!(artifacts, objects);
#[doc(hidden)]
#[path = "authority/mod.rs"]
pub mod delegation;
compat_module!(authority, delegation);
#[doc(hidden)]
#[path = "catalog/mod.rs"]
pub mod inventory;
compat_module!(catalog, inventory);
#[doc(hidden)]
#[path = "catalog/mcp.rs"]
pub mod inventory_api;
compat_module!(catalog_mcp, inventory_api);
#[doc(hidden)]
#[path = "chunk/store.rs"]
pub mod blocks;
compat_module!(chunk_store, blocks);
#[doc(hidden)]
#[path = "capability/mod.rs"]
pub mod capabilities_core;
compat_module!(capability_tokens, capabilities_core);
#[doc(hidden)]
#[path = "coordination/mod.rs"]
pub mod orchestration;
compat_module!(coordination, orchestration);
#[doc(hidden)]
#[path = "delivery/idempotency.rs"]
pub mod dedupe;
compat_module!(delivery_idempotency, dedupe);
#[doc(hidden)]
#[path = "deterministic/replay.rs"]
pub mod playback;
compat_module!(deterministic_replay, playback);
#[doc(hidden)]
#[path = "effects/mod.rs"]
pub mod actions;
compat_module!(effects, actions);
#[doc(hidden)]
#[path = "error/mod.rs"]
pub mod failures;
compat_module!(error, failures);
#[doc(hidden)]
#[path = "eval/cache.rs"]
pub mod memo;
compat_module!(eval_cache, memo);
#[doc(hidden)]
#[path = "evidence/mod.rs"]
pub mod proofs;
compat_module!(evidence, proofs);
#[doc(hidden)]
#[path = "evidence/chain.rs"]
pub mod lineage;
compat_module!(evidence_chain, lineage);
#[doc(hidden)]
#[path = "federation/mod.rs"]
pub mod mesh;
compat_module!(federation, mesh);
#[doc(hidden)]
#[path = "harness/mod.rs"]
pub mod testbed;
compat_module!(harness, testbed);
#[doc(hidden)]
#[path = "iroh/exchange.rs"]
pub mod netlink;
compat_module!(iroh_exchange, netlink);
#[doc(hidden)]
#[path = "job/dag.rs"]
pub mod workload;
compat_module!(job_dag, workload);
#[doc(hidden)]
#[path = "ledger/mod.rs"]
pub mod journal;
compat_module!(ledger, journal);
#[doc(hidden)]
#[path = "lifecycle/mod.rs"]
pub mod phases;
compat_module!(lifecycle, phases);
#[doc(hidden)]
#[path = "nixos/vm.rs"]
pub mod machine;
compat_module!(nixos_vm, machine);
#[doc(hidden)]
#[path = "node/daemon.rs"]
pub mod daemon_core;
compat_module!(node_daemon, daemon_core);
#[doc(hidden)]
#[path = "node/identity.rs"]
pub mod credential;
compat_module!(node_identity, credential);
#[doc(hidden)]
#[path = "node/iroh.rs"]
pub mod transport;
compat_module!(node_iroh, transport);
#[doc(hidden)]
#[path = "node/runtime.rs"]
pub mod kernel;
compat_module!(node_runtime, kernel);
#[doc(hidden)]
#[path = "octet/gate.rs"]
pub mod quality;
compat_module!(octet_gate, quality);
#[doc(hidden)]
#[path = "octet/remediation.rs"]
pub mod remediator;
compat_module!(octet_remediation, remediator);
#[doc(hidden)]
#[path = "operator/dogfood.rs"]
pub mod pilot;
compat_module!(operator_dogfood, pilot);
#[doc(hidden)]
#[path = "operator/gateway.rs"]
pub mod edgeway;
compat_module!(operator_gateway, edgeway);
#[doc(hidden)]
#[path = "peer/bootstrap.rs"]
pub mod peering;
compat_module!(peer_bootstrap, peering);
#[doc(hidden)]
#[path = "plugin/host.rs"]
pub mod extension;
compat_module!(plugin_host, extension);
#[doc(hidden)]
#[path = "preserves/rail.rs"]
pub mod codec;
compat_module!(preserves_rail, codec);
#[doc(hidden)]
#[path = "prod/readiness.rs"]
pub mod launch;
compat_module!(prod_readiness, launch);
#[doc(hidden)]
#[path = "prod/pilot.rs"]
pub mod pilot_readiness;
compat_module!(external_live_pilot, pilot_readiness);
#[doc(hidden)]
#[path = "prod/soak.rs"]
pub mod burnin;
compat_module!(prod_soak, burnin);
#[doc(hidden)]
#[path = "protocol/session.rs"]
pub mod conversation;
compat_module!(protocol_session, conversation);
#[doc(hidden)]
#[path = "provenance/mod.rs"]
pub mod lineage_meta;
compat_module!(provenance, lineage_meta);
#[doc(hidden)]
#[path = "propagation/mod.rs"]
pub mod propagation_core;
compat_module!(eventual_surface, propagation_core);
#[doc(hidden)]
#[path = "raft/control/plane.rs"]
pub mod quorum;
compat_module!(raft_control_plane, quorum);
#[doc(hidden)]
#[path = "raft/membership.rs"]
pub mod raft_membership_core;
compat_module!(raft_membership, raft_membership_core);
#[doc(hidden)]
#[path = "remote/dataspace.rs"]
pub mod meshspace;
compat_module!(remote_dataspace, meshspace);
#[doc(hidden)]
#[path = "resources/mod.rs"]
pub mod supplies;
compat_module!(resources, supplies);
#[doc(hidden)]
#[path = "retention/mod.rs"]
pub mod custody;
compat_module!(retention, custody);
#[doc(hidden)]
#[path = "rewrites/mod.rs"]
pub mod transforms;
compat_module!(rewrites, transforms);
#[doc(hidden)]
#[path = "runtime/mod.rs"]
pub mod engine;
compat_module!(runtime, engine);
#[doc(hidden)]
#[path = "schema/identity.rs"]
pub mod descriptor;
compat_module!(schema_identity, descriptor);
#[doc(hidden)]
#[path = "secrets/mod.rs"]
pub mod vault;
compat_module!(secrets, vault);
#[doc(hidden)]
#[path = "service/records.rs"]
pub mod registry;
compat_module!(service_records, registry);
#[doc(hidden)]
#[path = "service/runtime.rs"]
pub mod worker_core;
compat_module!(service_runtime, worker_core);
#[doc(hidden)]
#[path = "service/supervision.rs"]
pub mod watchdog;
compat_module!(service_supervision, watchdog);
#[doc(hidden)]
#[path = "testing/drift.rs"]
pub mod drift_core;
compat_module!(deterministic_drift, drift_core);
#[doc(hidden)]
#[path = "testing/distributed.rs"]
pub mod distributed_core;
compat_module!(distributed_testing, distributed_core);
#[doc(hidden)]
#[path = "testing/multinode.rs"]
pub mod multinode_core;
compat_module!(multinode_testing, multinode_core);
#[doc(hidden)]
#[path = "testing/proof_trace.rs"]
pub mod proof_trace_core;
compat_module!(state_machine_proof, proof_trace_core);
#[doc(hidden)]
#[path = "testing/traceability.rs"]
pub mod trace_core;
compat_module!(requirement_traceability, trace_core);
#[doc(hidden)]
#[path = "transcripts/mod.rs"]
pub mod narratives;
compat_module!(transcripts, narratives);
#[doc(hidden)]
#[path = "typed/storage.rs"]
pub mod cells;
compat_module!(typed_storage, cells);
#[doc(hidden)]
#[path = "upgrades/mod.rs"]
pub mod migrations;
compat_module!(upgrades, migrations);

#[cfg(test)]
#[path = "test/support.rs"]
pub(crate) mod test_support;

pub use failures::MoltenError;
pub use failures::Result;

pub fn greeting() -> &'static str {
    "hello from molten"
}

#[cfg(test)]
mod tests {
    #[test]
    fn greeting_mentions_project_name() {
        assert!(super::greeting().contains("molten"));
    }
}
