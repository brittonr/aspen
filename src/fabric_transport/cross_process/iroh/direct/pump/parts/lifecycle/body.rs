pub(super) async fn cancellation(receiver: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *receiver.borrow() {
            return;
        }
        if receiver.changed().await.is_err() {
            // Losing the owner is cancellation, not an unbounded orphan task.
            return;
        }
    }
}

pub(super) fn elapsed_ms(start: std::time::Instant, now: std::time::Instant) -> crate::fabric::FabricPortResult<u64> {
    let elapsed = now
        .checked_duration_since(start)
        .ok_or_else(|| crate::fabric::FabricPortError::malformed("direct monotonic clock moved backwards"))?;
    u64::try_from(elapsed.as_millis())
        .map_err(|_| crate::fabric::FabricPortError::malformed("direct monotonic duration exceeds wire range"))
}

pub(super) fn apply_session(
    session: &mut CrossProcessSessionState,
    command: CrossProcessSessionCommand,
) -> Result<(), DirectStreamTerminal> {
    *session = apply_cross_process_session_command(session, &command)
        .map_err(|_| DirectStreamTerminal::Malformed)?
        .next;
    Ok(())
}