use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use molten::artifacts;
use molten::authority;
use molten::catalog_mcp;
use molten::chunk_store;
use molten::coordination;
use molten::delivery_idempotency;
use molten::error::MoltenError;
use molten::error::Result;
use molten::eval_cache;
use molten::evidence::PASS_EVIDENCE_PURPOSE;
use molten::harness::failure_value;
use molten::harness::parse_failure;
use molten::harness::parse_repro_bundle;
use molten::job_dag;
use molten::ledger;
use molten::octet_gate;
use molten::operator_dogfood;
use molten::plugin_host;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::protocol_session;
use molten::provenance;
use molten::remote_dataspace;
use molten::retention;
use molten::schema_identity;
use molten::secrets;
use molten::typed_storage;
use molten::upgrades;

use super::*;
use crate::cli_chunk::Top;
use crate::cli_chunk::run;
use crate::cli_coordination::CoordinationCommand;
use crate::cli_coordination::run_coordination_command;
use crate::cli_delivery::DeliveryCommand;
use crate::cli_delivery::run_delivery_command;
use crate::cli_dogfood::DogfoodCommand;
use crate::cli_dogfood::run_dogfood_command;
use crate::cli_gate::GateCommand;
use crate::cli_gate::run_gate_command;
use crate::cli_job::JobCommand;
use crate::cli_job::run_job_command;
use crate::cli_ledger::ChainCommand;
use crate::cli_ledger::run_chain_command;
use crate::cli_plugin::PluginCommand;
use crate::cli_plugin::run_plugin_command;
use crate::cli_protocol::ProtocolCommand;
use crate::cli_protocol::run_protocol_command;
use crate::cli_provenance::ProvenanceCommand;
use crate::cli_provenance::run_provenance_command;
use crate::cli_raft::RaftCommand;
use crate::cli_raft::run_raft_command;
use crate::cli_receipts::ReceiptCommand;
use crate::cli_receipts::ReceiptsCommand;
use crate::cli_receipts::run_receipt_command;
use crate::cli_receipts::run_receipts_command;
use crate::cli_remote::RemoteCommand;
use crate::cli_remote::RemoteEnvelopeCommand;
use crate::cli_remote::run_remote_command;
use crate::cli_repro::ReproCommand;
use crate::cli_repro::run_repro_command;
use crate::cli_retention::RetentionCommand;
use crate::cli_retention::run_retention_command;
use crate::cli_rewrite::RewriteCommand;
use crate::cli_rewrite::run_rewrite_command;
use crate::cli_secrets::SecretsCommand;
use crate::cli_secrets::run_secrets_command;
use crate::cli_service::ServiceCommand;
use crate::cli_service::run_service_command;
use crate::cli_upgrade::UpgradeCommand;
use crate::cli_upgrade::run_upgrade_command;

fn cli_synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("remote-cli-ref", vec![string(label)]))
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}

include!("tests/core/basic.rs");
include!("tests/core/chunk.rs");
include!("tests/core/cache.rs");
include!("tests/core/storage.rs");
include!("tests/core/protocol.rs");
include!("tests/core/retention.rs");
include!("tests/ops/provenance.rs");
include!("tests/job/job.rs");
include!("tests/job/job-setup.rs");
include!("tests/job/job-sync.rs");
include!("tests/job/job-admission.rs");
include!("tests/job/job-worker.rs");
include!("tests/job/job-stale.rs");
include!("tests/job/job-run.rs");
include!("tests/ops/catalog.rs");
include!("tests/ops/misc.rs");
include!("tests/ops/upgrade.rs");
include!("tests/ops/receipts.rs");
include!("tests/ops/helpers.rs");
