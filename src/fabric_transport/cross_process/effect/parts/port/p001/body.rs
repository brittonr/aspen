
fn run_effect_exchange(input: ExchangeInput<'_>) -> crate::error::Result<CrossProcessFrameEvidence> {
    let ExchangeInput {
        profile,
        protocol,
        client,
        session_id,
        request_ref,
        payload,
    } = input;
    let input = IrohCrossProcessClientInput {
        profile: profile.clone(),
        protocol: protocol.clone(),
        capability: client.capability.clone(),
        bind_addr: client.bind_addr,
        expected: client.expected.clone(),
        endpoint: client.endpoint.clone(),
        admission: client.admission,
        session_ref: session_id.opaque_ref.clone(),
        request_ref: request_ref.to_string(),
    };
    let payload = payload.to_vec();
    let timeout = client.timeout;
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("cross-process effect runtime creation failed: {error}"))
        })?;
        runtime.block_on(exchange_cross_process_frame(input, &payload, timeout))
    })
    .join()
    .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process effect worker panicked"))?
}

fn matching_exchange_evidence(
    evidence: &CrossProcessFrameEvidence,
    request_ref: &str,
    payload_ref: &str,
    payload_bytes: u64,
) -> bool {
    evidence.role == EndpointParticipantRole::Client
        && evidence.request_ref == request_ref
        && evidence.payload_ref == payload_ref
        && evidence.acknowledgement_ref == payload_ref
        && evidence.payload_bytes == payload_bytes
        && evidence.delivery == DeliveryOutcome::Delivered
        && evidence.retry == RetryDisposition::NotApplicable
        && evidence.automatic_retry_count == 0
}

fn fail_canonical_session(
    adapter: &mut IrohTransportAdapter,
    operation_id: &str,
    session_id: &ScopedTransportId,
    class: TransportFailureClass,
) -> std::result::Result<CanonicalTransportTransition, String> {
    adapter
        .execute_command(&TransportCommand::FailSession {
            operation_id: operation_id.to_string(),
            session_id: session_id.clone(),
            class,
            delivery_definitive: false,
        })
        .map_err(|error| error.to_string())
}

fn effect_output(
    effect: &crate::system_extension::TypedEffectRequest,
    output_ref: String,
) -> crate::system_extension::PortEffectOutput {
    crate::system_extension::PortEffectOutput {
        output_schema_ref: effect.output_schema_ref.clone(),
        output_ref,
        materialized_output: None,
    }
}
