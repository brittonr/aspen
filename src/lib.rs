mod bounded;

pub mod artifacts;
pub mod authority;
pub mod catalog;
#[path = "catalog/mcp.rs"]
pub mod catalog_mcp;
#[path = "chunk/store.rs"]
pub mod chunk_store;
pub mod coordination;
#[path = "delivery/idempotency.rs"]
pub mod delivery_idempotency;
#[path = "deterministic/replay.rs"]
pub mod deterministic_replay;
pub mod effects;
pub mod error;
#[path = "eval/cache.rs"]
pub mod eval_cache;
pub mod evidence;
#[path = "evidence/chain.rs"]
pub mod evidence_chain;
pub mod federation;
pub mod harness;
#[path = "iroh/exchange.rs"]
pub mod iroh_exchange;
#[path = "job/dag.rs"]
pub mod job_dag;
pub mod ledger;
pub mod lifecycle;
#[path = "nixos/vm.rs"]
pub mod nixos_vm;
#[path = "node/daemon.rs"]
pub mod node_daemon;
#[path = "node/identity.rs"]
pub mod node_identity;
#[path = "node/runtime.rs"]
pub mod node_runtime;
#[path = "octet/gate.rs"]
pub mod octet_gate;
#[path = "octet/remediation.rs"]
pub mod octet_remediation;
#[path = "operator/dogfood.rs"]
pub mod operator_dogfood;
#[path = "peer/bootstrap.rs"]
pub mod peer_bootstrap;
#[path = "plugin/host.rs"]
pub mod plugin_host;
#[path = "preserves/rail.rs"]
pub mod preserves_rail;
#[path = "prod/readiness.rs"]
pub mod prod_readiness;
#[path = "prod/soak.rs"]
pub mod prod_soak;
#[path = "protocol/session.rs"]
pub mod protocol_session;
pub mod provenance;
#[path = "raft/control/plane.rs"]
pub mod raft_control_plane;
#[path = "remote/dataspace.rs"]
pub mod remote_dataspace;
pub mod resources;
pub mod retention;
pub mod rewrites;
pub mod runtime;
#[path = "schema/identity.rs"]
pub mod schema_identity;
pub mod secrets;
#[path = "service/records.rs"]
pub mod service_records;
#[path = "service/runtime.rs"]
pub mod service_runtime;
#[path = "service/supervision.rs"]
pub mod service_supervision;
pub mod transcripts;
#[path = "typed/storage.rs"]
pub mod typed_storage;
pub mod upgrades;

#[cfg(test)]
#[path = "test/support.rs"]
pub(crate) mod test_support;

pub use error::MoltenError;
pub use error::Result;

pub fn greeting() -> &'static str {
    "hello from molten"
}

#[cfg(test)]
mod tests {
    use super::greeting;

    #[test]
    fn greeting_mentions_project_name() {
        assert!(greeting().contains("molten"));
    }
}
