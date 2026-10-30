pub struct FetchInput<'a> {
    pub receive: crate::mesh::envelope::network::ReceiveInput<'a>,
    pub response_client: crate::fabric_transport::cross_process::ClientInput,
    pub signing: crate::mesh::envelope::signatures::SigningInput<'a>,
    pub service_generation: u64,
    pub root: &'a crate::chunk_store::CapabilityChunkRoot,
    pub content_profile: &'a molten_core::content_store_adapter::ContentAdapterProfile,
    pub available: &'a std::collections::BTreeMap<String, String>,
    pub maximum_objects: usize,
    pub maximum_bytes: u64,
}

pub struct PreparedResponse {
    pub signed: crate::mesh::envelope::signatures::SignedRecord,
    pub client: crate::fabric_transport::cross_process::ClientInput,
    pub maximum_bytes: u64,
    pub timeout: std::time::Duration,
}

pub async fn serve_fetch(
    listener: &mut crate::fabric_transport::cross_process::Listener,
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    input: FetchInput<'_>,
) -> crate::error::Result<crate::fabric_transport::cross_process::FrameEvidence> {
    let response = prepare_fetch(listener, adapter, input).await?;
    crate::mesh::envelope::network::send_record(
        response.client,
        &response.signed,
        response.maximum_bytes,
        response.timeout,
    )
    .await
}

// r[impl molten.federated_pull_runtime.receiver_pull]
pub async fn prepare_fetch(
    listener: &mut crate::fabric_transport::cross_process::Listener,
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    input: FetchInput<'_>,
) -> crate::error::Result<PreparedResponse> {
    validate_limits(&input)?;
    let maximum = input.receive.maximum_bytes;
    let timeout = input.receive.timeout;
    let session_ref = input.receive.session_ref;
    let request = crate::mesh::envelope::network::receive_record(listener, adapter, input.receive).await?;
    let object_ref = requested_object(&request, session_ref)?;
    crate::preserves_rail::validate_content_ref(&object_ref)?;
    let manifest_ref = input
        .available
        .get(object_ref.as_str())
        .ok_or_else(|| invalid("requested object not admitted for export"))?;
    let manifest = crate::chunk_store::read_manifest_with_root(input.root, manifest_ref)?;
    if manifest.total_len > input.maximum_bytes {
        return Err(invalid("provider byte budget exceeded"));
    }
    let descriptor = crate::content_store_adapter::manifest_descriptor(&manifest);
    let command =
        crate::content_store_adapter::command(input.content_profile, crate::content_store_adapter::CommandInput {
            operation_ref: request.transport().request_ref.clone(),
            operation: molten_core::content_store_adapter::ContentOperation::Export,
            manifest: &descriptor,
            range: None,
            submitted_tick: input.signing.issued_at,
            deadline_tick: input.signing.expires_at,
            retry_count: 0,
            cancelled: false,
            policy_refs: vec![input.signing.policy_ref.to_string()],
        })?;
    let exported = crate::content_store_adapter::execute_stream_get(
        input.content_profile,
        input.root,
        &command,
        input.service_generation,
        None,
    )?;
    let bytes = crate::content_store_adapter::assemble_verified(
        &exported.manifest,
        &exported.state.artifact,
        &exported.verified_chunks,
    )?;
    if crate::preserves_rail::content_ref_from_bytes(&bytes) != object_ref.as_str() {
        return Err(invalid("provider exported object identity mismatch"));
    }
    let payload = crate::preserves_rail::record("federation-object-v1", vec![
        crate::preserves_rail::string(&request.transport().request_ref),
        crate::preserves_rail::string(object_ref.as_str()),
        manifest.value,
        crate::preserves_rail::sequence(
            bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect(),
        ),
    ]);
    let signed =
        crate::mesh::envelope::signatures::sign_record(adapter, &crate::mesh::envelope::signatures::SigningInput {
            payload: &payload,
            ..input.signing
        })?;
    Ok(PreparedResponse {
        signed,
        client: input.response_client,
        maximum_bytes: maximum,
        timeout,
    })
}

fn validate_limits(input: &FetchInput<'_>) -> crate::error::Result<()> {
    if input.available.len() > input.maximum_objects
        || input.maximum_objects > molten_core::federation::MAX_FEDERATION_ENTRIES
        || input.receive.verification.purpose != molten_core::federation::RecordPurpose::Request
        || input.signing.purpose != molten_core::federation::RecordPurpose::Response
    {
        return Err(invalid("provider limits or signed request/response domain denied"));
    }
    Ok(())
}

/// Validates the signed fetch request shape and returns the requested object reference.
fn requested_object(
    request: &crate::mesh::envelope::network::VerifiedRecord,
    session_ref: &str,
) -> crate::error::Result<String> {
    let fields = request
        .record()
        .payload
        .collect_simple_record("federation-fetch-v1", Some(4))
        .ok_or_else(|| invalid("malformed federation fetch request"))?;
    if fields[0].as_string().as_deref() != Some(session_ref) {
        return Err(invalid("federation fetch session mismatch"));
    }
    if !matches!(fields[2].as_string().as_deref(), Some("node" | "content")) {
        return Err(invalid("federation fetch object kind is malformed"));
    }
    let sequence = fields[3].as_string().ok_or_else(|| invalid("missing fetch sequence"))?;
    let sequence_value = sequence.parse::<usize>().map_err(|_| invalid("malformed fetch sequence"))?;
    if sequence_value >= molten_core::dag_sync::MAX_DAG_STEPS || sequence_value.to_string() != sequence.as_ref() {
        return Err(invalid("fetch sequence is outside canonical protocol bounds"));
    }
    let object_ref = fields[1].as_string().ok_or_else(|| invalid("missing fetch object ref"))?;
    Ok(object_ref.to_string())
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
