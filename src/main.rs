#![feature(register_tool)]
#![register_tool(tigerstyle)]

#[cfg(test)]
#[path = "test/support.rs"]
mod test_support;

#[path = "cli/core/artifact.rs"]
mod object_port;
mod cli_artifact {
    pub(crate) use super::object_port::*;
}
#[path = "cli/core/cache.rs"]
mod memo_port;
mod cli_cache {
    pub(crate) use super::memo_port::*;
}
#[path = "cli/core/catalog.rs"]
mod inventory_port;
mod cli_catalog {
    pub(crate) use super::inventory_port::*;
}
#[path = "cli/core/chunk.rs"]
mod block_port;
mod cli_chunk {
    pub(crate) use super::block_port::*;
}
#[path = "cli/ops/cluster.rs"]
mod cluster_port;
mod cli_cluster {
    pub(crate) use super::cluster_port::*;
}
#[path = "cli/workflow/coordination.rs"]
mod orchestrate_port;
mod cli_coordination {
    pub(crate) use super::orchestrate_port::*;
}
#[path = "cli/workflow/delivery.rs"]
mod packet_port;
mod cli_delivery {
    pub(crate) use super::packet_port::*;
}
#[path = "cli/ops/dogfood/mod.rs"]
mod pilot_port;
mod cli_dogfood {
    pub(crate) use super::pilot_port::*;
}
#[path = "cli/ops/drift.rs"]
mod drift_port;
mod cli_drift {
    pub(crate) use super::drift_port::*;
}
#[path = "cli/evidence/gate.rs"]
mod barrier_port;
mod cli_gate {
    pub(crate) use super::barrier_port::*;
}
#[path = "cli/ops/gateway.rs"]
mod edge_port;
mod cli_gateway {
    pub(crate) use super::edge_port::*;
}
#[path = "cli/test/harness.rs"]
mod testbed_port;
mod cli_harness {
    pub(crate) use super::testbed_port::*;
}
#[path = "cli/workflow/job.rs"]
mod workload_port;
mod cli_job {
    pub(crate) use super::workload_port::*;
}
#[path = "cli/ops/ledger.rs"]
mod journal_port;
mod cli_ledger {
    pub(crate) use super::journal_port::*;
}
#[path = "cli/ops/nixosvm.rs"]
mod machine_port;
mod cli_nixos_vm {
    pub(crate) use super::machine_port::*;
}
#[path = "cli/ops/node.rs"]
mod kernel_shell;
mod cli_node {
    pub(crate) use super::kernel_shell::*;
}
#[path = "cli/ops/octet.rs"]
mod quality_port;
mod cli_octet {
    pub(crate) use super::quality_port::*;
}
#[path = "cli/ops/peer.rs"]
mod peer_port;
mod cli_peer {
    pub(crate) use super::peer_port::*;
}
#[path = "cli/ops/plugin.rs"]
mod extension_port;
mod cli_plugin {
    pub(crate) use super::extension_port::*;
}
#[path = "cli/ops/prodsoak.rs"]
mod burnin_port;
mod cli_prod_soak {
    pub(crate) use super::burnin_port::*;
}
#[path = "cli/workflow/protocol.rs"]
mod conversation_port;
mod cli_protocol {
    pub(crate) use super::conversation_port::*;
}
#[path = "cli/workflow/provenance.rs"]
mod lineage_port;
mod cli_provenance {
    pub(crate) use super::lineage_port::*;
}
#[path = "cli/runtime/raft.rs"]
mod quorum_port;
mod cli_raft {
    pub(crate) use super::quorum_port::*;
}
#[path = "cli/evidence/receipts.rs"]
mod voucher_port;
mod cli_receipts {
    pub(crate) use super::voucher_port::*;
}
#[path = "cli/workflow/remote.rs"]
mod mesh_port;
mod cli_remote {
    pub(crate) use super::mesh_port::*;
}
#[path = "cli/test/replayfixture.rs"]
mod scenario_port;
mod cli_replay_fixture {
    pub(crate) use super::scenario_port::*;
}
#[path = "cli/evidence/report.rs"]
mod display_port;
mod cli_report {
    pub(crate) use super::display_port::*;
}
#[path = "cli/runtime/repro.rs"]
mod bundle_port;
mod cli_repro {
    pub(crate) use super::bundle_port::*;
}
#[path = "cli/workflow/retention.rs"]
mod custody_port;
mod cli_retention {
    pub(crate) use super::custody_port::*;
}
#[path = "cli/runtime/rewrite.rs"]
mod transform_port;
mod cli_rewrite {
    pub(crate) use super::transform_port::*;
}
#[path = "main/root.rs"]
mod entrypoint;
mod cli_root {
    pub(crate) use super::entrypoint::*;
}
#[path = "cli/core/schema.rs"]
mod format_port;
mod cli_schema {
    pub(crate) use super::format_port::*;
}
#[path = "cli/runtime/secrets.rs"]
mod vault_port;
mod cli_secrets {
    pub(crate) use super::vault_port::*;
}
#[path = "cli/runtime/service.rs"]
mod worker_port;
mod cli_service {
    pub(crate) use super::worker_port::*;
}
#[path = "cli/runtime/fabric_time.rs"]
mod fabric_time_port;
mod cli_fabric_time {
    pub(crate) use super::fabric_time_port::*;
}
#[path = "cli/runtime/fabric_simulation.rs"]
mod fabric_simulation_port;
mod cli_fabric_simulation {
    pub(crate) use super::fabric_simulation_port::*;
}
#[path = "cli/runtime/system_extension.rs"]
mod system_extension_port;
mod cli_system_extension {
    pub(crate) use super::system_extension_port::*;
}
#[path = "cli/runtime/worldcommit.rs"]
mod world_commit_port;
mod cli_world_commit {
    pub(crate) use super::world_commit_port::*;
}
#[path = "cli/runtime/worldsnapshot.rs"]
mod world_snapshot_port;
mod cli_world_snapshot {
    pub(crate) use super::world_snapshot_port::*;
}
#[path = "cli/runtime/worldhead.rs"]
mod world_head_port;
mod cli_world_head {
    pub(crate) use super::world_head_port::*;
}
#[path = "cli/runtime/worlddistribution.rs"]
mod world_distribution_port;
mod cli_world_distribution {
    pub(crate) use super::world_distribution_port::*;
}
#[path = "cli/runtime/worldmerge.rs"]
mod world_merge_port;
mod cli_world_merge {
    pub(crate) use super::world_merge_port::*;
}
#[path = "cli/runtime/worldpromotion.rs"]
mod world_promotion_port;
mod cli_world_promotion {
    pub(crate) use super::world_promotion_port::*;
}
#[path = "cli/core/storage.rs"]
mod cell_port;
mod cli_storage {
    pub(crate) use super::cell_port::*;
}
#[path = "cli/core/transcript.rs"]
mod narrative_port;
mod cli_transcript {
    pub(crate) use super::narrative_port::*;
}
#[path = "cli/ops/traceability.rs"]
mod trace_port;
mod cli_traceability {
    pub(crate) use super::trace_port::*;
}
#[path = "cli/runtime/upgrade.rs"]
mod migration_port;
mod cli_upgrade {
    pub(crate) use super::migration_port::*;
}
#[path = "cli/runtime/vat.rs"]
mod actor_port;
mod cli_vat {
    pub(crate) use super::actor_port::*;
}

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
    molten::profiling::enable_development_profiler();
    if let Err(error) = cli_root::run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
