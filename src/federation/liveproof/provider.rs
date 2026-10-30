/// Actual provider role. Shared files carry only public endpoint/key/session handoffs.
/// Inventory and every object byte travel over the admitted Iroh cross-process rail.
pub fn run(root: &crate::node_state::DirectoryRoot, exchange: &crate::node_state::Scope) -> crate::error::Result<()> {
    let secrets = root.secrets()?;
    let diagnostics = root.receipts()?;
    let adapter = super::setup::profiles::crypto(&secrets)?;
    let policy = super::setup::transport::reference("federation-live-signature-policy");
    let context = super::setup::transport::reference("federation-live-signature-context");
    let key =
        adapter.resolve_or_generate(crate::fabric_crypto_identity::KeyPurpose::FederationOrigin, &policy, true)?;
    let runtime = tokio::runtime::Runtime::new().map_err(crate::error::Failure::from)?;
    let mut listener =
        runtime.block_on(super::setup::transport::listen("federation-live-provider-listener", &adapter))?;
    super::setup::exchange::publish(exchange, "provider", &listener, &key)?;
    let receiver = super::setup::exchange::handoff(exchange, "receiver")?;
    let role = Role {
        runtime: &runtime,
        adapter: &adapter,
        key: &key,
        receiver: &receiver,
        policy: &policy,
        context: &context,
        diagnostics: &diagnostics,
    };
    let artifact = super::setup::profiles::artifact();
    let bytes = crate::preserves_rail::canonical_bytes(&artifact)?;
    let head = crate::preserves_rail::canonical_hash(&artifact)?;
    let chunks = root.chunk_store()?;
    let stored = crate::chunk_store::put_bytes_with_root(&chunks, "artifact", &bytes, 4096)?;
    announce(&role, &head, bytes.len())?;
    let session = super::setup::exchange::wait(exchange, "receiver-session.preserves")?;
    let session = session
        .as_string()
        .ok_or_else(|| super::invalid("receiver session handoff malformed"))?
        .into_owned();
    let available = std::collections::BTreeMap::from([(head, stored.manifest_ref)]);
    let mut prepared = fetch(&role, &mut listener, &session, &chunks, &available)?;
    super::campaign::transform(root, exchange, &adapter, &mut prepared)?;
    let sent = runtime.block_on(crate::mesh::envelope::network::send_record(
        prepared.client,
        &prepared.signed,
        prepared.maximum_bytes,
        prepared.timeout,
    ))?;
    super::setup::dagports::persist(
        &root.receipts()?,
        "provider-response.preserves",
        &super::setup::transport::evidence(&sent),
    )?;
    runtime.block_on(listener.drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest))?;
    Ok(())
}
/// Provider-owned production material shared by the inventory announcement and the fetch response.
struct Role<'a> {
    runtime: &'a tokio::runtime::Runtime,
    adapter: &'a crate::fabric_crypto_identity::IrohEd25519<'a>,
    key: &'a crate::fabric_crypto_identity::ResolvedProductionKey,
    receiver: &'a super::setup::exchange::Handoff,
    policy: &'a str,
    context: &'a str,
    diagnostics: &'a crate::node_state::Scope,
}
fn announce(role: &Role<'_>, head: &str, length: usize) -> crate::error::Result<()> {
    let inventory = inventory(&role.key.handle.handle, head, length)?;
    let signing = super::setup::exchange::signing(
        role.adapter,
        &role.key.handle.handle,
        molten_core::federation::RecordPurpose::Inventory,
        role.context,
        role.policy,
    );
    let signed = crate::mesh::envelope::signatures::sign_record(
        role.adapter,
        &crate::mesh::envelope::signatures::SigningInput {
            payload: &inventory,
            ..signing
        },
    )?;
    role.runtime.block_on(crate::mesh::envelope::network::send_record(
        super::setup::transport::client(
            role.receiver.endpoint.clone(),
            &super::setup::transport::reference("federation-live-inventory-session"),
            role.adapter,
        )?,
        &signed,
        super::setup::transport::LIMIT,
        super::setup::transport::TIMEOUT,
    ))?;
    Ok(())
}
fn fetch(
    role: &Role<'_>,
    listener: &mut crate::fabric_transport::cross_process::Listener,
    session: &str,
    chunks: &crate::chunk_store::CapabilityChunkRoot,
    available: &std::collections::BTreeMap<String, String>,
) -> crate::error::Result<crate::mesh::transfer::provider::PreparedResponse> {
    let content = super::setup::profiles::content()?;
    role.runtime.block_on(crate::mesh::transfer::provider::prepare_fetch(
        listener,
        role.adapter,
        crate::mesh::transfer::provider::FetchInput {
            receive: crate::mesh::envelope::network::ReceiveInput {
                session_ref: session,
                verification: super::setup::exchange::verification(
                    role.adapter,
                    role.receiver,
                    molten_core::federation::RecordPurpose::Request,
                    role.context,
                    role.policy,
                ),
                maximum_bytes: super::setup::transport::LIMIT,
                timeout: super::setup::transport::TIMEOUT,
                diagnostics: role.diagnostics,
            },
            response_client: super::setup::transport::client(role.receiver.endpoint.clone(), session, role.adapter)?,
            signing: super::setup::exchange::signing(
                role.adapter,
                &role.key.handle.handle,
                molten_core::federation::RecordPurpose::Response,
                role.context,
                role.policy,
            ),
            root: chunks,
            content_profile: &content,
            available,
            maximum_objects: 1,
            maximum_bytes: super::setup::transport::LIMIT,
            service_generation: 1,
        },
    ))
}
fn inventory(
    handle: &crate::fabric_crypto_identity::OpaqueKeyHandle,
    head: &str,
    length: usize,
) -> crate::error::Result<preserves::IOValue> {
    crate::mesh::envelope::records::inventory_payload(&molten_core::federation::InventoryObservation {
        peer_ref: super::setup::transport::reference("federation-live-provider"),
        signer_ref: handle.public_key_ref.clone(),
        generation: 1,
        issued_at: super::setup::transport::NOW,
        expires_at: super::setup::transport::UNTIL,
        encoded_bytes: 0,
        entries: vec![molten_core::federation::InventoryEntry {
            name: "signed-live-artifact".to_string(),
            head_ref: head.to_string(),
            ancestry_ref: super::setup::transport::reference("federation-live-genesis"),
            policy_ref: super::setup::transport::reference("federation-live-export-policy"),
            bytes: u64::try_from(length).map_err(|_| super::invalid("artifact length overflow"))?,
            dag_nodes: 1,
            chunks: 1,
        }],
    })
}
