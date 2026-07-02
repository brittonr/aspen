#[path = "cli/core/artifact.rs"]
mod cli_artifact;
#[path = "cli/core/cache.rs"]
mod cli_cache;
#[path = "cli/core/catalog.rs"]
mod cli_catalog;
#[path = "cli/core/chunk.rs"]
mod cli_chunk;
#[path = "cli/workflow/coordination.rs"]
mod cli_coordination;
#[path = "cli/workflow/delivery.rs"]
mod cli_delivery;
#[path = "cli/ops/dogfood/mod.rs"]
mod cli_dogfood;
#[path = "cli/evidence/gate.rs"]
mod cli_gate;
#[path = "cli/ops/gateway.rs"]
mod cli_gateway;
#[path = "cli/test/harness.rs"]
mod cli_harness;
#[path = "cli/workflow/job.rs"]
mod cli_job;
#[path = "cli/ops/ledger.rs"]
mod cli_ledger;
#[path = "cli/ops/nixosvm.rs"]
mod cli_nixos_vm;
#[path = "cli/ops/node.rs"]
mod cli_node;
#[path = "cli/ops/octet.rs"]
mod cli_octet;
#[path = "cli/ops/plugin.rs"]
mod cli_plugin;
#[path = "cli/ops/prodsoak.rs"]
mod cli_prod_soak;
#[path = "cli/workflow/protocol.rs"]
mod cli_protocol;
#[path = "cli/workflow/provenance.rs"]
mod cli_provenance;
#[path = "cli/runtime/raft.rs"]
mod cli_raft;
#[path = "cli/evidence/receipts.rs"]
mod cli_receipts;
#[path = "cli/workflow/remote.rs"]
mod cli_remote;
#[path = "cli/test/replayfixture.rs"]
mod cli_replay_fixture;
#[path = "cli/evidence/report.rs"]
mod cli_report;
#[path = "cli/runtime/repro.rs"]
mod cli_repro;
#[path = "cli/workflow/retention.rs"]
mod cli_retention;
#[path = "cli/runtime/rewrite.rs"]
mod cli_rewrite;
#[path = "main/root.rs"]
mod cli_root;
#[path = "cli/core/schema.rs"]
mod cli_schema;
#[path = "cli/runtime/secrets.rs"]
mod cli_secrets;
#[path = "cli/runtime/service.rs"]
mod cli_service;
#[path = "cli/core/storage.rs"]
mod cli_storage;
#[path = "cli/core/transcript.rs"]
mod cli_transcript;
#[path = "cli/runtime/upgrade.rs"]
mod cli_upgrade;
#[path = "cli/runtime/vat.rs"]
mod cli_vat;

pub(crate) type RetentionEvidenceArgs = cli_root::RetentionEvidenceArgs;
#[cfg(test)]
pub(crate) type RuntimeCommand = cli_root::RuntimeCommand;
#[cfg(test)]
pub(crate) type TestCommand = cli_root::TestCommand;

#[cfg(test)]
pub(crate) fn run_runtime_command(command: RuntimeCommand) -> molten::error::Result<()> {
    cli_root::run_runtime_command(command)
}

#[cfg(test)]
pub(crate) fn run_test_command(command: TestCommand) -> molten::error::Result<()> {
    cli_root::run_test_command(command)
}

fn main() {
    if let Err(error) = cli_root::run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
