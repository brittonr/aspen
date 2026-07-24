use std::collections::BTreeSet;

use super::tests::test_ref;
use super::*;
use crate::error::MoltenError;
use crate::error::Result;

const FIRST_INDEX: u64 = 1;
const SECOND_INDEX: u64 = 2;
const SERVICE_GENERATION: u64 = 1;

#[derive(Debug, Default)]
struct RecordingBatchHandler {
    calls: usize,
    fail: bool,
}

impl CommittedBatchHandler for RecordingBatchHandler {
    fn restore_snapshot(&mut self, _snapshot: &ApplicationSnapshotRestore) -> Result<String> {
        self.calls += 1;
        if self.fail {
            return Err(MoltenError::invalid_harness("injected application failure"));
        }
        Ok(test_ref("application-snapshot-handler-evidence"))
    }

    fn apply_batch(&mut self, _commands: &[ApplicationCommand]) -> Result<String> {
        self.calls += 1;
        if self.fail {
            return Err(MoltenError::invalid_harness("injected application failure"));
        }
        Ok(test_ref("application-handler-evidence"))
    }
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn application_port_applies_one_contiguous_admitted_batch() {
    let schema_ref = test_ref("application-command-schema");
    let handler = RecordingBatchHandler::default();
    let mut port = application_port(schema_ref.clone(), handler).expect("application port");
    let entries = vec![entry(FIRST_INDEX, &schema_ref), entry(SECOND_INDEX, &schema_ref)];

    let receipt_ref = port.apply_committed(&entries).expect("committed batch");
    assert!(receipt_ref.starts_with("blake3:"));
    assert_eq!(port.last_applied_index(), SECOND_INDEX);
    assert_eq!(port.handler().calls, 1);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn application_port_restores_bound_snapshot_once() {
    let schema_ref = test_ref("snapshot-command-schema");
    let handler = RecordingBatchHandler::default();
    let mut port = application_port(schema_ref, handler).expect("application port");
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: test_ref("application-group"),
        membership_ref: test_ref("application-snapshot-membership"),
        config_epoch: SERVICE_GENERATION,
        fencing_epoch: SERVICE_GENERATION,
        last_included_index: FIRST_INDEX,
        last_included_term: SERVICE_GENERATION,
        application_state_ref: test_ref("application-snapshot-state"),
        completed_requests: std::collections::BTreeMap::from([(test_ref("application-snapshot-request"), FIRST_INDEX)]),
    };
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("snapshot identity");

    let receipt = port.restore_snapshot(&snapshot).expect("application snapshot restore");
    assert!(receipt.starts_with("blake3:"));
    assert_eq!(port.last_applied_index(), FIRST_INDEX);
    assert_eq!(port.handler().calls, 1);
    let duplicate = port.restore_snapshot(&snapshot).expect_err("duplicate snapshot must deny");
    assert!(duplicate.to_string().contains("stale or duplicated"));
    assert_eq!(port.handler().calls, 1);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn application_port_denies_gap_and_handler_failure_without_advancing_index() {
    let schema_ref = test_ref("application-command-schema-negative");
    let handler = RecordingBatchHandler::default();
    let mut gap_port = application_port(schema_ref.clone(), handler).expect("application port");
    let gap_error = gap_port.apply_committed(&[entry(SECOND_INDEX, &schema_ref)]).expect_err("gap must deny");
    assert!(gap_error.to_string().contains("noncontiguous"));
    assert_eq!(gap_port.handler().calls, 0);
    assert_eq!(gap_port.last_applied_index(), 0);

    let failing_handler = RecordingBatchHandler { calls: 0, fail: true };
    let mut failing_port = application_port(schema_ref.clone(), failing_handler).expect("failing application port");
    let failure = failing_port
        .apply_committed(&[entry(FIRST_INDEX, &schema_ref)])
        .expect_err("handler failure must retain index");
    assert!(failure.to_string().contains("injected application failure"));
    assert_eq!(failing_port.handler().calls, 1);
    assert_eq!(failing_port.last_applied_index(), 0);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn channel_control_port_publishes_bound_receipt_and_reports_closed_supervisor() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let config = control_config();
    let mut port = ChannelReplicaControlPort::new(config.clone(), sender).expect("control port");
    let request_ref = test_ref("proposal-control-request");
    let receipt_ref = port
        .proposal_outcome(&request_ref, ProposalDisposition::Committed, Some(FIRST_INDEX))
        .expect("proposal observation");
    let observation = receiver.try_recv().expect("supervision observation");
    assert_eq!(observation.receipt_ref, receipt_ref);
    assert!(matches!(observation.kind, ReplicaControlObservationKind::Proposal {
        disposition: ProposalDisposition::Committed,
        committed_index: Some(FIRST_INDEX),
        ..
    }));

    drop(receiver);
    let error = port.lifecycle_changed(ReplicaLifecycle::Draining).expect_err("closed supervisor must fail");
    assert!(error.to_string().contains("receiver is unavailable"));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn channel_control_port_denies_zero_generation_before_publication() {
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut config = control_config();
    config.service_generation = 0;
    let result = ChannelReplicaControlPort::new(config, sender);
    let error = result.err().expect("zero generation must deny");
    assert!(error.to_string().contains("generation must be positive"));
}

fn application_port(
    schema_ref: String,
    handler: RecordingBatchHandler,
) -> Result<AdmittedReplicaApplicationPort<RecordingBatchHandler>> {
    AdmittedReplicaApplicationPort::new(
        ReplicaApplicationConfig {
            group_binding_ref: test_ref("application-group"),
            application_manifest_ref: test_ref("application-manifest"),
            handler_ref: test_ref("application-handler"),
            command_schema_refs: BTreeSet::from([schema_ref]),
            initial_applied_index: 0,
        },
        handler,
    )
}

fn entry(index: u64, schema_ref: &str) -> ReplicatedEntry {
    ReplicatedEntry {
        index,
        term: SERVICE_GENERATION,
        request_ref: test_ref(&format!("application-request-{index}")),
        command_ref: test_ref(&format!("application-command-{index}")),
        command_schema_ref: schema_ref.to_string(),
    }
}

fn control_config() -> ReplicaControlConfig {
    ReplicaControlConfig {
        service_id: "raft-service".to_string(),
        service_generation: SERVICE_GENERATION,
        supervision_ref: test_ref("raft-supervision-binding"),
    }
}
