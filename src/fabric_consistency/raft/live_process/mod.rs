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
mod receipt;
mod recovered;

use receipt::*;

const CHILD_NODE_ENV: &str = "MOLTEN_LIVE_RAFT_CHILD_NODE";
const CHILD_RUN_DIRECTORY_ENV: &str = "MOLTEN_LIVE_RAFT_RUN_DIRECTORY";
const CHILD_MODE_ENV: &str = "MOLTEN_LIVE_RAFT_CHILD_MODE";
const CHILD_TEST_FILTER: &str = "fabric_consistency::raft::live_process::distinct_process_replica_child";
const CHILD_TIMEOUT_SECONDS: u64 = 30;
const FILE_POLL_MILLISECONDS: u64 = 10;
const FILE_POLL_LIMIT: usize = 3_000;
const RECEIPT_FIELD_COUNT: usize = 18;
const MAX_HARNESS_FILE_BYTES: u64 = 1_048_576;
const RECEIPT_SCHEMA: &str = "molten.fabric-consistency.distinct-process-participant.v1";
const START_FILE: &str = "start.preserves";
const STOP_FILE: &str = "stop.preserves";
const LEADER_DONE_FILE: &str = "leader-done.preserves";
const PARTITION_FILE: &str = "partition.preserves";
const CHECKPOINT_FILE: &str = "checkpoint.preserves";
const RESTART_START_FILE: &str = "restart-start.preserves";
const RECOVERED_LEADER_FILE: &str = "recovered-leader.preserves";
const STALE_FENCED_FILE: &str = "stale-fenced.preserves";
const REQUEST_LABEL: &str = "distinct-process-request";
const APPLICATION_STATE_LABEL: &str = "distinct-process-application-state";
const QUORUM_LOSS_REQUEST_LABEL: &str = "distinct-process-quorum-loss-request";
const STALE_LEADER_TERM: u64 = 1;
const RECOVERED_LEADER_TERM: u64 = 2;
const EVENT_LOOP_LIMIT: usize = 1_200;
const INGRESS_EVENT_CAPACITY: usize = 32;
const INGRESS_DELIVERY_LIMIT: u64 = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildMode {
    Fresh,
    Recover,
}

impl ChildMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Recover => "recover",
        }
    }
}

#[test]
fn distinct_process_replica_child() {
    let Some((node_id, run_directory, mode)) = child_invocation_from_environment().expect("child invocation") else {
        return;
    };
    let runtime = tokio::runtime::Runtime::new().expect("child Tokio runtime");
    runtime.block_on(child::run(node_id, run_directory, mode)).expect("distinct-process replica child");
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn three_process_live_raft_elects_commits_reads_and_catches_up() {
    let workspace = crate::test_support::process_workspace("live-raft-three-process").expect("process workspace");
    let run_directory = workspace.to_path_buf();
    let executable = std::env::current_exe().expect("current test executable");
    let mut children = [
        process::ChildGuard::spawn(&executable, &run_directory, NODE_A, ChildMode::Fresh).expect("node A child"),
        process::ChildGuard::spawn(&executable, &run_directory, NODE_B, ChildMode::Fresh).expect("node B child"),
        process::ChildGuard::spawn(&executable, &run_directory, NODE_C, ChildMode::Fresh).expect("node C child"),
    ];
    let child_process_ids = children.iter().map(process::ChildGuard::id).collect::<BTreeSet<_>>();
    assert_eq!(child_process_ids.len(), STATIC_VOTER_COUNT);
    wait_for_nodes(&run_directory, "ready").expect("child readiness");
    write_signal(&run_directory.join(START_FILE), "start").expect("start signal");
    if let Err(error) = wait_for_file(&run_directory.join(LEADER_DONE_FILE)) {
        panic!("leader completion: {error}\n{}", child_diagnostics(&run_directory));
    }
    write_signal(&run_directory.join(CHECKPOINT_FILE), "checkpoint").expect("checkpoint signal");
    wait_for_nodes(&run_directory, "checkpoint").expect("active checkpoint receipts");
    let active_receipts = read_checkpoint_receipts(&run_directory);
    assert_active_process_receipts(&active_receipts);
    for child in &mut children {
        child.crash().expect("injected child crash");
    }
    clear_phase_files(&run_directory).expect("restart phase cleanup");

    let mut recovered = [
        process::ChildGuard::spawn(&executable, &run_directory, NODE_A, ChildMode::Recover).expect("recovered node A"),
        process::ChildGuard::spawn(&executable, &run_directory, NODE_B, ChildMode::Recover).expect("recovered node B"),
        process::ChildGuard::spawn(&executable, &run_directory, NODE_C, ChildMode::Recover).expect("recovered node C"),
    ];
    let recovered_process_ids = recovered.iter().map(process::ChildGuard::id).collect::<BTreeSet<_>>();
    assert_eq!(recovered_process_ids.len(), STATIC_VOTER_COUNT);
    assert!(child_process_ids.is_disjoint(&recovered_process_ids));
    wait_for_nodes(&run_directory, "ready").expect("recovered child readiness");
    write_signal(&run_directory.join(RESTART_START_FILE), "restart-start").expect("restart election signal");
    wait_for_file(&run_directory.join(STALE_FENCED_FILE)).expect("stale leader fencing");
    write_signal(&run_directory.join(STOP_FILE), "stop").expect("recovered stop signal");
    if let Err(error) = wait_for_nodes(&run_directory, "terminal") {
        panic!("recovery receipts: {error}\n{}", child_diagnostics(&run_directory));
    }
    for child in &mut recovered {
        child.wait_success(Duration::from_secs(CHILD_TIMEOUT_SECONDS)).expect("clean recovered child exit");
    }
    let recovery_receipts = read_terminal_receipts(&run_directory);
    assert_recovery_receipts(&recovery_receipts);
}

#[test]
fn distinct_process_receipt_parser_rejects_wrong_schema() {
    let invalid = crate::preserves_rail::record("wrong-schema", Vec::new());
    let error = parse_receipt(&invalid).expect_err("wrong receipt schema must deny");
    assert!(error.to_string().contains("participant receipt"));
}

fn child_invocation_from_environment() -> Result<Option<(String, PathBuf, ChildMode)>> {
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
    let mode = match std::env::var(CHILD_MODE_ENV).as_deref() {
        Ok("fresh") => ChildMode::Fresh,
        Ok("recover") => ChildMode::Recover,
        _ => return Err(MoltenError::invalid_harness("live Raft child mode is absent or invalid")),
    };
    Ok(Some((node_id, PathBuf::from(run_directory), mode)))
}

fn child_diagnostics(run_directory: &Path) -> String {
    [NODE_A, NODE_B, NODE_C]
        .into_iter()
        .map(|node_id| {
            let log_path = run_directory.join(format!("{node_id}-child.log"));
            let log = match std::fs::read_to_string(log_path) {
                Ok(log) => log,
                Err(error) => format!("log read failed: {error}"),
            };
            format!("{node_id} child log:\n{log}")
        })
        .collect::<Vec<_>>()
        .join("\n")
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

fn durability_path(run_directory: &Path, node_id: &str) -> PathBuf {
    run_directory.join(format!("{node_id}-durability"))
}

fn clear_phase_files(run_directory: &Path) -> Result<()> {
    for name in [
        START_FILE,
        STOP_FILE,
        LEADER_DONE_FILE,
        PARTITION_FILE,
        CHECKPOINT_FILE,
        RESTART_START_FILE,
        RECOVERED_LEADER_FILE,
        STALE_FENCED_FILE,
    ] {
        remove_file_if_present(&run_directory.join(name))?;
    }
    for node_id in [NODE_A, NODE_B, NODE_C] {
        for path in [
            endpoint_path(run_directory, node_id),
            run_directory.join(format!("{node_id}-ready.preserves")),
            checkpoint_path(run_directory, node_id),
            receipt_path(run_directory, node_id),
        ] {
            remove_file_if_present(&path)?;
        }
    }
    Ok(())
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(MoltenError::from(error)),
    }
}
