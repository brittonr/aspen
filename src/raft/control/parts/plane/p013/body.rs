
fn duplicate_sequence(runtime: &ControlRegistryRuntime, envelope: &RaftCommandEnvelope) -> Option<DuplicateSequence> {
    if let Some(session) = runtime
        .state
        .client_sessions
        .iter()
        .find(|session| session.client_session == envelope.client_session && session.sequence == envelope.sequence)
    {
        if session.result_command_ref != envelope.envelope_ref {
            return Some(DuplicateSequence::Conflict(session.clone()));
        }
        return runtime
            .registry_receipts
            .iter()
            .find(|receipt| receipt.command_ref == envelope.envelope_ref)
            .cloned()
            .map(DuplicateSequence::Replay)
            .or_else(|| Some(DuplicateSequence::Conflict(session.clone())));
    }
    runtime
        .state
        .client_sessions
        .iter()
        .filter(|session| session.client_session == envelope.client_session && envelope.sequence <= session.sequence)
        .max_by_key(|session| session.sequence)
        .cloned()
        .map(DuplicateSequence::Conflict)
}
