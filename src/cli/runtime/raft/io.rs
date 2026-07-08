type Outcome<T> = molten::error::Result<T>;

pub(super) fn write_fixture(
    out: &std::path::Path,
    runtime: &molten::raft_control_plane::ControlRegistryRuntime,
    read_receipt: &preserves::IOValue,
    snapshot: &preserves::IOValue,
    recovery: &preserves::IOValue,
) -> Outcome<()> {
    std::fs::create_dir_all(out).map_err(molten::error::MoltenError::from)?;
    write_file(out.join("manifest.preserves"), &molten::preserves_rail::to_text(&runtime.manifest.value)?)?;
    write_file(out.join("state.preserves"), &molten::preserves_rail::to_text(&runtime.state.value)?)?;
    write_file(out.join("read-receipt.preserves"), &molten::preserves_rail::to_text(read_receipt)?)?;
    write_file(out.join("snapshot.preserves"), &molten::preserves_rail::to_text(snapshot)?)?;
    write_file(out.join("recovery-receipt.preserves"), &molten::preserves_rail::to_text(recovery)?)?;
    write_file(out.join("summary.txt"), &molten::raft_control_plane::control_registry_summary(runtime))?;
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

pub(super) fn artifact_summary(value: &preserves::IOValue) -> Outcome<String> {
    match molten::ledger::artifact_kind(value) {
        "control-registry-state" => {
            let state = molten::raft_control_plane::parse_control_registry_state(value)?;
            Ok(format!(
                "control registry state ref={} entries={} sessions={}",
                state.state_ref,
                state.entries.len(),
                state.client_sessions.len()
            ))
        }
        "raft-group-manifest" => {
            let manifest = molten::raft_control_plane::parse_raft_group_manifest(value)?;
            Ok(format!(
                "raft group manifest ref={} group={} profile={} version={} production={} members={} read_modes={} placement={} caveats={}",
                manifest.manifest_ref,
                manifest.group_id,
                manifest.algorithm_profile,
                manifest.admitted_profile_version,
                manifest.production_status,
                manifest.members.len(),
                manifest.read_consistency_support.join(","),
                manifest.placement_ref.as_deref().unwrap_or("none"),
                manifest.fault_model_caveats.join(",")
            ))
        }
        "consensus-placement-report" => {
            let report = molten::raft_control_plane::parse_consensus_placement_report(value)?;
            Ok(format!(
                "consensus placement decision={} group={} members={} diagnostics={}",
                report.decision,
                report.group_id,
                report.admitted_members.len(),
                report.diagnostics.join(";")
            ))
        }
        "consensus-non-claim-receipt" => {
            let receipt = molten::raft_control_plane::parse_consensus_claim_boundary_receipt(value)?;
            Ok(format!(
                "consensus non-claim decision={} claim={} diagnostics={}",
                receipt.decision,
                receipt.claim,
                receipt.diagnostics.join(";")
            ))
        }
        "consensus-simulation-receipt" => {
            let receipt = molten::raft_control_plane::parse_consensus_simulation_receipt(value)?;
            Ok(format!(
                "consensus simulation decision={} scenario={} final_state={} diagnostics={}",
                receipt.decision,
                receipt.scenario,
                receipt.final_state_ref.as_deref().unwrap_or("none"),
                receipt.diagnostics.join(";")
            ))
        }
        kind => Ok(format!("raft artifact kind={} ref={}", kind, molten::preserves_rail::canonical_hash(value)?)),
    }
}

pub(super) fn read_preserves_file(path: &std::path::Path) -> Outcome<preserves::IOValue> {
    let text = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
    molten::preserves_rail::parse_text(&text)
}

fn write_indexed_values(out: &std::path::Path, prefix: &str, values: &[preserves::IOValue]) -> Outcome<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(out.join(format!("{prefix}-{index}.preserves")), &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

fn write_file(path: std::path::PathBuf, contents: &str) -> Outcome<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, contents).map_err(molten::error::MoltenError::from)
}
