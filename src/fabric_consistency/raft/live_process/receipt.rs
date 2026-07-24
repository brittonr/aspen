use preserves::ValueImpl as _;

use super::*;
use crate::fabric_consistency::raft::live_cluster;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProcessReceipt {
    pub(super) node_id: String,
    pub(super) process_id: u64,
    pub(super) endpoint_identity: String,
    pub(super) role: ReplicaRole,
    pub(super) term: u64,
    pub(super) commit_index: u64,
    pub(super) last_applied: u64,
    pub(super) quorum_term: u64,
    pub(super) pending_read_count: u64,
    pub(super) snapshot_ref: String,
    pub(super) request_completed: bool,
    pub(super) quorum_loss_request_uncommitted: bool,
    pub(super) application_applied: bool,
    pub(super) application_restored: bool,
    pub(super) durable_record_count: u64,
    pub(super) durable_snapshot_count: u64,
    pub(super) clean_shutdown: bool,
    pub(super) recovery_ref: String,
    pub(super) group_binding_ref: String,
    pub(super) membership_ref: String,
    pub(super) config_epoch: u64,
    pub(super) admitted_voters: Vec<String>,
    pub(super) commit_effect_ref: String,
    pub(super) commit_quorum_ref: String,
    pub(super) commit_quorum_members: Vec<String>,
}

pub(super) fn receipt_path(run_directory: &Path, node_id: &str) -> PathBuf {
    run_directory.join(format!("{node_id}-terminal.preserves"))
}

pub(super) fn checkpoint_path(run_directory: &Path, node_id: &str) -> PathBuf {
    run_directory.join(format!("{node_id}-checkpoint.preserves"))
}

pub(super) fn read_checkpoint_receipts(run_directory: &Path) -> [ProcessReceipt; STATIC_VOTER_COUNT] {
    [
        read_receipt(&checkpoint_path(run_directory, NODE_A)).expect("node A checkpoint"),
        read_receipt(&checkpoint_path(run_directory, NODE_B)).expect("node B checkpoint"),
        read_receipt(&checkpoint_path(run_directory, NODE_C)).expect("node C checkpoint"),
    ]
}

pub(super) fn read_terminal_receipts(run_directory: &Path) -> [ProcessReceipt; STATIC_VOTER_COUNT] {
    [
        read_receipt(&receipt_path(run_directory, NODE_A)).expect("node A terminal receipt"),
        read_receipt(&receipt_path(run_directory, NODE_B)).expect("node B terminal receipt"),
        read_receipt(&receipt_path(run_directory, NODE_C)).expect("node C terminal receipt"),
    ]
}

fn read_receipt(path: &Path) -> Result<ProcessReceipt> {
    parse_receipt(&read_value(path)?)
}

pub(super) fn parse_receipt(value: &IOValue) -> Result<ProcessReceipt> {
    let fields = canonical::required_record(value, RECEIPT_SCHEMA, RECEIPT_FIELD_COUNT)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid participant receipt: {error}")))?;
    Ok(ProcessReceipt {
        node_id: canonical::required_string(&fields[0], "receipt node")?,
        process_id: canonical::required_u64(&fields[1], "receipt process")?,
        endpoint_identity: canonical::required_string(&fields[2], "receipt endpoint")?,
        role: parse_role(&fields[3])?,
        term: canonical::required_u64(&fields[4], "receipt term")?,
        commit_index: canonical::required_u64(&fields[5], "receipt commit")?,
        last_applied: canonical::required_u64(&fields[6], "receipt applied")?,
        quorum_term: canonical::required_u64(&fields[7], "receipt quorum term")?,
        pending_read_count: canonical::required_u64(&fields[8], "receipt pending reads")?,
        snapshot_ref: canonical::required_string(&fields[9], "receipt snapshot")?,
        request_completed: canonical::required_bool(&fields[10], "receipt completed request")?,
        quorum_loss_request_uncommitted: canonical::required_bool(&fields[11], "receipt quorum-loss request")?,
        application_applied: canonical::required_bool(&fields[12], "receipt application apply")?,
        application_restored: canonical::required_bool(&fields[13], "receipt application restore")?,
        durable_record_count: canonical::required_u64(&fields[14], "receipt durable records")?,
        durable_snapshot_count: canonical::required_u64(&fields[15], "receipt durable snapshots")?,
        clean_shutdown: canonical::required_bool(&fields[16], "receipt clean shutdown")?,
        recovery_ref: canonical::required_string(&fields[17], "receipt recovery ref")?,
        group_binding_ref: canonical::required_string(&fields[18], "receipt group binding")?,
        membership_ref: canonical::required_string(&fields[19], "receipt membership")?,
        config_epoch: canonical::required_u64(&fields[20], "receipt configuration epoch")?,
        admitted_voters: parse_member_sequence(&fields[21], "receipt admitted voters")?,
        commit_effect_ref: canonical::required_string(&fields[22], "receipt commit effect")?,
        commit_quorum_ref: canonical::required_string(&fields[23], "receipt commit quorum")?,
        commit_quorum_members: parse_member_sequence(&fields[24], "receipt commit quorum members")?,
    })
}

fn parse_member_sequence(value: &preserves::Value<IOValue>, label: &str) -> Result<Vec<std::string::String>> {
    let members = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    if members.len() > STATIC_VOTER_COUNT {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds the static voter bound")));
    }
    members.iter().map(|member| canonical::required_string(&member, label)).collect()
}

fn parse_role(value: &preserves::Value<IOValue>) -> Result<ReplicaRole> {
    match canonical::required_string(value, "receipt role")?.as_str() {
        "leader" => Ok(ReplicaRole::Leader),
        "follower" => Ok(ReplicaRole::Follower),
        _ => Err(MoltenError::invalid_harness("participant receipt has an invalid role")),
    }
}

pub(super) fn receipt_from_node(
    node_id: &str,
    endpoint_identity: &str,
    node: &live_cluster::LiveNode,
) -> Result<ProcessReceipt> {
    let state = node.service.state();
    let application = node.service.ports().application.handler();
    let durability = node.service.ports().durability.adapter().state();
    let commit_evidence =
        node.service.evidence().records().iter().find(|record| record.kind == ReplicaEvidenceKind::Commit);
    let commit_effect_ref = commit_evidence.map_or_else(String::new, |record| record.source_ref.clone());
    let commit_quorum_ref = commit_evidence.and_then(|record| record.quorum_evidence_ref.clone()).unwrap_or_default();
    let commit_quorum_members = commit_evidence.map_or_else(Vec::new, |record| record.quorum_members.clone());
    Ok(ProcessReceipt {
        node_id: node_id.to_string(),
        process_id: u64::from(std::process::id()),
        endpoint_identity: endpoint_identity.to_string(),
        role: state.role,
        term: state.current_term,
        commit_index: state.commit_index,
        last_applied: state.last_applied,
        quorum_term: state.quorum_confirmed_term.unwrap_or(INITIAL_TERM),
        pending_read_count: u64::try_from(state.pending_reads.len())
            .map_err(|_| MoltenError::invalid_harness("pending read count overflow"))?,
        snapshot_ref: state.snapshot.as_ref().map_or_else(String::new, |snapshot| snapshot.snapshot_ref.clone()),
        request_completed: state.completed_requests.contains_key(&super::super::tests::test_ref(REQUEST_LABEL)),
        quorum_loss_request_uncommitted: !state
            .completed_requests
            .contains_key(&super::super::tests::test_ref(QUORUM_LOSS_REQUEST_LABEL)),
        application_applied: application.applied_request_refs.contains(&super::super::tests::test_ref(REQUEST_LABEL)),
        application_restored: application.restored_application_state_ref
            == Some(super::super::tests::test_ref(APPLICATION_STATE_LABEL)),
        durable_record_count: u64::try_from(durability.durable_log.len())
            .map_err(|_| MoltenError::invalid_harness("durable record count overflow"))?,
        durable_snapshot_count: u64::try_from(durability.snapshots.len())
            .map_err(|_| MoltenError::invalid_harness("durable snapshot count overflow"))?,
        clean_shutdown: false,
        recovery_ref: node.recovery_ref.clone().unwrap_or_default(),
        group_binding_ref: state.profile.group_binding_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        config_epoch: state.membership.config_epoch,
        admitted_voters: state.membership.voters.clone(),
        commit_effect_ref,
        commit_quorum_ref,
        commit_quorum_members,
    })
}

pub(super) fn receipt_value(receipt: &ProcessReceipt) -> IOValue {
    crate::preserves_rail::record(RECEIPT_SCHEMA, vec![
        crate::preserves_rail::string(&receipt.node_id),
        crate::preserves_rail::u64_value(receipt.process_id),
        crate::preserves_rail::string(&receipt.endpoint_identity),
        crate::preserves_rail::string(match receipt.role {
            ReplicaRole::Leader => "leader",
            _ => "follower",
        }),
        crate::preserves_rail::u64_value(receipt.term),
        crate::preserves_rail::u64_value(receipt.commit_index),
        crate::preserves_rail::u64_value(receipt.last_applied),
        crate::preserves_rail::u64_value(receipt.quorum_term),
        crate::preserves_rail::u64_value(receipt.pending_read_count),
        crate::preserves_rail::string(&receipt.snapshot_ref),
        crate::preserves_rail::bool_value(receipt.request_completed),
        crate::preserves_rail::bool_value(receipt.quorum_loss_request_uncommitted),
        crate::preserves_rail::bool_value(receipt.application_applied),
        crate::preserves_rail::bool_value(receipt.application_restored),
        crate::preserves_rail::u64_value(receipt.durable_record_count),
        crate::preserves_rail::u64_value(receipt.durable_snapshot_count),
        crate::preserves_rail::bool_value(receipt.clean_shutdown),
        crate::preserves_rail::string(&receipt.recovery_ref),
        crate::preserves_rail::string(&receipt.group_binding_ref),
        crate::preserves_rail::string(&receipt.membership_ref),
        crate::preserves_rail::u64_value(receipt.config_epoch),
        member_sequence_value(&receipt.admitted_voters),
        crate::preserves_rail::string(&receipt.commit_effect_ref),
        crate::preserves_rail::string(&receipt.commit_quorum_ref),
        member_sequence_value(&receipt.commit_quorum_members),
    ])
}

fn member_sequence_value(members: &[std::string::String]) -> IOValue {
    let values = members.iter().map(|member| crate::preserves_rail::string(member.as_str())).collect();
    crate::preserves_rail::sequence(values)
}

pub(super) fn assert_active_process_receipts(receipts: &[ProcessReceipt; STATIC_VOTER_COUNT]) {
    assert_distinct_identity(receipts);
    assert!(receipts.iter().all(|receipt| !receipt.clean_shutdown));
    assert!(receipts.iter().all(|receipt| receipt.recovery_ref.is_empty()));
    let leader = &receipts[0];
    assert_eq!(leader.role, ReplicaRole::Leader);
    assert_eq!(leader.commit_index, INITIAL_LOG_INDEX);
    assert_eq!(leader.last_applied, INITIAL_LOG_INDEX);
    assert_eq!(leader.quorum_term, leader.term);
    assert_eq!(leader.pending_read_count, 1);
    assert!(leader.request_completed);
    assert!(leader.quorum_loss_request_uncommitted);
    assert!(leader.application_applied);
    assert!(leader.durable_record_count > 0);
    let quorum_ref = validate_process_commit_quorum(leader).expect("offline process commit quorum evidence");
    assert_eq!(quorum_ref, leader.commit_quorum_ref);
    assert_eq!(receipts[1].commit_index, INITIAL_LOG_INDEX);
    assert!(receipts[1].application_applied);
    assert_eq!(receipts[2].commit_index, INITIAL_LOG_INDEX);
    assert!(receipts[2].application_restored);
    assert_eq!(receipts[2].durable_snapshot_count, 1);
}

pub(super) fn assert_recovery_receipts(receipts: &[ProcessReceipt; STATIC_VOTER_COUNT]) {
    assert_distinct_identity(receipts);
    for receipt in receipts {
        assert!(receipt.clean_shutdown);
        assert_eq!(receipt.term, RECOVERED_LEADER_TERM);
        assert_eq!(receipt.commit_index, INITIAL_LOG_INDEX);
        assert_eq!(receipt.last_applied, INITIAL_LOG_INDEX);
        assert_eq!(receipt.pending_read_count, 0);
        assert!(receipt.request_completed);
        assert!(receipt.quorum_loss_request_uncommitted);
        crate::preserves_rail::validate_content_ref(&receipt.recovery_ref).expect("recovery receipt ref");
    }
    assert_eq!(receipts[0].role, ReplicaRole::Follower);
    assert_eq!(receipts[1].role, ReplicaRole::Leader);
    assert_eq!(receipts[1].quorum_term, RECOVERED_LEADER_TERM);
    assert_eq!(receipts[2].role, ReplicaRole::Follower);
    assert!(receipts[0].application_restored);
    assert!(receipts[1].application_applied);
    assert!(receipts[2].application_restored);
    assert_eq!(receipts[0].durable_snapshot_count, 1);
    assert_eq!(receipts[2].durable_snapshot_count, 1);
}

pub(super) fn validate_process_commit_quorum(receipt: &ProcessReceipt) -> Result<String> {
    let validated = validate_replica_quorum_evidence(&ReplicaQuorumEvidence {
        boundary: ReplicaQuorumEvidenceBoundary::Commit,
        group_binding_ref: receipt.group_binding_ref.clone(),
        membership_ref: receipt.membership_ref.clone(),
        config_epoch: receipt.config_epoch,
        term: receipt.term,
        index: receipt.commit_index,
        admitted_voters: receipt.admitted_voters.clone(),
        acknowledgement_members: receipt.commit_quorum_members.clone(),
        source_ref: receipt.commit_effect_ref.clone(),
    })?;
    if validated.evidence_ref != receipt.commit_quorum_ref {
        return Err(MoltenError::invalid_harness("distinct-process commit quorum evidence binding mismatch"));
    }
    Ok(validated.evidence_ref)
}

fn assert_distinct_identity(receipts: &[ProcessReceipt; STATIC_VOTER_COUNT]) {
    let process_ids = receipts.iter().map(|receipt| receipt.process_id).collect::<BTreeSet<_>>();
    let endpoint_ids = receipts.iter().map(|receipt| &receipt.endpoint_identity).collect::<BTreeSet<_>>();
    assert_eq!(process_ids.len(), STATIC_VOTER_COUNT);
    assert_eq!(endpoint_ids.len(), STATIC_VOTER_COUNT);
}
