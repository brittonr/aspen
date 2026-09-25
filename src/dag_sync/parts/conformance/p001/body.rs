
fn scoped_id(opaque_ref: &str) -> ScopedTransportId {
    ScopedTransportId {
        opaque_ref: opaque_ref.to_string(),
        service_id: DAG_TRANSPORT_SERVICE.to_string(),
        generation: DAG_TRANSPORT_GENERATION,
    }
}

fn send_command(
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    request: &DagFetchRequest,
    payload: &[u8],
) -> Result<TransportCommand> {
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| MoltenError::invalid_harness("DAG transport payload length exceeds u64"))?;
    Ok(TransportCommand::SendFrame {
        operation_id: DAG_TRANSPORT_OPERATION_REF.to_string(),
        session_id: session_id.clone(),
        stream_id: stream_id.clone(),
        payload_ref: format!("blake3:{}", blake3::hash(payload).to_hex()),
        payload_bytes,
        observed_tick: observed_tick(request.sequence)?,
    })
}

fn observed_tick(sequence: usize) -> Result<u64> {
    let sequence =
        u64::try_from(sequence).map_err(|_| MoltenError::invalid_harness("DAG request sequence exceeds u64"))?;
    Ok(DAG_TRANSPORT_INITIAL_TICK.saturating_add(sequence))
}

fn dag_request_payload(request: &DagFetchRequest) -> Vec<u8> {
    format!(
        "{}:{}:{}:{}",
        request.object_ref.kind(),
        request.object_ref.as_str(),
        request.assigned_peer.as_ref().map_or("unassigned", DagPeerId::as_str),
        request.sequence
    )
    .into_bytes()
}
