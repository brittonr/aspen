#![feature(register_tool)]
#![register_tool(tigerstyle)]

// Node state and errors are owned by molten-node-host. The concrete execution
// adapters below share their implementations with the root package by source
// path; none introduces a dependency on the root molten crate.
#[path = "../../../src/bounded/core.rs"]
pub(crate) mod bounded;
#[path = "../../../src/error/mod.rs"]
pub mod error;
#[path = "../../../src/local_store/mod.rs"]
pub mod local_store;
#[path = "../../../src/preserves/rail.rs"]
pub mod preserves_rail;
#[path = "chunk_storage.rs"]
pub mod chunk_store;
#[path = "../../../src/content_store_adapter/mod.rs"]
pub mod content_store_adapter;
#[path = "../../../src/fabric_crypto_identity/mod.rs"]
pub mod fabric_crypto_identity;
mod transport_key_record;
#[path = "../../../src/delivery/idempotency.rs"]
pub mod delivery_idempotency;
#[path = "ledger_storage.rs"]
pub mod ledger;
#[path = "artifact_registry.rs"]
pub mod artifacts;
#[path = "../../../src/provenance/mod.rs"]
pub mod provenance;
#[path = "../../../src/evidence/mod.rs"]
pub mod evidence;
#[path = "evidence_chain_store.rs"]
pub mod evidence_chain;
#[path = "protocol_session_store.rs"]
pub mod protocol_session;
#[path = "../../../src/remote/dataspace.rs"]
pub mod remote_dataspace;
#[path = "raft_control_plane_storage.rs"]
pub mod raft_control_plane;
#[path = "deterministic_replay_storage.rs"]
pub mod deterministic_replay;
#[path = "../../../src/runtime/mod.rs"]
pub mod runtime;
#[path = "../../../src/authority/mod.rs"]
pub mod authority;
#[path = "eval_cache_storage.rs"]
pub mod eval_cache;
#[path = "../../../src/resources/mod.rs"]
pub mod resources;
#[path = "job_dag_execution.rs"]
pub mod job_dag;
#[path = "effects_handlers.rs"]
pub mod effects;
#[path = "schema_identity.rs"]
pub mod schema_identity;
#[path = "typed_storage.rs"]
pub mod typed_storage;
#[path = "source_gate.rs"]
pub mod octet_gate;

#[path = "node/state.rs"]
pub mod node_state;
#[path = "node/identity.rs"]
pub mod node_identity;
#[path = "node/iroh.rs"]
pub mod node_iroh;
#[path = "node/runtime.rs"]
pub mod node_runtime;
#[path = "node/profile_config.rs"]
pub mod node_profile_config;
#[path = "node/service_fsm.rs"]
pub mod node_service_fsm;
#[path = "node/startup_evidence.rs"]
pub mod node_startup_evidence;
#[path = "node/daemon.rs"]
pub mod node_daemon;
#[path = "node/content.rs"]
pub mod node_content;

pub use error::{Failure, Result};

#[cfg(test)]
#[path = "../../../src/test/support.rs"]
pub(crate) mod test_support;
