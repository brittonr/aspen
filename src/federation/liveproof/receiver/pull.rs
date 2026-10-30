/// Receiver-owned evidence cells shared by the network, quarantine and admission ports.
#[derive(Default)]
struct Cells {
    staged: std::cell::RefCell<Option<crate::mesh::transfer::dagnetwork::StagedObject>>,
    completed: std::cell::RefCell<Vec<crate::mesh::transfer::quarantine::QuarantinedObject>>,
    requests: std::cell::RefCell<Vec<crate::mesh::envelope::signatures::SignedRecord>>,
    request_transport: std::cell::RefCell<Vec<crate::fabric_transport::cross_process::FrameEvidence>>,
}
/// Receiver-local capabilities and the expected artifact head selected for one pull.
struct Stage<'a> {
    cells: &'a Cells,
    quarantine: &'a crate::chunk_store::CapabilityChunkRoot,
    registry: &'a crate::ledger::CapabilityLedgerRoot,
    durable: &'a crate::node_state::Scope,
    content_profile: &'a molten_core::content_store_adapter::ContentAdapterProfile,
    head: &'a str,
}
struct Ports<'a> {
    transport: crate::mesh::transfer::dagnetwork::NetworkDagPort<'a>,
    content: crate::mesh::transfer::quarantine::VerificationPort<'a>,
    authority: super::super::setup::dagports::Authority<'a>,
    resources: super::super::setup::dagports::Resources<'a>,
    progress: super::super::setup::dagports::Progress<'a>,
    observations: super::super::setup::dagports::Observations<'a>,
    receipts: super::super::setup::dagports::Receipts<'a>,
}
pub(super) fn execute(
    service: &mut crate::mesh::lifecycle::Service,
    listener: &mut crate::fabric_transport::cross_process::Listener,
    input: super::Pull<'_>,
) -> crate::error::Result<Vec<u8>> {
    let cells = Cells::default();
    let quarantine = input.root.chunk_store()?;
    let registry = input.root.ledger_store()?;
    let durable = input.root.receipts()?;
    let content_profile = super::super::setup::profiles::content()?;
    let mut policy = crate::mesh::admission::rules::PolicyPort::admit(crate::mesh::admission::rules::PolicyInput {
        manifest: &service.manifest,
        policy: input.policy,
        proofset: input.proofset,
        origins: &cells.completed,
    })?;
    let schema = crate::preserves_rail::boundary_schema_ref(&super::super::setup::profiles::ARTIFACT_SCHEMA)?;
    let head = crate::preserves_rail::canonical_hash(&super::super::setup::profiles::artifact())?;
    let (graph, request) = super::super::setup::dagports::request(
        &head,
        &input.session.session_ref,
        schema.as_str(),
        input.session.plan.bytes,
    )?;
    let effect = super::effect(&request)?;
    let stage = Stage {
        cells: &cells,
        quarantine: &quarantine,
        registry: &registry,
        durable: &durable,
        content_profile: &content_profile,
        head: &head,
    };
    let mut ports = ports(&input, listener, &stage)?;
    let outcome = crate::mesh::transfer::execution::execute_pull(
        service,
        crate::mesh::transfer::execution::SessionPullInput {
            session_ref: &input.session.session_ref,
            now: super::super::setup::transport::NOW,
            graph: &graph,
            request,
            effect,
            quarantined: &cells.completed,
        },
        crate::dag_sync::DagSyncPorts {
            authority: &mut ports.authority,
            resources: &mut ports.resources,
            transport: &mut ports.transport,
            content: &mut ports.content,
            progress: &mut ports.progress,
            observations: &mut ports.observations,
            receipts: &mut ports.receipts,
        },
        crate::mesh::transfer::import::Ports {
            admission: &mut policy,
            quarantine: &quarantine,
            registry: &registry,
            durable: &durable,
        },
    )?;
    conclude(service, &input, &stage, outcome, &mut policy)
}
fn ports<'a>(
    input: &'a super::Pull<'_>,
    listener: &'a mut crate::fabric_transport::cross_process::Listener,
    stage: &'a Stage<'_>,
) -> crate::error::Result<Ports<'a>> {
    let transport = transport(input, listener, stage)?;
    let content = crate::mesh::transfer::quarantine::VerificationPort {
        staged: &stage.cells.staged,
        root: stage.quarantine,
        profile: stage.content_profile,
        policy_ref: &input.context.policy_ref,
        now: super::super::setup::transport::NOW,
        deadline: super::super::setup::transport::UNTIL,
        maximum_bytes: super::super::setup::transport::LIMIT,
        completed: &stage.cells.completed,
        maximum_objects: 1,
        diagnostics: stage.durable,
    };
    Ok(Ports {
        transport,
        content,
        authority: super::super::setup::dagports::Authority {
            proofset: input.proofset,
            head: stage.head,
        },
        resources: super::super::setup::dagports::Resources {
            scope: stage.durable,
            remaining: 1,
        },
        progress: super::super::setup::dagports::Progress { scope: stage.durable },
        observations: super::super::setup::dagports::Observations { scope: stage.durable },
        receipts: super::super::setup::dagports::Receipts { scope: stage.durable },
    })
}
fn transport<'a>(
    input: &'a super::Pull<'_>,
    listener: &'a mut crate::fabric_transport::cross_process::Listener,
    stage: &'a Stage<'_>,
) -> crate::error::Result<crate::mesh::transfer::dagnetwork::NetworkDagPort<'a>> {
    Ok(crate::mesh::transfer::dagnetwork::NetworkDagPort {
        runtime: input.runtime,
        client: super::super::setup::transport::client(
            input.peer.endpoint.clone(),
            &input.session.session_ref,
            input.adapter,
        )?,
        listener,
        adapter: input.adapter,
        signing: super::super::setup::exchange::signing(
            input.adapter,
            &input.key.handle.handle,
            molten_core::federation::RecordPurpose::Request,
            &input.context.context_ref,
            &input.context.policy_ref,
        ),
        verification: super::super::setup::exchange::verification(
            input.adapter,
            input.peer,
            molten_core::federation::RecordPurpose::Response,
            &input.context.context_ref,
            &input.context.policy_ref,
        ),
        session_ref: &input.session.session_ref,
        peer: molten_core::dag_sync::DagPeerId::new("federation-live-provider")
            .map_err(|_| super::super::invalid("invalid peer"))?,
        maximum_bytes: super::super::setup::transport::LIMIT,
        remaining_bytes: super::super::setup::transport::LIMIT,
        timeout: super::super::setup::transport::TIMEOUT,
        deadline: crate::fabric_time::SupervisionDeadline::after(super::super::setup::transport::TIMEOUT)?,
        staged: &stage.cells.staged,
        requests: &stage.cells.requests,
        request_transport: &stage.cells.request_transport,
        diagnostics: stage.durable,
    })
}
fn conclude(
    service: &crate::mesh::lifecycle::Service,
    input: &super::Pull<'_>,
    stage: &Stage<'_>,
    outcome: crate::mesh::transfer::execution::PullOutcome,
    policy: &mut crate::mesh::admission::rules::PolicyPort<'_>,
) -> crate::error::Result<Vec<u8>> {
    if outcome.session.phase != molten_core::federation::SessionPhase::Committed || outcome.imports.len() != 1 {
        return Err(super::super::invalid("live receiver did not commit exactly one admitted import"));
    }
    let readback = crate::ledger::read_artifact_with_root(stage.registry, stage.head)?;
    super::super::terminal::compare(
        &service.manifest,
        input.inventory,
        &outcome,
        policy.evidence_values(),
        stage.durable,
    )?;
    let objects = stage.cells.completed.borrow();
    let object = objects
        .first()
        .ok_or_else(|| super::super::invalid("receiver omitted verified quarantine object"))?;
    let evidence = super::super::receipt::Evidence {
        inventory: input.inventory.record().clone(),
        request: stage
            .cells
            .requests
            .borrow()
            .first()
            .ok_or_else(|| super::super::invalid("receiver omitted signed request"))?
            .clone(),
        response: object.origin().record().clone(),
        transport: vec![
            super::super::setup::transport::evidence(input.inventory.transport()),
            super::super::setup::transport::evidence(
                stage
                    .cells
                    .request_transport
                    .borrow()
                    .first()
                    .ok_or_else(|| super::super::invalid("receiver omitted send evidence"))?,
            ),
            super::super::setup::transport::evidence(object.origin().transport()),
            crate::preserves_rail::record("federation-live-role-processes-v1", vec![
                crate::preserves_rail::u64_value(input.peer.process_id),
                crate::preserves_rail::u64_value(u64::from(std::process::id())),
            ]),
        ],
        dag: outcome.dag.canonical_receipt.value,
        content: object.content_evidence.clone(),
        admission: policy.take_evidence(),
        import: outcome.imports[0].clone(),
        registry_readback: readback,
        supervised: supervision(service, &outcome.dag.canonical_receipt.record_ref)?,
    };
    super::super::receipt::seal(
        input.adapter,
        input.context,
        &evidence,
        &super::super::setup::exchange::signing(
            input.adapter,
            &input.key.handle.handle,
            molten_core::federation::RecordPurpose::Receipt,
            &input.context.context_ref,
            &input.context.policy_ref,
        ),
    )
}
fn supervision(
    service: &crate::mesh::lifecycle::Service,
    dag_ref: &str,
) -> crate::error::Result<Vec<preserves::IOValue>> {
    let callback = service
        .host
        .evidence()
        .iter()
        .find_map(|evidence| match evidence {
            crate::system_extension::Evidence::Callback(receipt)
                if receipt.invocation.callback == crate::system_extension::CallbackKind::Request
                    && receipt.decision == crate::system_extension::CallbackExecutionDecision::Succeeded =>
            {
                Some(receipt)
            }
            _ => None,
        })
        .ok_or_else(|| super::super::invalid("missing actual host-owned request callback receipt"))?;
    let completion = service
        .host
        .evidence()
        .iter()
        .find_map(|evidence| match evidence {
            crate::system_extension::Evidence::EffectCompletion(receipt)
                if receipt.callback_receipt_ref == callback.receipt_ref
                    && receipt.binding_ref == service.manifest.dag_binding_ref()
                    && receipt.output_ref == dag_ref =>
            {
                Some(receipt)
            }
            _ => None,
        })
        .ok_or_else(|| super::super::invalid("missing actual correlated supervised DAG completion"))?;
    let binding = service
        .manifest
        .extension()
        .required_port_bindings()
        .iter()
        .find(|binding| binding.binding_ref == completion.binding_ref)
        .ok_or_else(|| super::super::invalid("missing admitted DAG binding"))?;
    Ok(vec![
        service.manifest.value().clone(),
        service.manifest.extension().value().clone(),
        binding.binding_value.clone(),
        binding.descriptor_value.clone(),
        binding.registry_value.clone(),
        callback.value.clone(),
        completion.value.clone(),
    ])
}
