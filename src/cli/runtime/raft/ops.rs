type Command = super::RaftCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::RunFixture { out } => run_fixture(out),
        Command::MembershipPreflight { out, peer } => run_membership_preflight(out, &peer),
        Command::Show { artifact } => run_show(artifact),
    }
}

fn run_fixture(out: std::path::PathBuf) -> Outcome<()> {
    let runtime = molten::raft_control_plane::run_control_registry_fixture()?;
    let read =
        molten::raft_control_plane::read_control_registry(&molten::raft_control_plane::ControlRegistryReadInput {
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
    let snapshot =
        molten::raft_control_plane::snapshot_control_registry(&molten::raft_control_plane::RaftSnapshotInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            term: runtime.term,
            index: runtime.committed_index,
            state: runtime.state.value.clone(),
            log_refs,
        })?;
    let recovery =
        molten::raft_control_plane::recover_control_registry(&molten::raft_control_plane::RaftRecoveryInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            snapshot: snapshot.value.clone(),
            log_entries: Vec::new(),
        })?;
    super::io::write_fixture(&out, &runtime, &read.value, &snapshot.value, &recovery.value)?;
    println!(
        "raft fixture committed={} entries={} state={} out={}",
        runtime.committed_index,
        runtime.state.entries.len(),
        runtime.state.state_ref,
        out.display()
    );
    Ok(())
}

fn run_membership_preflight(out: std::path::PathBuf, peer: &str) -> Outcome<()> {
    let evidence_ref = cli_synthetic_ref("membership-evidence")?;
    let request = molten::raft_membership::RaftMembershipRequest {
        group_ref: cli_synthetic_ref("raft-group")?,
        target_peer_ref: cli_synthetic_ref(peer)?,
        target_session_ref: cli_synthetic_ref(&format!("session:{peer}"))?,
        requested_role: "voter".to_string(),
        configuration_ref: cli_synthetic_ref("configuration")?,
        peer_session_scope: "raft-membership".to_string(),
        authority_refs: vec![evidence_ref.clone()],
        policy_refs: vec![evidence_ref.clone()],
        resource_refs: vec![evidence_ref.clone()],
        source_gate_refs: vec![evidence_ref.clone()],
        provenance_refs: vec![evidence_ref.clone()],
        compatibility_refs: vec![evidence_ref.clone()],
        snapshot_refs: vec![evidence_ref.clone()],
        replay_refs: vec![evidence_ref.clone()],
        quorum_safety_refs: vec![evidence_ref.clone()],
        operator_evidence_refs: vec![evidence_ref],
    };
    let receipt = molten::raft_membership::preflight_raft_membership(&request)?;
    std::fs::write(&out, molten::preserves_rail::to_text(&receipt.value)?)?;
    println!(
        "raft membership preflight peer={} decision={} receipt={} out={}",
        peer,
        receipt.decision,
        receipt.receipt_ref,
        out.display()
    );
    Ok(())
}

fn run_show(artifact: std::path::PathBuf) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    println!("{}", super::io::artifact_summary(&value)?);
    Ok(())
}

fn cli_synthetic_ref(label: &str) -> Outcome<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("remote-cli-ref", vec![
        molten::preserves_rail::string(label),
    ]))
}
