pub struct SessionPullInput<'a> {
    pub session_ref: &'a str,
    pub now: u64,
    pub graph: &'a molten_core::dag_sync::DagGraph,
    pub request: molten_core::dag_sync::DagSyncRequest,
    pub effect: crate::system_extension::TypedEffectRequest,
    pub quarantined: &'a std::cell::RefCell<Vec<super::quarantine::QuarantinedObject>>,
}

pub struct PullOutcome {
    pub session: molten_core::federation::Session,
    pub dag: crate::dag_sync::DagSyncOutcome,
    pub imports: Vec<preserves::IOValue>,
    pub status_ref: String,
}

// r[impl molten.federated_pull_runtime.manifest]
// r[impl molten.federated_pull_runtime.receiver_pull]
pub fn execute_pull<'a, A, R, T, C, P, O, E>(
    service: &mut crate::mesh::lifecycle::Service,
    input: SessionPullInput<'a>,
    dag_ports: crate::dag_sync::DagSyncPorts<'a, A, R, T, C, P, O, E>,
    mut ports: super::import::Ports<'_>,
) -> crate::error::Result<PullOutcome>
where
    A: crate::dag_sync::DagAuthorityPort,
    R: crate::dag_sync::DagResourcePort,
    T: crate::dag_sync::DagTransportPort,
    C: crate::dag_sync::DagContentVerificationPort,
    P: crate::dag_sync::DagProgressPort,
    O: crate::dag_sync::DagObservationPort,
    E: crate::dag_sync::DagReceiptPort,
{
    service.require_running()?;
    let session = service.active.get(input.session_ref).ok_or_else(|| invalid("unknown receiver pull session"))?;
    validate_request(session, &input, service.manifest.bounds())?;
    let mut session = molten_core::federation::advance_session(
        session,
        &molten_core::federation::SessionEvent::BeginFetch,
        service.manifest.generation(),
        input.now,
        service.manifest.bounds(),
    )
    .map_err(|issue| invalid(&format!("federation fetch transition denied: {issue:?}")))?;
    let canonical = crate::dag_sync::canonical_dag_request(&input.request)?;
    require_supervised_effect(&input.effect, &canonical.record_ref, session.generation)?;
    service.admit_fetch(&session, input.now, ports.durable)?;
    service.host.executor_mut().pending = Some(input.effect);
    let (callback, _) = service
        .host
        .dispatch_request(
            &canonical.record_ref,
            u64::try_from(canonical.bytes.len()).map_err(|_| invalid("DAG request byte count overflow"))?,
            input.now,
        )?
        .require_executed("federation pull")?;
    service.retain_status(&session, ports.durable)?;
    service.active.insert(session.session_ref.clone(), session.clone());
    let mut router = DagRoute {
        graph: input.graph,
        request: Some(input.request),
        binding_ref: service.manifest.dag_binding_ref().to_string(),
        request_ref: canonical.record_ref,
        ports: Some(dag_ports),
        outcome: None,
    };
    if let Err(error) = service.host.route_approved_effects(&callback, &mut router) {
        retain_failure(service, &session, input.now, ports.durable, "transfer-failed")?;
        return Err(error);
    }
    let dag = router.outcome.take().ok_or_else(|| invalid("supervised DAG effect produced no outcome"))?;
    let imports = if dag.receipt.decision == molten_core::dag_sync::DagSyncDecision::Complete {
        let context = ImportContext {
            now: input.now,
            quarantined: input.quarantined,
            dag: &dag,
        };
        import_missing(service, &mut session, &context, &mut ports)?
    } else {
        Vec::new()
    };
    let status_ref = service.retain_status(&session, ports.durable)?;
    if session.phase == molten_core::federation::SessionPhase::Committed {
        service.active.remove(&session.session_ref);
    } else {
        service.active.insert(session.session_ref.clone(), session.clone());
    }
    Ok(PullOutcome {
        session,
        dag,
        imports,
        status_ref,
    })
}

struct ImportContext<'a> {
    now: u64,
    quarantined: &'a std::cell::RefCell<Vec<super::quarantine::QuarantinedObject>>,
    dag: &'a crate::dag_sync::DagSyncOutcome,
}

/// Imports each planned head from quarantine after a complete DAG, retaining status after every
/// commit.
fn import_missing(
    service: &mut crate::mesh::lifecycle::Service,
    session: &mut molten_core::federation::Session,
    context: &ImportContext<'_>,
    ports: &mut super::import::Ports<'_>,
) -> crate::error::Result<Vec<preserves::IOValue>> {
    let mut imports = Vec::with_capacity(session.plan.missing.len());
    for index in 0..session.plan.missing.len() {
        let head_ref = session.plan.missing[index].head_ref.clone();
        let completed = context.quarantined.borrow();
        let object = completed
            .iter()
            .find(|object| object.object_ref.as_str() == head_ref)
            .ok_or_else(|| invalid("complete DAG omitted quarantine content required for import"))?;
        let imported = super::import::commit_verified(
            &super::import::Input {
                session,
                bounds: service.manifest.bounds(),
                generation: service.manifest.generation(),
                now: context.now,
                head_ref: &head_ref,
                manifest_ref: &object.manifest_ref,
                dag: context.dag,
            },
            super::import::Ports {
                admission: &mut *ports.admission,
                quarantine: ports.quarantine,
                registry: ports.registry,
                durable: ports.durable,
            },
        );
        let imported = match imported {
            Ok(imported) => imported,
            Err(error) => {
                retain_failure(service, session, context.now, ports.durable, "import-requires-readback")?;
                return Err(error);
            }
        };
        *session = imported.session;
        imports.push(imported.receipt);
        service.retain_status(session, ports.durable)?;
        if session.phase == molten_core::federation::SessionPhase::Committed {
            service.active.remove(&session.session_ref);
        } else {
            // The fetching session was admitted into `active` before import; replace it in place.
            let active = service
                .active
                .get_mut(&session.session_ref)
                .ok_or_else(|| invalid("importing session is no longer active"))?;
            *active = session.clone();
        }
    }
    Ok(imports)
}

/// The supervised effect must name exactly this receiver DAG request at the session generation.
pub(crate) fn require_supervised_effect(
    effect: &crate::system_extension::TypedEffectRequest,
    request_ref: &str,
    generation: u64,
) -> crate::error::Result<()> {
    if effect.request_ref != request_ref || effect.generation != generation {
        return Err(invalid("supervised effect does not match receiver DAG request"));
    }
    Ok(())
}

fn validate_request(
    session: &molten_core::federation::Session,
    input: &SessionPullInput<'_>,
    bounds: &molten_core::federation::PullBounds,
) -> crate::error::Result<()> {
    if input.request.generation != session.generation
        || input.request.epoch_ref.as_str() != session.session_ref
        || u64::try_from(input.request.bounds.max_nodes).map_err(|_| invalid("receiver DAG node bound overflow"))?
            > bounds.dag_nodes
        || input.request.bounds.max_bytes > bounds.fetch_bytes
    {
        return Err(invalid("receiver DAG request generation, epoch or resource bounds drifted"));
    }
    for root in &input.request.root_refs {
        let root = input
            .graph
            .roots
            .iter()
            .find(|candidate| &candidate.root_ref == root)
            .ok_or_else(|| invalid("receiver requested an unknown graph root"))?;
        if !session.plan.missing.iter().any(|entry| entry.head_ref == root.node_ref.as_str()) {
            return Err(invalid("receiver DAG graph includes a root absent from the missing set"));
        }
    }
    Ok(())
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

fn retain_failure(
    service: &mut crate::mesh::lifecycle::Service,
    session: &molten_core::federation::Session,
    now: u64,
    durable: &crate::node_state::Scope,
    kind: &'static str,
) -> crate::error::Result<()> {
    let value = crate::preserves_rail::record("federation-effect-failure-v1", vec![
        crate::preserves_rail::string(&session.session_ref),
        crate::preserves_rail::u64_value(session.generation),
        crate::preserves_rail::u64_value(now),
        crate::preserves_rail::string(kind),
        crate::preserves_rail::u64_value(u64::from(session.attempts)),
        crate::preserves_rail::string(
            "protected-effect observation; import intent requires registry readback, not blind retry",
        ),
    ]);
    let slot = service.status_slot()?;
    let path = crate::node_state::RelativePath::parse(&format!("federation-effect-failure-{slot}.preserves"))?;
    durable.write(&path, &crate::preserves_rail::canonical_bytes(&value)?)?;
    service.retain_status(session, durable)?;
    Ok(())
}

struct DagRoute<'a, A, R, T, C, P, O, E> {
    pub graph: &'a molten_core::dag_sync::DagGraph,
    pub request: Option<molten_core::dag_sync::DagSyncRequest>,
    pub binding_ref: String,
    pub request_ref: String,
    pub ports: Option<crate::dag_sync::DagSyncPorts<'a, A, R, T, C, P, O, E>>,
    pub outcome: Option<crate::dag_sync::DagSyncOutcome>,
}
impl<A, R, T, C, P, O, E> crate::system_extension::FabricEffectPort for DagRoute<'_, A, R, T, C, P, O, E>
where
    A: crate::dag_sync::DagAuthorityPort,
    R: crate::dag_sync::DagResourcePort,
    T: crate::dag_sync::DagTransportPort,
    C: crate::dag_sync::DagContentVerificationPort,
    P: crate::dag_sync::DagProgressPort,
    O: crate::dag_sync::DagObservationPort,
    E: crate::dag_sync::DagReceiptPort,
{
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> crate::fabric::FabricPortResult<crate::system_extension::PortEffectOutput> {
        let request = self
            .request
            .take()
            .ok_or_else(|| crate::fabric::FabricPortError::capability("federation DAG effect cannot execute twice"))?;
        if binding.binding_ref != self.binding_ref
            || effect.request_ref != self.request_ref
            || effect.generation != request.generation
        {
            return Err(crate::fabric::FabricPortError::capability(
                "federation DAG effect binding or generation drifted",
            ));
        }
        let ports = self
            .ports
            .take()
            .ok_or_else(|| crate::fabric::FabricPortError::capability("federation DAG effect cannot execute twice"))?;
        let outcome =
            crate::dag_sync::run_dag_sync(self.graph, request, ports).map_err(crate::fabric::FabricPortError::from)?;
        let output_ref = outcome.canonical_receipt.record_ref.clone();
        self.outcome = Some(outcome);
        Ok(crate::system_extension::PortEffectOutput {
            output_schema_ref: effect.output_schema_ref.clone(),
            output_ref,
            materialized_output: None,
        })
    }
}
