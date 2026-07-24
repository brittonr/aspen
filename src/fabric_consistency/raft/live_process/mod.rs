use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use preserves::IOValue;

use super::tests::NODE_A;
use super::tests::NODE_B;
use super::tests::NODE_C;
use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_transport::CanonicalCrossProcessEndpoint;

mod child;
mod process;

const CHILD_NODE_ENV: &str = "MOLTEN_LIVE_RAFT_CHILD_NODE";
const CHILD_RUN_DIRECTORY_ENV: &str = "MOLTEN_LIVE_RAFT_RUN_DIRECTORY";
const CHILD_TEST_FILTER: &str = "fabric_consistency::raft::live_process::distinct_process_replica_child";
const CHILD_TIMEOUT_SECONDS: u64 = 30;
const FILE_POLL_MILLISECONDS: u64 = 10;
const FILE_POLL_LIMIT: usize = 3_000;
const RECEIPT_FIELD_COUNT: usize = 17;
const MAX_HARNESS_FILE_BYTES: u64 = 1_048_576;
const RECEIPT_SCHEMA: &str = "molten.fabric-consistency.distinct-process-participant.v1";
const START_FILE: &str = "start.preserves";
const STOP_FILE: &str = "stop.preserves";
const LEADER_DONE_FILE: &str = "leader-done.preserves";
const PARTITION_FILE: &str = "partition.preserves";
const REQUEST_LABEL: &str = "distinct-process-request";
const APPLICATION_STATE_LABEL: &str = "distinct-process-application-state";
const QUORUM_LOSS_REQUEST_LABEL: &str = "distinct-process-quorum-loss-request";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessReceipt {
    node_id: String,
    process_id: u64,
    endpoint_identity: String,
    role: ReplicaRole,
    term: u64,
    commit_index: u64,
    last_applied: u64,
    quorum_term: u64,
    pending_read_count: u64,
    snapshot_ref: String,
    request_completed: bool,
    quorum_loss_request_uncommitted: bool,
    application_applied: bool,
    application_restored: bool,
    durable_record_count: u64,
    durable_snapshot_count: u64,
    clean_shutdown: bool,
}

#[test]
fn distinct_process_replica_child() {
    let Some((node_id, run_directory)) = child_invocation_from_environment().expect("child invocation") else {
        return;
    };
    let runtime = tokio::runtime::Runtime::new().expect("child Tokio runtime");
    runtime.block_on(child::run(node_id, run_directory)).expect("distinct-process replica child");
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn three_process_live_raft_elects_commits_reads_and_catches_up() {
    let workspace = crate::test_support::process_workspace("live-raft-three-process").expect("process workspace");
    let run_directory = workspace.to_path_buf();
    let executable = std::env::current_exe().expect("current test executable");
    let mut children = [
        process::ChildGuard::spawn(&executable, &run_directory, NODE_A).expect("node A child"),
        process::ChildGuard::spawn(&executable, &run_directory, NODE_B).expect("node B child"),
        process::ChildGuard::spawn(&executable, &run_directory, NODE_C).expect("node C child"),
    ];
    let child_process_ids = children.iter().map(process::ChildGuard::id).collect::<BTreeSet<_>>();
    assert_eq!(child_process_ids.len(), STATIC_VOTER_COUNT);
    wait_for_nodes(&run_directory, "ready").expect("child readiness");
    write_signal(&run_directory.join(START_FILE), "start").expect("start signal");
    if let Err(error) = wait_for_file(&run_directory.join(LEADER_DONE_FILE)) {
        let diagnostics = [NODE_A, NODE_B, NODE_C]
            .into_iter()
            .map(|node_id| {
                let log_path = run_directory.join(format!("{node_id}-child.log"));
                let log = match std::fs::read_to_string(&log_path) {
                    Ok(log) => log,
                    Err(read_error) => format!("log read failed: {read_error}"),
                };
                format!("{node_id} child log:\n{log}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        panic!("leader completion: {error}\n{diagnostics}");
    }
    write_signal(&run_directory.join(STOP_FILE), "stop").expect("stop signal");
    for child in &mut children {
        child.wait_success(Duration::from_secs(CHILD_TIMEOUT_SECONDS)).expect("clean child exit");
    }

    let receipts = [
        read_receipt(&receipt_path(&run_directory, NODE_A)).expect("node A receipt"),
        read_receipt(&receipt_path(&run_directory, NODE_B)).expect("node B receipt"),
        read_receipt(&receipt_path(&run_directory, NODE_C)).expect("node C receipt"),
    ];
    assert_distinct_process_receipts(&receipts);
}

#[test]
fn distinct_process_receipt_parser_rejects_wrong_schema() {
    let invalid = crate::preserves_rail::record("wrong-schema", Vec::new());
    let error = parse_receipt(&invalid).expect_err("wrong receipt schema must deny");
    assert!(error.to_string().contains("participant receipt"));
}

fn child_invocation_from_environment() -> Result<Option<(String, PathBuf)>> {
    let Some(node_id) = std::env::var_os(CHILD_NODE_ENV) else {
        return Ok(None);
    };
    let run_directory = std::env::var_os(CHILD_RUN_DIRECTORY_ENV)
        .ok_or_else(|| MoltenError::invalid_harness("live Raft child is missing its explicit run directory"))?;
    let node_id = node_id
        .into_string()
        .map_err(|_| MoltenError::invalid_harness("live Raft child node ID is not UTF-8"))?;
    if ![NODE_A, NODE_B, NODE_C].contains(&node_id.as_str()) {
        return Err(MoltenError::invalid_harness("live Raft child node is outside static membership"));
    }
    Ok(Some((node_id, PathBuf::from(run_directory))))
}

fn wait_for_nodes(run_directory: &Path, suffix: &str) -> Result<()> {
    for node_id in [NODE_A, NODE_B, NODE_C] {
        wait_for_file(&run_directory.join(format!("{node_id}-{suffix}.preserves")))?;
    }
    Ok(())
}

fn wait_for_file(path: &Path) -> Result<()> {
    for _attempt in 0..FILE_POLL_LIMIT {
        if path.is_file() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(FILE_POLL_MILLISECONDS));
    }
    Err(MoltenError::invalid_harness(format!(
        "timed out waiting for explicit harness file {}",
        path.display()
    )))
}

fn write_signal(path: &Path, label: &str) -> Result<()> {
    write_value(
        path,
        &crate::preserves_rail::record("live-raft-harness-signal-v1", vec![crate::preserves_rail::string(label)]),
    )
}

fn write_value(path: &Path, value: &IOValue) -> Result<()> {
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes).map_err(MoltenError::from)?;
    std::fs::rename(temporary, path).map_err(MoltenError::from)
}

fn read_value(path: &Path) -> Result<IOValue> {
    let metadata = std::fs::metadata(path).map_err(MoltenError::from)?;
    if metadata.len() > MAX_HARNESS_FILE_BYTES {
        return Err(MoltenError::invalid_harness("live Raft harness file exceeds its byte bound"));
    }
    let bytes = std::fs::read(path).map_err(MoltenError::from)?;
    Ok(crate::preserves_rail::strict_canonical_decode(&bytes)?.value)
}

fn endpoint_path(run_directory: &Path, node_id: &str) -> PathBuf {
    run_directory.join(format!("{node_id}-endpoint.preserves"))
}

fn receipt_path(run_directory: &Path, node_id: &str) -> PathBuf {
    run_directory.join(format!("{node_id}-terminal.preserves"))
}

fn read_receipt(path: &Path) -> Result<ProcessReceipt> {
    parse_receipt(&read_value(path)?)
}

fn parse_receipt(value: &IOValue) -> Result<ProcessReceipt> {
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
    })
}

fn parse_role(value: &preserves::Value<IOValue>) -> Result<ReplicaRole> {
    match canonical::required_string(value, "receipt role")?.as_str() {
        "leader" => Ok(ReplicaRole::Leader),
        "follower" => Ok(ReplicaRole::Follower),
        _ => Err(MoltenError::invalid_harness("participant receipt has an invalid role")),
    }
}

fn assert_distinct_process_receipts(receipts: &[ProcessReceipt; STATIC_VOTER_COUNT]) {
    let process_ids = receipts.iter().map(|receipt| receipt.process_id).collect::<BTreeSet<_>>();
    let endpoint_ids = receipts.iter().map(|receipt| &receipt.endpoint_identity).collect::<BTreeSet<_>>();
    assert_eq!(process_ids.len(), STATIC_VOTER_COUNT);
    assert_eq!(endpoint_ids.len(), STATIC_VOTER_COUNT);
    assert!(receipts.iter().all(|receipt| receipt.clean_shutdown));
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
    let follower = &receipts[1];
    assert_eq!(follower.commit_index, INITIAL_LOG_INDEX);
    assert!(follower.application_applied);
    let caught_up = &receipts[2];
    assert_eq!(caught_up.commit_index, INITIAL_LOG_INDEX);
    assert!(caught_up.application_restored);
    assert_eq!(caught_up.durable_snapshot_count, 1);
}
