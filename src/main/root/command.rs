#[derive(Debug, clap::Parser)]
#[command(name = "molten", version, about = "Molten runtime prototype")]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Option<Top>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(super) enum Top {
    Test {
        #[command(subcommand)]
        command: Test,
    },
    Dogfood {
        #[command(subcommand)]
        command: crate::cli_dogfood::DogfoodCommand,
    },
    Receipts {
        #[command(subcommand)]
        command: crate::cli_receipts::ReceiptsCommand,
    },
    Node {
        #[command(subcommand)]
        command: crate::cli_node::Command,
    },
    Runtime {
        #[command(subcommand)]
        command: Runtime,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Runtime {
    Config {
        #[arg(long)]
        config: std::path::PathBuf,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Test {
    Run {
        suite: std::path::PathBuf,
        #[arg(long)]
        report_out: Option<std::path::PathBuf>,
    },
    Replay {
        report: std::path::PathBuf,
        #[arg(long)]
        failure_out: Option<std::path::PathBuf>,
    },
    ReplayFixture {
        #[command(subcommand)]
        command: crate::cli_replay_fixture::ReplayFixtureCommand,
    },
    Report {
        #[command(subcommand)]
        command: crate::cli_report::ReportCommand,
    },
    Gate {
        #[command(subcommand)]
        command: crate::cli_gate::GateCommand,
    },
    Drift {
        #[command(subcommand)]
        command: crate::cli_drift::DriftCommand,
    },
    Gateway {
        #[command(subcommand)]
        command: crate::cli_gateway::Command,
    },
    Receipt {
        #[command(subcommand)]
        command: crate::cli_receipts::ReceiptCommand,
    },
    Ledger {
        #[command(subcommand)]
        command: crate::cli_ledger::LedgerCommand,
    },
    Chain {
        #[command(subcommand)]
        command: crate::cli_ledger::ChainCommand,
    },
    Chunk {
        #[command(subcommand)]
        command: crate::cli_chunk::Top,
    },
    Storage {
        #[command(subcommand)]
        command: crate::cli_storage::Command,
    },
    Artifact {
        #[command(subcommand)]
        command: crate::cli_artifact::Command,
    },
    Schema {
        #[command(subcommand)]
        command: crate::cli_schema::Command,
    },
    Cache {
        #[command(subcommand)]
        command: crate::cli_cache::Command,
    },
    Upgrade {
        #[command(subcommand)]
        command: crate::cli_upgrade::UpgradeCommand,
    },
    Transcript {
        #[command(subcommand)]
        command: crate::cli_transcript::Command,
    },
    Rewrite {
        #[command(subcommand)]
        command: crate::cli_rewrite::RewriteCommand,
    },
    Catalog {
        #[command(subcommand)]
        command: crate::cli_catalog::Command,
    },
    Job {
        #[command(subcommand)]
        command: crate::cli_job::JobCommand,
    },
    Remote {
        #[command(subcommand)]
        command: crate::cli_remote::RemoteCommand,
    },
    Delivery {
        #[command(subcommand)]
        command: crate::cli_delivery::DeliveryCommand,
    },
    Retention {
        #[command(subcommand)]
        command: crate::cli_retention::RetentionCommand,
    },
    Provenance {
        #[command(subcommand)]
        command: crate::cli_provenance::ProvenanceCommand,
    },
    Protocol {
        #[command(subcommand)]
        command: crate::cli_protocol::ProtocolCommand,
    },
    Raft {
        #[command(subcommand)]
        command: crate::cli_raft::RaftCommand,
    },
    Plugin {
        #[command(subcommand)]
        command: crate::cli_plugin::PluginCommand,
    },
    Coordination {
        #[command(subcommand)]
        command: crate::cli_coordination::CoordinationCommand,
    },
    Secrets {
        #[command(subcommand)]
        command: crate::cli_secrets::SecretsCommand,
    },
    Service {
        #[command(subcommand)]
        command: crate::cli_service::ServiceCommand,
    },
    Vat {
        #[command(subcommand)]
        command: crate::cli_vat::VatCommand,
    },
    NixosVm {
        #[command(subcommand)]
        command: crate::cli_nixos_vm::NixosVmCommand,
    },
    Traceability {
        #[command(subcommand)]
        command: crate::cli_traceability::TraceabilityCommand,
    },
    ProdSoak {
        #[command(subcommand)]
        command: crate::cli_prod_soak::ProdSoakCommand,
    },
    Octet {
        #[command(subcommand)]
        command: crate::cli_octet::OctetCommand,
    },
    Node {
        #[command(subcommand)]
        command: crate::cli_node::Command,
    },
    Repro {
        #[command(subcommand)]
        command: crate::cli_repro::ReproCommand,
    },
}
