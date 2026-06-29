#[path = "root/command.rs"]
mod command;

#[cfg(test)]
pub(crate) type RuntimeCommand = command::Runtime;
#[cfg(test)]
pub(crate) type TestCommand = command::Test;

#[derive(Debug, Clone, clap::Args)]
pub(crate) struct RetentionEvidenceArgs {
    #[arg(long = "retention-requester")]
    pub(crate) requester_ref: Option<String>,
    #[arg(long = "retention-policy-ref")]
    pub(crate) policy_refs: Vec<String>,
    #[arg(long = "retention-authority-ref")]
    pub(crate) authority_refs: Vec<String>,
    #[arg(long = "retention-evidence-ref")]
    pub(crate) evidence_refs: Vec<String>,
    #[arg(long = "retention-retained-ref")]
    pub(crate) retained_refs: Vec<String>,
    #[arg(long = "retention-remote-peer-ref")]
    pub(crate) remote_peer_refs: Vec<String>,
    #[arg(long = "retention-remote-ref")]
    pub(crate) remote_refs: Vec<String>,
    #[arg(long = "retention-reference-index-ref")]
    pub(crate) reference_index_refs: Vec<String>,
    #[arg(long = "retention-remote-gc-ref")]
    pub(crate) remote_gc_refs: Vec<String>,
    #[arg(long = "retention-remote-clearance-ref")]
    pub(crate) remote_clearance_refs: Vec<String>,
    #[arg(long = "retention-reference-index-complete")]
    pub(crate) is_reference_index_complete: bool,
}

impl RetentionEvidenceArgs {
    pub(crate) fn into_retention_evidence(self) -> molten::retention::DestructiveRetentionEvidence {
        molten::retention::DestructiveRetentionEvidence {
            requester_ref: self.requester_ref,
            policy_refs: self.policy_refs,
            authority_refs: self.authority_refs,
            evidence_refs: self.evidence_refs,
            retained_refs: self.retained_refs,
            remote_peer_refs: self.remote_peer_refs,
            remote_refs: self.remote_refs,
            reference_index_refs: self.reference_index_refs,
            remote_gc_refs: self.remote_gc_refs,
            remote_clearance_refs: self.remote_clearance_refs,
            is_reference_index_complete: self.is_reference_index_complete,
        }
    }
}
pub(crate) fn run() -> molten::error::Result<()> {
    let cli = <command::Cli as clap::Parser>::parse();
    match cli.command {
        None => {
            println!("{}", molten::greeting());
            Ok(())
        }
        Some(command::Top::Test { command }) => run_test_command(command),
        Some(command::Top::Dogfood { command }) => crate::cli_dogfood::run_dogfood_command(command),
        Some(command::Top::Receipts { command }) => crate::cli_receipts::run_receipts_command(command),
        Some(command::Top::Node { command }) => crate::cli_node::run(command),
        Some(command::Top::Runtime { command }) => run_runtime_command(command),
    }
}

pub(crate) fn run_runtime_command(command: command::Runtime) -> molten::error::Result<()> {
    match command {
        command::Runtime::Config { config } => {
            let source = std::fs::read_to_string(&config).map_err(molten::error::MoltenError::from)?;
            let startup = molten::runtime::RuntimeStartupConfig::from_nickel_export_json(&source)?;
            println!(
                "runtime config ok source=nickel actors={} subscriptions={}",
                startup.actors.len(),
                startup.subscriptions.len()
            );
            Ok(())
        }
    }
}

pub(crate) fn run_test_command(command: command::Test) -> molten::error::Result<()> {
    match command {
        command::Test::Run { suite, report_out } => crate::cli_harness::run_harness_suite_command(suite, report_out),
        command::Test::Replay { report, failure_out } => {
            crate::cli_harness::run_harness_replay_command(report, failure_out)
        }
        command::Test::ReplayFixture { command } => crate::cli_replay_fixture::run_replay_fixture_command(command),
        command::Test::Report { command } => crate::cli_report::run_report_command(command),
        command::Test::Gate { command } => crate::cli_gate::run_gate_command(command),
        command::Test::Gateway { command } => crate::cli_gateway::run_command(command),
        command::Test::Receipt { command } => crate::cli_receipts::run_receipt_command(command),
        command::Test::Ledger { command } => crate::cli_ledger::run_ledger_command(command),
        command::Test::Chain { command } => crate::cli_ledger::run_chain_command(command),
        command::Test::Chunk { command } => crate::cli_chunk::run(command),
        command::Test::Storage { command } => crate::cli_storage::run_storage_command(command),
        command::Test::Artifact { command } => crate::cli_artifact::run_artifact_command(command),
        command::Test::Schema { command } => crate::cli_schema::run_schema_command(command),
        command::Test::Cache { command } => crate::cli_cache::run_cache_command(command),
        command::Test::Upgrade { command } => crate::cli_upgrade::run_upgrade_command(command),
        command::Test::Transcript { command } => crate::cli_transcript::run_transcript_command(command),
        command::Test::Rewrite { command } => crate::cli_rewrite::run_rewrite_command(command),
        command::Test::Catalog { command } => crate::cli_catalog::run_catalog_command(command),
        command::Test::Job { command } => crate::cli_job::run_job_command(command),
        command::Test::Remote { command } => crate::cli_remote::run_remote_command(command),
        command::Test::Delivery { command } => crate::cli_delivery::run_delivery_command(command),
        command::Test::Retention { command } => crate::cli_retention::run_retention_command(command),
        command::Test::Provenance { command } => crate::cli_provenance::run_provenance_command(command),
        command::Test::Protocol { command } => crate::cli_protocol::run_protocol_command(command),
        command::Test::Raft { command } => crate::cli_raft::run_raft_command(command),
        command::Test::Plugin { command } => crate::cli_plugin::run_plugin_command(command),
        command::Test::Coordination { command } => crate::cli_coordination::run_coordination_command(command),
        command::Test::Secrets { command } => crate::cli_secrets::run_secrets_command(command),
        command::Test::Service { command } => crate::cli_service::run_service_command(command),
        command::Test::Vat { command } => crate::cli_vat::run_vat_command(command),
        command::Test::NixosVm { command } => crate::cli_nixos_vm::run_nixos_vm_command(command),
        command::Test::ProdSoak { command } => crate::cli_prod_soak::run_prod_soak_command(command),
        command::Test::Octet { command } => crate::cli_octet::run_octet_command(command),
        command::Test::Node { command } => crate::cli_node::run(command),
        command::Test::Repro { command } => crate::cli_repro::run_repro_command(command),
    }
}
