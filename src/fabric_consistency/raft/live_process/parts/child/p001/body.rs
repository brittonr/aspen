
pub(super) async fn poll_event(
    ingress: &mut IrohReplicaIngressPump,
) -> crate::error::Result<Option<ReceivedReplicaEvent>> {
    tokio::select! {
        result = ingress.next() => result.map(Some),
        () = tokio::time::sleep(std::time::Duration::from_millis(EVENT_POLL_MILLISECONDS)) => Ok(None),
    }
}

fn has_read_outcome(outcome: &ReplicaExecutionOutcome) -> bool {
    matches!(outcome, ReplicaExecutionOutcome::Applied(applied)
        if applied.observations.iter().any(|observation| observation.kind == ReplicaEffectKind::ReadOutcome))
}

pub(super) fn require_applied(outcome: ReplicaExecutionOutcome) -> crate::error::Result<()> {
    match outcome {
        ReplicaExecutionOutcome::Applied(_) => Ok(()),
        ReplicaExecutionOutcome::Denied { diagnostic, .. } => {
            Err(crate::error::MoltenError::invalid_harness(format!("distinct-process turn denied: {diagnostic}")))
        }
        ReplicaExecutionOutcome::Failed(failed) => Err(crate::error::MoltenError::invalid_harness(format!(
            "distinct-process effect {} failed: {}",
            failed.failed_kind.as_str(),
            failed.diagnostic
        ))),
    }
}
