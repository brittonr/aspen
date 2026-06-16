use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::raft_control_plane;

#[derive(Debug, Subcommand)]
pub(crate) enum RaftCommand {
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_raft_command(command: RaftCommand) -> Result<()> {
    match command {
        RaftCommand::RunFixture { out } => {
            let runtime = raft_control_plane::run_control_registry_fixture()?;
            let read = raft_control_plane::read_control_registry(&raft_control_plane::ControlRegistryReadInput {
                state: runtime.state.value.clone(),
                group_ref: runtime.manifest.manifest_ref.clone(),
                committed_term: runtime.term,
                committed_index: runtime.committed_index,
                read_index: runtime.committed_index,
                namespace: "protocol".to_string(),
                name: "proto:request-response".to_string(),
                authority_refs: vec![cli_synthetic_ref("raft-read-authority")?],
                resource_refs: runtime.manifest.resource_refs.clone(),
            })?;
            let log_refs = runtime.log_entries.iter().map(|entry| entry.entry_ref.clone()).collect::<Vec<_>>();
            let snapshot = raft_control_plane::snapshot_control_registry(&raft_control_plane::RaftSnapshotInput {
                group_ref: runtime.manifest.manifest_ref.clone(),
                term: runtime.term,
                index: runtime.committed_index,
                state: runtime.state.value.clone(),
                log_refs,
            })?;
            let recovery = raft_control_plane::recover_control_registry(&raft_control_plane::RaftRecoveryInput {
                group_ref: runtime.manifest.manifest_ref.clone(),
                snapshot: snapshot.value.clone(),
                log_entries: Vec::new(),
            })?;
            write_raft_fixture(&out, &runtime, &read.value, &snapshot.value, &recovery.value)?;
            println!(
                "raft fixture committed={} entries={} state={} out={}",
                runtime.committed_index,
                runtime.state.entries.len(),
                runtime.state.state_ref,
                out.display()
            );
            Ok(())
        }
        RaftCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", raft_artifact_summary(&value)?);
            Ok(())
        }
    }
}

fn write_raft_fixture(
    out: &Path,
    runtime: &raft_control_plane::ControlRegistryRuntime,
    read_receipt: &preserves::IOValue,
    snapshot: &preserves::IOValue,
    recovery: &preserves::IOValue,
) -> Result<()> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    write_file(&out.join("manifest.preserves"), &to_text(&runtime.manifest.value)?)?;
    write_file(&out.join("state.preserves"), &to_text(&runtime.state.value)?)?;
    write_file(&out.join("read-receipt.preserves"), &to_text(read_receipt)?)?;
    write_file(&out.join("snapshot.preserves"), &to_text(snapshot)?)?;
    write_file(&out.join("recovery-receipt.preserves"), &to_text(recovery)?)?;
    write_file(&out.join("summary.txt"), &raft_control_plane::control_registry_summary(runtime))?;
    write_indexed_values(
        out,
        "log-entry",
        &runtime.log_entries.iter().map(|entry| entry.value.clone()).collect::<Vec<_>>(),
    )?;
    write_indexed_values(
        out,
        "commit-receipt",
        &runtime.commit_receipts.iter().map(|receipt| receipt.value.clone()).collect::<Vec<_>>(),
    )?;
    write_indexed_values(
        out,
        "registry-receipt",
        &runtime.registry_receipts.iter().map(|receipt| receipt.value.clone()).collect::<Vec<_>>(),
    )?;
    write_indexed_values(
        out,
        "predicate-receipt",
        &runtime.predicate_receipts.iter().map(|receipt| receipt.value.clone()).collect::<Vec<_>>(),
    )
}

fn raft_artifact_summary(value: &preserves::IOValue) -> Result<String> {
    match molten::ledger::artifact_kind(value) {
        "control-registry-state" => {
            let state = raft_control_plane::parse_control_registry_state(value)?;
            Ok(format!(
                "control registry state ref={} entries={} sessions={}",
                state.state_ref,
                state.entries.len(),
                state.client_sessions.len()
            ))
        }
        "raft-group-manifest" => {
            let manifest = raft_control_plane::parse_raft_group_manifest(value)?;
            Ok(format!(
                "raft group manifest ref={} group={} members={}",
                manifest.manifest_ref,
                manifest.group_id,
                manifest.members.len()
            ))
        }
        kind => Ok(format!("raft artifact kind={} ref={}", kind, canonical_hash(value)?)),
    }
}

fn write_indexed_values(out: &Path, prefix: &str, values: &[preserves::IOValue]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &to_text(value)?)?;
    }
    Ok(())
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
