use super::*;

type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type MoltenError = molten::error::MoltenError;
type Result<T> = molten::error::Result<T>;
type Top = crate::cli_chunk::Top;
type CoordinationCommand = crate::cli_coordination::CoordinationCommand;
type DeliveryCommand = crate::cli_delivery::DeliveryCommand;
type DogfoodCommand = crate::cli_dogfood::DogfoodCommand;
type GateCommand = crate::cli_gate::GateCommand;
type JobCommand = crate::cli_job::JobCommand;
type ChainCommand = crate::cli_ledger::ChainCommand;
type PluginCommand = crate::cli_plugin::PluginCommand;
type ProtocolCommand = crate::cli_protocol::ProtocolCommand;
type ProvenanceCommand = crate::cli_provenance::ProvenanceCommand;
type RaftCommand = crate::cli_raft::RaftCommand;
type ReceiptCommand = crate::cli_receipts::ReceiptCommand;
type ReceiptsCommand = crate::cli_receipts::ReceiptsCommand;
type RemoteCommand = crate::cli_remote::RemoteCommand;
type RemoteEnvelopeCommand = crate::cli_remote::RemoteEnvelopeCommand;
type ReproCommand = crate::cli_repro::ReproCommand;
type RetentionCommand = crate::cli_retention::RetentionCommand;
type RewriteCommand = crate::cli_rewrite::RewriteCommand;
type SecretsCommand = crate::cli_secrets::SecretsCommand;
type ServiceCommand = crate::cli_service::ServiceCommand;
type UpgradeCommand = crate::cli_upgrade::UpgradeCommand;

const PASS_EVIDENCE_PURPOSE: &str = molten::evidence::PASS_EVIDENCE_PURPOSE;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    pub(super) fn read_dir(path: impl AsRef<std::path::Path>) -> std::io::Result<std::fs::ReadDir> {
        std::fs::read_dir(path)
    }

    pub(super) fn read_to_string(path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

fn canonical_hash(value: &preserves::IOValue) -> Result<String> {
    molten::preserves_rail::canonical_hash(value)
}

fn parse_text(source: &str) -> Result<preserves::IOValue> {
    molten::preserves_rail::parse_text(source)
}

fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    molten::preserves_rail::record(label, fields)
}

fn string(value: impl AsRef<str>) -> preserves::IOValue {
    molten::preserves_rail::string(value)
}

fn to_text(value: &preserves::IOValue) -> Result<String> {
    molten::preserves_rail::to_text(value)
}

fn failure_value(phase: &str, error: &MoltenError, diagnostics: Vec<preserves::IOValue>) -> preserves::IOValue {
    molten::harness::failure_value(phase, error, diagnostics)
}

fn parse_failure(value: &preserves::IOValue) -> Result<molten::harness::HarnessFailure> {
    molten::harness::parse_failure(value)
}

fn parse_repro_bundle(value: &preserves::IOValue) -> Result<molten::harness::HarnessReproBundle> {
    molten::harness::parse_repro_bundle(value)
}

fn run(command: Top) -> Result<()> {
    crate::cli_chunk::run(command)
}

fn run_coordination_command(command: CoordinationCommand) -> Result<()> {
    crate::cli_coordination::run_coordination_command(command)
}

fn run_delivery_command(command: DeliveryCommand) -> Result<()> {
    crate::cli_delivery::run_delivery_command(command)
}

fn run_dogfood_command(command: DogfoodCommand) -> Result<()> {
    crate::cli_dogfood::run_dogfood_command(command)
}

fn run_gate_command(command: GateCommand) -> Result<()> {
    crate::cli_gate::run_gate_command(command)
}

fn run_job_command(command: JobCommand) -> Result<()> {
    crate::cli_job::run_job_command(command)
}

fn run_chain_command(command: ChainCommand) -> Result<()> {
    crate::cli_ledger::run_chain_command(command)
}

fn run_plugin_command(command: PluginCommand) -> Result<()> {
    crate::cli_plugin::run_plugin_command(command)
}

fn run_protocol_command(command: ProtocolCommand) -> Result<()> {
    crate::cli_protocol::run_protocol_command(command)
}

fn run_provenance_command(command: ProvenanceCommand) -> Result<()> {
    crate::cli_provenance::run_provenance_command(command)
}

fn run_raft_command(command: RaftCommand) -> Result<()> {
    crate::cli_raft::run_raft_command(command)
}

fn run_receipt_command(command: ReceiptCommand) -> Result<()> {
    crate::cli_receipts::run_receipt_command(command)
}

fn run_receipts_command(command: ReceiptsCommand) -> Result<()> {
    crate::cli_receipts::run_receipts_command(command)
}

fn run_remote_command(command: RemoteCommand) -> Result<()> {
    crate::cli_remote::run_remote_command(command)
}

fn run_repro_command(command: ReproCommand) -> Result<()> {
    crate::cli_repro::run_repro_command(command)
}

fn run_retention_command(command: RetentionCommand) -> Result<()> {
    crate::cli_retention::run_retention_command(command)
}

fn run_rewrite_command(command: RewriteCommand) -> Result<()> {
    crate::cli_rewrite::run_rewrite_command(command)
}

fn run_secrets_command(command: SecretsCommand) -> Result<()> {
    crate::cli_secrets::run_secrets_command(command)
}

fn run_service_command(command: ServiceCommand) -> Result<()> {
    crate::cli_service::run_service_command(command)
}

fn run_upgrade_command(command: UpgradeCommand) -> Result<()> {
    crate::cli_upgrade::run_upgrade_command(command)
}

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
