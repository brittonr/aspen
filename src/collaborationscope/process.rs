//! Actual OS processes, Iroh peers, Redb recovery and collaboration application.
use super::*;
#[path = "process/child.rs"]
mod child;
#[path = "process/flow.rs"]
mod flow;
const ROOT_ENV: &str = "MOLTEN_SCOPE_PROCESS_ROOT";
const NODE_ENV: &str = "MOLTEN_SCOPE_PROCESS_NODE";
const PHASE_ENV: &str = "MOLTEN_SCOPE_PROCESS_PHASE";
const FILTER: &str = "fabric_consistency::raft::collaboration_process::participant";
const NODES: [&str; 3] = ["node-a", "node-b", "node-c"];
const POLLS: usize = 3000;
#[test]
fn participant() {
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return;
    };
    let node = std::env::var(NODE_ENV).expect("node");
    let phase = std::env::var(PHASE_ENV).expect("phase");
    assert!(NODES.contains(&node.as_str()));
    assert!(matches!(phase.as_str(), "fresh" | "recover"));
    tokio::runtime::Runtime::new()
        .expect("runtime")
        .block_on(child::run(std::path::Path::new(&root), &node, &phase))
        .expect("actual participant");
}
// r[verify molten.collaboration_scope.validation]
#[test]
fn three_participant_conflict_partition_restart_snapshot_and_currentness_recovery() {
    let workspace = crate::test_support::process_workspace("collaboration-three-process").expect("workspace");
    let root: &std::path::Path = &workspace;
    let mut fresh = spawn(root, "fresh");
    wait_nodes(root, "fresh", "ready");
    signal(&root.join("fresh-start.preserves"));
    wait_nodes(root, "fresh", "done");
    let pids: std::collections::BTreeSet<_> = fresh.iter().map(|child| child.0.id()).collect();
    assert_eq!(pids.len(), 3);
    check_states(root, "fresh", 3, 1, 3);
    for child in &mut fresh {
        child.0.kill().expect("crash");
        child.0.wait().expect("reap");
    }
    let mut recovered = spawn(root, "recover");
    let new_pids: std::collections::BTreeSet<_> = recovered.iter().map(|child| child.0.id()).collect();
    assert_eq!(new_pids.len(), 3);
    assert!(pids.is_disjoint(&new_pids));
    wait_nodes(root, "recover", "ready");
    signal(&root.join("recover-start.preserves"));
    wait_nodes(root, "recover", "done");
    check_states(root, "recover", 4, 2, 4);
    signal(&root.join("recover-stop.preserves"));
    for child in &mut recovered {
        child.wait();
    }
}
fn check_states(root: &std::path::Path, phase: &str, epoch: u64, count: usize, index: u64) {
    let states: Vec<_> = NODES
        .iter()
        .map(|node| {
            crate::collaboration_scope::parse_state(&read(&file(root, phase, node, "state")).expect("bytes"))
                .expect("state")
        })
        .collect();
    let first = crate::collaboration_scope::validated_state_ref(&states[0]).expect("identity");
    for state in &states {
        assert_eq!(crate::collaboration_scope::validated_state_ref(state).expect("identity"), first);
        let scope = &state.scopes[&crate::collaboration_scope::fixtures::space_ref()];
        assert_eq!(scope.membership_epoch, epoch);
        assert_eq!(scope.members.len(), count);
        assert_eq!(state.currentness.committed_index, index);
        assert!(
            !scope.members.contains_key(
                &crate::collaboration_scope::SubjectRef::new(crate::collaboration_scope::fixtures::reference(
                    "subject-partitioned-member"
                ))
                .expect("subject")
            )
        );
        if phase == "recover" {
            assert!(state.currentness.consistency_epoch > 1);
        }
    }
}
struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl Child {
    fn wait(&mut self) {
        for _ in 0..POLLS {
            if let Some(status) = self.0.try_wait().expect("status") {
                assert!(status.success());
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("scope participant deadline");
    }
}
fn spawn(root: &std::path::Path, phase: &str) -> Vec<Child> {
    NODES
        .iter()
        .map(|node| {
            Child(
                std::process::Command::new(std::env::current_exe().expect("executable"))
                    .args(["--exact", FILTER, "--nocapture"])
                    .env(ROOT_ENV, root)
                    .env(NODE_ENV, node)
                    .env(PHASE_ENV, phase)
                    .stdin(std::process::Stdio::null())
                    .spawn()
                    .expect("participant"),
            )
        })
        .collect()
}
fn file(root: &std::path::Path, phase: &str, node: &str, kind: &str) -> std::path::PathBuf {
    root.join(format!("{phase}-{node}-{kind}.preserves"))
}
fn wait_nodes(root: &std::path::Path, phase: &str, kind: &str) {
    for _ in 0..POLLS {
        if NODES.iter().all(|node| file(root, phase, node, kind).is_file()) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("scope barrier {phase}/{kind}");
}
fn signal(path: &std::path::Path) {
    write(path, &crate::preserves_rail::record("scope-process-signal-v1", Vec::new())).expect("signal");
}
fn write(path: &std::path::Path, value: &preserves::IOValue) -> crate::error::Result<()> {
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    let temporary = path.with_extension("pending");
    std::fs::write(&temporary, bytes).map_err(crate::error::Failure::from)?;
    std::fs::rename(temporary, path).map_err(crate::error::Failure::from)
}
fn read(path: &std::path::Path) -> crate::error::Result<preserves::IOValue> {
    let bytes = std::fs::read(path).map_err(crate::error::Failure::from)?;
    if bytes.len() as u64 > crate::collaboration_scope::MAX_RECORD_BYTES {
        return Err(invalid("process read bound"));
    }
    crate::preserves_rail::parse_canonical_bytes(&bytes)
}
fn invalid(message: &'static str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
fn delivered(outcome: ReplicaExecutionOutcome) -> crate::error::Result<()> {
    match outcome {
        ReplicaExecutionOutcome::Failed(failed) => {
            Err(crate::error::Failure::invalid_harness(format!("actual inbound replica message failed: {failed:?}")))
        }
        ReplicaExecutionOutcome::Applied(_) | ReplicaExecutionOutcome::Denied { .. } => Ok(()),
    }
}
fn applied(outcome: ReplicaExecutionOutcome) -> crate::error::Result<()> {
    match outcome {
        ReplicaExecutionOutcome::Applied(_) => Ok(()),
        ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
            Err(crate::error::Failure::invalid_harness(format!("actual replica effect denied: {diagnostic}")))
        }
        ReplicaExecutionOutcome::Failed(failed) => {
            Err(crate::error::Failure::invalid_harness(format!("actual replica effect failed: {failed:?}")))
        }
    }
}
