mod bounded;

pub mod artifacts;
pub mod authority;
pub mod catalog;
pub mod catalog_mcp;
pub mod chunk_store;
pub mod coordination;
pub mod delivery_idempotency;
pub mod effects;
pub mod error;
pub mod eval_cache;
pub mod evidence;
pub mod evidence_chain;
pub mod federation;
pub mod harness;
pub mod iroh_exchange;
pub mod job_dag;
pub mod ledger;
pub mod node_daemon;
pub mod node_identity;
pub mod node_runtime;
pub mod octet_gate;
pub mod octet_remediation;
pub mod operator_dogfood;
pub mod peer_bootstrap;
pub mod plugin_host;
pub mod preserves_rail;
pub mod protocol_session;
pub mod raft_control_plane;
pub mod remote_dataspace;
pub mod resources;
pub mod rewrites;
pub mod runtime;
pub mod schema_identity;
pub mod secrets;
pub mod service_records;
pub mod service_runtime;
pub mod service_supervision;
pub mod transcripts;
pub mod typed_storage;
pub mod upgrades;

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
