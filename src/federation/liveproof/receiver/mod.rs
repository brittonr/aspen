mod pull;

pub fn run(
    root: &crate::node_state::DirectoryRoot,
    exchange: &crate::node_state::Scope,
) -> crate::error::Result<super::receipt::VerifiedBundle> {
    let secrets = root.secrets()?;
    let adapter = super::setup::profiles::crypto(&secrets)?;
    let policy = super::setup::transport::reference("federation-live-signature-policy");
    let key =
        adapter.resolve_or_generate(crate::fabric_crypto_identity::KeyPurpose::FederationOrigin, &policy, true)?;
    let runtime = tokio::runtime::Runtime::new().map_err(crate::error::Failure::from)?;
    let mut listener =
        runtime.block_on(super::setup::transport::listen("federation-live-receiver-listener", &adapter))?;
    super::setup::exchange::publish(exchange, "receiver", &listener, &key)?;
    let peer = super::setup::exchange::handoff(exchange, "provider")?;
    let context = super::setup::exchange::context(
        &peer,
        &key.public_key,
        &key.handle.handle.public_key_ref,
        key.handle.handle.generation,
    );
    let artifact_ref = crate::preserves_rail::canonical_hash(&super::setup::profiles::artifact())?;
    let local_policy = super::setup::profiles::local_policy(&artifact_ref, &peer.signer_ref)?;
    let proofset = super::setup::profiles::proofset(&artifact_ref, &key.handle.handle.public_key_ref)?;
    let manifest = admit(exchange, &local_policy, &proofset, &peer.signer_ref, adapter.profile())?;
    let mut host = crate::mesh::lifecycle::extension::create_host(&manifest)?;
    host.activate(super::setup::transport::NOW)?;
    let durable = root.receipts()?;
    let mut service = crate::mesh::lifecycle::Service::start(manifest, host, &durable)?;
    let verification = super::setup::exchange::verification(
        &adapter,
        &peer,
        molten_core::federation::RecordPurpose::Inventory,
        &context.context_ref,
        &context.policy_ref,
    );
    let inventory = receive(&runtime, &mut listener, &adapter, verification, &durable)?;
    let registry = root.ledger_store()?;
    if crate::ledger::read_artifact_with_root(&registry, &artifact_ref).is_ok() {
        return Err(super::invalid("unsolicited inventory imported an artifact or receiver was not fresh"));
    }
    let session = service.plan_inventory(&inventory, &[], super::setup::transport::NOW, &durable)?;
    replay(&service, &inventory, &durable, &session)?;
    super::setup::exchange::publish_value(
        exchange,
        "receiver-session.preserves",
        &crate::preserves_rail::string(&session.session_ref),
    )?;
    let input = Pull {
        root,
        runtime: &runtime,
        adapter: &adapter,
        key: &key,
        peer: &peer,
        context: &context,
        session: &session,
        policy: &local_policy,
        proofset: &proofset,
        inventory: &inventory,
    };
    let bytes = pull::execute(&mut service, &mut listener, input)?;
    let verified = super::receipt::verify(&adapter, &context, &bytes)?;
    durable.write(&crate::node_state::RelativePath::parse("detached-bundle.preserves")?, &bytes)?;
    super::setup::dagports::persist(&durable, "public-context.preserves", &context.value(adapter.profile()))?;
    service.stop(super::setup::transport::NOW)?;
    super::setup::dagports::persist(&durable, "supervision-stopped.preserves", &service.host.operator_status()?.value)?;
    runtime.block_on(listener.drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest))?;
    Ok(verified)
}
struct Pull<'a> {
    root: &'a crate::node_state::DirectoryRoot,
    runtime: &'a tokio::runtime::Runtime,
    adapter: &'a crate::fabric_crypto_identity::IrohEd25519<'a>,
    key: &'a crate::fabric_crypto_identity::ResolvedProductionKey,
    peer: &'a super::setup::exchange::Handoff,
    context: &'a super::receipt::PublicContext,
    session: &'a molten_core::federation::Session,
    policy: &'a crate::mesh::admission::rules::LocalPolicy,
    proofset: &'a crate::capability_tokens::CapabilityProofset,
    inventory: &'a crate::mesh::envelope::network::VerifiedRecord,
}
fn admit(
    exchange: &crate::node_state::Scope,
    policy: &crate::mesh::admission::rules::LocalPolicy,
    proofset: &crate::capability_tokens::CapabilityProofset,
    signer: &str,
    crypto: &crate::fabric_crypto_identity::Profile,
) -> crate::error::Result<crate::mesh::admission::Manifest> {
    let conformance = crate::preserves_rail::parse_canonical_bytes(
        &exchange.read(&crate::node_state::RelativePath::parse("dag-conformance.preserves")?, 256)?,
    )?;
    let conformance = conformance
        .as_string()
        .ok_or_else(|| super::invalid("missing reviewed DAG conformance selection"))?;
    super::setup::admission::manifest(policy, proofset, signer, crypto, conformance.as_ref())
}
fn receive(
    runtime: &tokio::runtime::Runtime,
    listener: &mut crate::fabric_transport::cross_process::Listener,
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    verification: crate::mesh::envelope::signatures::VerificationInput<'_>,
    durable: &crate::node_state::Scope,
) -> crate::error::Result<crate::mesh::envelope::network::VerifiedRecord> {
    runtime.block_on(crate::mesh::envelope::network::receive_record(
        listener,
        adapter,
        crate::mesh::envelope::network::ReceiveInput {
            session_ref: &super::setup::transport::reference("federation-live-inventory-session"),
            verification,
            maximum_bytes: super::setup::transport::LIMIT,
            timeout: super::setup::transport::TIMEOUT,
            diagnostics: durable,
        },
    ))
}
pub(super) fn effect(
    request: &molten_core::dag_sync::DagSyncRequest,
) -> crate::error::Result<crate::system_extension::TypedEffectRequest> {
    let canonical = crate::dag_sync::canonical_dag_request(request)?;
    Ok(crate::system_extension::TypedEffectRequest {
        target: crate::system_extension::EffectTarget::FabricPort(crate::fabric::FabricPortKey {
            port_id: crate::dag_sync::DAG_SYNC_PORT_ID.to_string(),
            version: crate::dag_sync::DAG_SYNC_PORT_VERSION.to_string(),
        }),
        operation: crate::dag_sync::DAG_SYNC_OPERATION.to_string(),
        input_schema_ref: crate::dag_sync::dag_sync_request_schema_ref()?,
        output_schema_ref: crate::dag_sync::dag_sync_receipt_schema_ref()?,
        request_ref: canonical.record_ref,
        generation: 1,
        incarnation: 1,
        accounted_bytes: u64::try_from(canonical.bytes.len()).map_err(|_| super::invalid("DAG bytes overflow"))?,
    })
}
fn replay(
    service: &crate::mesh::lifecycle::Service,
    record: &crate::mesh::envelope::network::VerifiedRecord,
    durable: &crate::node_state::Scope,
    session: &molten_core::federation::Session,
) -> crate::error::Result<()> {
    let run = || {
        let mut state = crate::mesh::simulation::Replay {
            bounds: service.manifest.bounds(),
            manifest_ref: service.manifest.manifest_ref(),
            peers: service.manifest.peers(),
            local: &[],
            generation: 1,
            session: None,
            last_query: None,
            resource_history: Vec::new(),
        };
        crate::mesh::simulation::replay(&mut state, vec![
            crate::mesh::simulation::Input::Inventory {
                record: Box::new(record.clone()),
                now: super::setup::transport::NOW,
                requested: false,
            },
            crate::mesh::simulation::Input::Inventory {
                record: Box::new(record.clone()),
                now: super::setup::transport::NOW,
                requested: true,
            },
            crate::mesh::simulation::Input::Transition {
                event: molten_core::federation::SessionEvent::BeginFetch,
                generation: 1,
                now: super::setup::transport::NOW,
            },
            crate::mesh::simulation::Input::Resume,
        ])
    };
    let first = run()?;
    if first != run()? {
        return Err(super::invalid("recorded verified input replay drifted"));
    }
    let state = crate::mesh::simulation::Replay {
        bounds: service.manifest.bounds(),
        manifest_ref: service.manifest.manifest_ref(),
        peers: service.manifest.peers(),
        local: &[],
        generation: 1,
        session: None,
        last_query: None,
        resource_history: Vec::new(),
    };
    let installed = crate::mesh::simulationworld::run(crate::mesh::simulationworld::CampaignInput {
        manifest: &service.manifest,
        replay: &state,
        inventory: record,
        live_session: session,
        now: super::setup::transport::NOW,
    })?;
    super::setup::dagports::persist(durable, "installed-simulation.preserves", &installed)?;
    super::setup::dagports::persist(durable, "deterministic-replay.preserves", &first).map(|_| ())
}
