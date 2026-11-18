use super::*;

#[derive(Debug, Default)]
pub(super) struct Progress {
    pub(super) sent_bytes: u64,
    pub(super) received_bytes: u64,
    pub(super) sent_frames: u64,
    pub(super) received_frames: u64,
    pub(super) local_consumed_bytes: u64,
    pub(super) is_open_started: bool,
    pub(super) submitted: bool,
    pub(super) acknowledged_bytes: u64,
    pub(super) deny_code: Option<&'static str>,
    pub(super) authority_ref: Option<String>,
}

pub(super) struct Input<'a> {
    pub(super) descriptor_ref: &'a str,
    pub(super) request: &'a DirectStreamRequest,
    pub(super) peer: &'a DirectPeerBinding,
    pub(super) actual_peer: Option<&'a str>,
    pub(super) contract_ref: &'a str,
    pub(super) progress: &'a Progress,
    pub(super) terminal: DirectStreamTerminal,
    pub(super) path: DirectStreamPath,
    pub(super) duration_ms: u64,
    pub(super) is_reset: bool,
    pub(super) is_cleanup: bool,
}

/// Opening the protected service may already have had an effect, so it is
/// uncertain like consumption, submission or a protected write.
fn classify_delivery(terminal: DirectStreamTerminal, progress: &Progress) -> DeliveryOutcome {
    if terminal == DirectStreamTerminal::Clean {
        DeliveryOutcome::Delivered
    } else if progress.is_open_started
        || progress.submitted
        || progress.local_consumed_bytes != 0
        || progress.received_bytes != 0
    {
        DeliveryOutcome::Uncertain
    } else {
        DeliveryOutcome::NotDelivered
    }
}

pub(super) fn build(input: Input<'_>) -> crate::fabric::FabricPortResult<Evidence> {
    let progress = input.progress;
    let delivery = classify_delivery(input.terminal, progress);
    let value = crate::preserves_rail::record("fabric-direct-stream-evidence-v1", vec![
        crate::preserves_rail::string("molten.fabric.transport.direct-stream-evidence.v1"),
        crate::preserves_rail::string(input.descriptor_ref),
        crate::preserves_rail::string(&input.request.request_ref),
        crate::preserves_rail::string(&input.request.operation_ref),
        crate::preserves_rail::string(&input.request.resource_ref),
        crate::preserves_rail::string(input.contract_ref),
        crate::preserves_rail::string(input.actual_peer.unwrap_or("not-authenticated")),
        crate::preserves_rail::string(&input.peer.peer_context_ref),
        crate::preserves_rail::string(&input.peer.service_id),
        crate::preserves_rail::string(&input.peer.alpn),
        crate::preserves_rail::u64_value(input.peer.generation),
        crate::preserves_rail::string(input.peer.remote_role.as_str()),
        crate::preserves_rail::string(delivery.as_str()),
        crate::preserves_rail::string(input.terminal.as_str()),
        crate::preserves_rail::string(input.path.as_str()),
        crate::preserves_rail::u64_value(progress.sent_bytes),
        crate::preserves_rail::u64_value(progress.received_bytes),
        crate::preserves_rail::u64_value(progress.sent_frames),
        crate::preserves_rail::u64_value(progress.received_frames),
        crate::preserves_rail::u64_value(progress.local_consumed_bytes),
        crate::preserves_rail::u64_value(progress.acknowledged_bytes),
        crate::preserves_rail::u64_value(input.duration_ms),
        crate::preserves_rail::u64_value(0),
        crate::preserves_rail::string(if input.is_reset {
            "reset-observed"
        } else {
            "no-reset-observed"
        }),
        crate::preserves_rail::string(if input.is_cleanup {
            "cleanup-observed"
        } else {
            "cleanup-unobserved"
        }),
        crate::preserves_rail::string(progress.deny_code.unwrap_or("no-receive-denial")),
        crate::preserves_rail::string(progress.authority_ref.as_deref().unwrap_or("no-operation-authority")),
        crate::preserves_rail::string("no-relay-or-discovery-fallback"),
        crate::preserves_rail::string("not-two-host-qualification"),
        crate::preserves_rail::string("not-kiln-or-vm-migration-qualification"),
        crate::preserves_rail::string("transport-ack-not-durable-publication-or-resume-authority"),
    ]);
    let evidence_ref = crate::preserves_rail::canonical_hash(&value).map_err(port_error)?;
    Ok(Evidence {
        delivery,
        terminal: input.terminal,
        sent_bytes: progress.sent_bytes,
        received_bytes: progress.received_bytes,
        sent_frames: progress.sent_frames,
        received_frames: progress.received_frames,
        local_consumed_bytes: progress.local_consumed_bytes,
        automatic_retry_count: 0,
        reset_observed: input.is_reset,
        cleanup_observed: input.is_cleanup,
        actual_path: input.path,
        duration_ms: input.duration_ms,
        evidence_ref,
        value,
    })
}

pub(super) fn contract_ref(
    profile: &Profile,
    protocol: &ProtocolDescriptor,
    bounds: &DirectStreamBounds,
) -> crate::fabric::FabricPortResult<String> {
    let value = crate::preserves_rail::record("fabric-direct-stream-contract-v1", vec![
        crate::preserves_rail::string(&profile.profile_ref),
        crate::preserves_rail::string(&protocol.protocol_id),
        crate::preserves_rail::string(&protocol.version),
        crate::preserves_rail::string(&protocol.alpn),
        crate::preserves_rail::string(&protocol.service_id),
        crate::preserves_rail::u64_value(protocol.generation),
        crate::preserves_rail::u64_value(bounds.max_connections),
        crate::preserves_rail::u64_value(bounds.max_frame_bytes),
        crate::preserves_rail::u64_value(bounds.max_total_bytes),
        crate::preserves_rail::u64_value(bounds.max_frames),
        crate::preserves_rail::u64_value(bounds.max_buffered_bytes),
        crate::preserves_rail::u64_value(bounds.idle_timeout_ms),
        crate::preserves_rail::u64_value(bounds.total_timeout_ms),
        crate::preserves_rail::u64_value(bounds.cleanup_timeout_ms),
    ]);
    crate::preserves_rail::canonical_hash(&value).map_err(port_error)
}

pub(super) fn operation_binding(
    request: &DirectStreamRequest,
    contract: &str,
) -> crate::fabric::FabricPortResult<[u8; 32]> {
    let value = crate::preserves_rail::record("fabric-direct-stream-operation-v1", vec![
        crate::preserves_rail::string(&request.request_ref),
        crate::preserves_rail::string(&request.operation_ref),
        crate::preserves_rail::string(&request.resource_ref),
        crate::preserves_rail::u64_value(request.generation),
        crate::preserves_rail::string(contract),
    ]);
    let reference = crate::preserves_rail::canonical_hash(&value).map_err(port_error)?;
    let digest = reference
        .strip_prefix("blake3:")
        .ok_or_else(|| crate::fabric::FabricPortError::malformed("operation identity prefix"))?;
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).map_err(port_error)?;
    }
    Ok(bytes)
}
