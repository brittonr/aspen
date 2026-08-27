#[derive(Debug, clap::Parser)]
#[command(name = "molten", version, about = "Molten runtime prototype")]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Option<Top>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(super) enum Top {
    Cluster {
        #[command(subcommand)]
        command: crate::cli_cluster::ClusterCommand,
    },
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
    Peer {
        #[command(subcommand)]
        command: crate::cli_peer::Command,
    },
    Runtime {
        #[command(subcommand)]
        command: Runtime,
    },
    FabricTime {
        #[command(subcommand)]
        command: crate::cli_fabric_time::FabricTimeCommand,
    },
    FabricSimulation {
        #[command(subcommand)]
        command: crate::cli_fabric_simulation::FabricSimulationCommand,
    },
    SystemExtension {
        #[command(subcommand)]
        command: crate::cli_system_extension::SystemExtensionCommand,
    },
    WorldCommit {
        #[command(subcommand)]
        command: crate::cli_world_commit::WorldCommitCommand,
    },
    WorldHead {
        #[command(subcommand)]
        command: crate::cli_world_head::WorldHeadCommand,
    },
    WorldMerge {
        #[command(subcommand)]
        command: crate::cli_world_merge::WorldMergeCommand,
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Cli;
    use super::Top;

    const COMMIT_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn world_commit_operator_commands_parse_explicit_state_and_identity() {
        let cli = Cli::try_parse_from([
            "molten",
            "world-commit",
            "plan-restore",
            "--state-root",
            "state",
            COMMIT_REF,
            "--out",
            "restore.preserves",
        ])
        .expect("world commit command");

        assert!(matches!(cli.command, Some(Top::WorldCommit { .. })));
    }

    #[test]
    fn world_commit_operator_commands_reject_missing_state_root() {
        let result = Cli::try_parse_from(["molten", "world-commit", "inspect", COMMIT_REF]);

        assert!(result.is_err());
    }

    #[test]
    fn world_head_plan_parses_every_compare_and_swap_input() {
        let cli = Cli::try_parse_from([
            "molten",
            "world-head",
            "plan",
            "--branch",
            "main",
            "--expected-head",
            COMMIT_REF,
            "--successor-head",
            COMMIT_REF,
            "--expected-generation",
            "1",
            "--successor-generation",
            "2",
            "--purpose",
            "advance",
            "--policy-ref",
            COMMIT_REF,
            "--out",
            "claim.preserves",
        ])
        .expect("world-head plan command");

        assert!(matches!(cli.command, Some(Top::WorldHead { .. })));
    }

    #[test]
    fn world_head_mutation_commands_reject_missing_capability_root() {
        let result = Cli::try_parse_from([
            "molten",
            "world-head",
            "advance",
            "--claim",
            "claim.preserves",
            "--signature",
            "signature.json",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn world_merge_plan_parses_explicit_base_sources_and_policy() {
        let cli = Cli::try_parse_from([
            "molten",
            "world-merge",
            "merge-plan",
            "--state-root",
            "state",
            "--base",
            COMMIT_REF,
            "--left",
            COMMIT_REF,
            "--right",
            COMMIT_REF,
            "--profile-ref",
            COMMIT_REF,
            "--policy-ref",
            COMMIT_REF,
            "--out",
            "merge.preserves",
        ])
        .expect("world-merge plan command");

        assert!(matches!(cli.command, Some(Top::WorldMerge { .. })));
    }

    #[test]
    fn world_merge_publish_rejects_missing_capability_root() {
        let result = Cli::try_parse_from(["molten", "world-merge", "merge-publish", "--plan", "merge.preserves"]);

        assert!(result.is_err());
    }
}
