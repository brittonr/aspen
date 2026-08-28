use std::collections::BTreeSet;

use molten_core::world_replay::*;

use super::super::*;
use super::fixture::*;
use super::ports::*;

#[test]
fn logical_replay_uses_ports_and_emits_bounded_receipts() {
    // r[verify molten.world_replay.transition_chain]
    // r[verify molten.world_replay.receipts]
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let mut materialization = Materialization::default();
    let mut restore = Restore::default();
    let mut admission = Admission { allowed: true };
    let mut transitions = Transitions::default();
    let mut capture = Capture::from_fixture(&fixture);
    let mut receipts = Receipts::default();
    let outcome = run(
        &fixture,
        &mut materialization,
        &mut restore,
        &mut admission,
        &mut transitions,
        &mut capture,
        &mut receipts,
    )
    .expect("logical replay");

    assert_eq!(outcome.receipt.decision, WorldReplayReceiptDecision::Replayed);
    assert_eq!(outcome.receipt.horizon, EXPECTED_TRANSITIONS);
    assert_eq!(transitions.positions, vec![0, 1]);
    assert_eq!(restore.logical_calls, 1);
    assert_eq!(restore.opaque_calls, 0);
    assert_eq!(materialization.members, fixture.request.capsule.members.len());
    assert_eq!(
        fixture
            .request
            .capsule
            .members
            .iter()
            .map(|member| member.object_ref.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        fixture.request.capsule.members.len()
    );
    assert!(outcome.divergence_record.is_none());
    assert!(receipts.kinds.contains(&WORLD_REPLAY_RECEIPT_RECORD));
}

#[test]
fn current_authority_denial_emits_receipt_before_transition_execution() {
    // r[verify molten.world_replay.execution_boundary]
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let mut materialization = Materialization::default();
    let mut restore = Restore::default();
    let mut admission = Admission { allowed: false };
    let mut transitions = Transitions::default();
    let mut capture = Capture::from_fixture(&fixture);
    let mut receipts = Receipts::default();
    let outcome = run(
        &fixture,
        &mut materialization,
        &mut restore,
        &mut admission,
        &mut transitions,
        &mut capture,
        &mut receipts,
    )
    .expect("bounded denial outcome");

    assert_eq!(outcome.receipt.decision, WorldReplayReceiptDecision::Denied);
    assert!(outcome.executions.is_empty());
    assert!(transitions.positions.is_empty());
    assert_eq!(outcome.receipt.diagnostics, vec!["current replay admission denied"]);
}

#[test]
fn replay_stops_before_later_steps_after_first_divergence() {
    // r[verify molten.world_replay.divergence]
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let mut materialization = Materialization::default();
    let mut restore = Restore::default();
    let mut admission = Admission { allowed: true };
    let mut transitions = Transitions::default();
    let mut capture = Capture::from_fixture(&fixture);
    capture.diverge_at = Some(0);
    let mut receipts = Receipts::default();
    let outcome = run(
        &fixture,
        &mut materialization,
        &mut restore,
        &mut admission,
        &mut transitions,
        &mut capture,
        &mut receipts,
    )
    .expect("divergence outcome");

    assert_eq!(outcome.receipt.decision, WorldReplayReceiptDecision::Diverged);
    assert_eq!(transitions.positions, vec![0]);
    assert!(outcome.divergence_record.is_some());
    assert!(outcome.receipt.divergence_ref.is_some());
}

#[test]
fn exact_opaque_replay_runs_without_logical_fallback() {
    // r[verify molten.world_replay.execution_boundary]
    let fixture = fixture(WorldReplayProfileKind::Opaque);
    let mut materialization = Materialization::default();
    let mut restore = Restore::default();
    let mut admission = Admission { allowed: true };
    let mut transitions = Transitions::default();
    let mut capture = Capture::from_fixture(&fixture);
    let mut receipts = Receipts::default();
    let outcome = run(
        &fixture,
        &mut materialization,
        &mut restore,
        &mut admission,
        &mut transitions,
        &mut capture,
        &mut receipts,
    )
    .expect("exact opaque replay");

    assert_eq!(outcome.receipt.decision, WorldReplayReceiptDecision::Replayed);
    assert_eq!(restore.logical_calls, 0);
    assert_eq!(restore.opaque_calls, 1);
}

#[test]
fn opaque_replay_uses_only_the_exact_adapter_and_rejects_fallback() {
    // r[verify molten.world_replay.execution_boundary]
    let fixture = fixture(WorldReplayProfileKind::Opaque);
    let mut materialization = Materialization::default();
    let mut restore = Restore {
        logical_fallback: true,
        ..Restore::default()
    };
    let mut admission = Admission { allowed: true };
    let mut transitions = Transitions::default();
    let mut capture = Capture::from_fixture(&fixture);
    let mut receipts = Receipts::default();
    let error = run(
        &fixture,
        &mut materialization,
        &mut restore,
        &mut admission,
        &mut transitions,
        &mut capture,
        &mut receipts,
    )
    .expect_err("opaque logical fallback denied");

    assert!(error.to_string().contains("logical fallback"));
    assert_eq!(restore.logical_calls, 0);
    assert_eq!(restore.opaque_calls, 1);
    assert!(transitions.positions.is_empty());
}

fn run(
    fixture: &Fixture,
    materialization: &mut Materialization,
    restore: &mut Restore,
    admission: &mut Admission,
    transitions: &mut Transitions,
    capture: &mut Capture,
    receipts: &mut Receipts,
) -> crate::error::Result<WorldReplayRunOutcome> {
    run_world_replay(&fixture.request, &fixture.commits[0], &dependency_refs(), WorldReplayPorts {
        materialization,
        restore,
        admission,
        transitions,
        capture,
        receipts,
    })
}
