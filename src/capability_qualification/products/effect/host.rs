//! The production capability-rooted service host boundary: durable native work
//! whose delivery runs one real child-process effect.

/// Open the host with a fresh runtime whose effect runs `shell` and whose effect
/// attempts are counted in `attempts`.
pub(super) fn open(
    root: &crate::node_state::DirectoryRoot,
    attempts: &std::rc::Rc<std::cell::Cell<u64>>,
    shell: &std::path::Path,
) -> crate::error::Result<crate::node_service_hosting::Host<Authority, ProcessRuntime>> {
    crate::node_service_hosting::Host::open(root, Authority, ProcessRuntime::new(attempts.clone(), shell)?)
}

/// Install the member, submit durable work, claim it and deliver it. The
/// delivery runs one real child-process effect whose outcome is unknown.
pub(super) fn deliver_work(
    host: &mut crate::node_service_hosting::Host<Authority, ProcessRuntime>,
) -> crate::error::Result<crate::error::Result<()>> {
    host.install(
        crate::system_extension::RecoveryGroupDeclaration {
            group_id: super::GROUP_ID.to_string(),
            members: vec![crate::system_extension::RecoveryGroupMember {
                member_id: super::SERVICE_ID.to_string(),
                restart_class: crate::system_extension::RestartClass::Permanent,
                after: Vec::new(),
            }],
            window_ticks: 100,
            backoff_ticks: 1,
            max_restarts_per_member: 1,
            max_restarts_per_group: 1,
            elapsed_source_ref: super::reference("qualification effect logical clock"),
        },
        super::INSTALL_TICK,
    )?;
    host.submit(super::SERVICE_ID, presentation(), payload(), super::SUBMIT_TICK)?;
    let claim = host.readmit(super::SERVICE_ID, &work_ref(), super::CLAIM_TICK)?;
    Ok(host.deliver(&delivery_input(Some(claim.claim_ref))).map(|_| ()))
}

pub(super) fn delivery_input(claim_ref: Option<String>) -> crate::node_service_hosting::DeliveryInput {
    crate::node_service_hosting::DeliveryInput {
        member: super::SERVICE_ID.to_string(),
        presentation: presentation(),
        claim_ref,
        ingress: crate::system_extension::NativeIngressEnvelope {
            schema: crate::system_extension::NATIVE_INGRESS_SCHEMA.to_string(),
            request_ref: super::reference("qualification effect ingress request"),
            endpoint_ref: super::reference("qualification effect endpoint"),
            peer_ref: super::reference("qualification effect peer"),
            service_id: super::SERVICE_ID.to_string(),
            manifest_ref: super::reference("qualification effect manifest"),
            generation: crate::system_extension::Generation::new(super::GENERATION),
            authority_ref: super::reference("qualification effect ingress authority"),
            policy_ref: super::reference("qualification effect policy"),
            resource_ref: super::reference("qualification effect resources"),
            transport_profile_ref: super::reference("qualification effect transport profile"),
            alpn: "aspen/qualification-effect/1".to_string(),
            framing: "length-prefixed".to_string(),
            payload: payload(),
            accounted_bytes: super::WORK_PAYLOAD.len() as u64,
        },
        logical_tick: super::DELIVER_TICK,
    }
}

fn presentation() -> crate::system_extension::WorkPresentation {
    crate::system_extension::WorkPresentation {
        work_ref: work_ref(),
        class: crate::system_extension::WorkClass::Durable,
        generation: crate::system_extension::Generation::new(super::GENERATION),
        incarnation: crate::system_extension::Incarnation::new(0),
    }
}

fn payload() -> crate::system_extension::NativeCallbackValue {
    crate::system_extension::NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(super::WORK_PAYLOAD),
        bytes: super::WORK_PAYLOAD.to_vec(),
    }
}

/// Uncertainty is the typed `UncertainExternalOutcome` class, never any error.
pub(super) fn is_uncertain(delivered: &crate::error::Result<()>) -> bool {
    matches!(
        delivered,
        Err(crate::error::Failure::Classified {
            class: crate::error::FailureClass::UncertainExternalOutcome,
            ..
        })
    )
}

pub(super) fn work_ref() -> String {
    super::reference("qualification durable effect work item")
}

/// The persisted work state from the host's own canonical status readback.
pub(super) fn work_state(status: &preserves::IOValue) -> crate::error::Result<String> {
    let text = crate::preserves_rail::to_text(status)?;
    let marker = format!("<host-work-status-v1 \"{}\" \"{}\" \"", super::SERVICE_ID, work_ref());
    let start = text
        .find(&marker)
        .ok_or_else(|| super::super::super::invalid("host status lacks the retained work"))?;
    let rest = &text[start + marker.len()..];
    let end = rest.find('"').ok_or_else(|| super::super::super::invalid("malformed host work status"))?;
    Ok(rest[..end].to_string())
}

/// Producer-owned admission port: it admits the fixed operations by binding a
/// canonical receipt over the exact request the host asks it to admit.
pub(super) struct Authority;

impl crate::node_service_hosting::AuthorityPort for Authority {
    fn admit(&mut self, request: &crate::node_service_hosting::OperationRequest) -> crate::error::Result<String> {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("qualification-host-admission-v1", vec![
            crate::preserves_rail::string(&request.operation),
            crate::preserves_rail::string(&request.subject),
            crate::preserves_rail::u64_value(request.logical_tick),
            crate::preserves_rail::string(&request.state_ref),
            crate::preserves_rail::string(&request.input_ref),
        ]))
    }
}

/// Runtime port whose native work delivery performs one real child-process
/// effect through the live bounded execution adapter.
pub(super) struct ProcessRuntime {
    instance: crate::system_extension::NativeInstanceRecord,
    pub(super) effect: super::bounded::LiveEffect,
    attempts: std::rc::Rc<std::cell::Cell<u64>>,
}

impl ProcessRuntime {
    pub(super) fn new(
        attempts: std::rc::Rc<std::cell::Cell<u64>>,
        shell: &std::path::Path,
    ) -> crate::error::Result<Self> {
        Ok(Self {
            instance: crate::system_extension::NativeInstanceRecord {
                schema: crate::system_extension::NATIVE_INSTANCE_STATE_SCHEMA.to_string(),
                instance_id: super::reference("qualification effect instance"),
                extension_id: super::EXTENSION_ID.to_string(),
                service_id: super::SERVICE_ID.to_string(),
                manifest_ref: super::reference("qualification effect manifest"),
                executable_ref: super::reference("qualification effect executable"),
                profile_ref: super::reference("qualification effect native profile"),
                state_schema_ref: super::reference("qualification effect state schema"),
                lifecycle: crate::system_extension::LifecycleState {
                    generation: crate::system_extension::Generation::new(super::GENERATION),
                    incarnation: crate::system_extension::Incarnation::new(0),
                    phase: crate::system_extension::LifecyclePhase::Admitted,
                    restart_attempts: 0,
                    health: crate::system_extension::HealthState::Healthy,
                    checkpoint_ref: None,
                },
                usage: crate::system_extension::ResourceUsage::zero(),
                callback_sequence: 0,
                event_sequence: 0,
                state_ref: None,
                checkpoint_ref: None,
                unresolved: Vec::new(),
                completed_operations: Vec::new(),
                completed_operation_refs: Vec::new(),
                evidence_refs: Vec::new(),
                is_accepting_ingress: false,
            },
            effect: super::bounded::LiveEffect::new(shell)?,
            attempts,
        })
    }
}

impl crate::node_service_hosting::RuntimePort for ProcessRuntime {
    fn instance(&self, member: &str) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        if member != super::SERVICE_ID {
            return Err(super::super::super::invalid("unknown qualification effect member"));
        }
        Ok(self.instance.clone())
    }

    fn apply(
        &mut self,
        member: &str,
        action: &crate::node_service_hosting::RuntimeAction,
        _logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        if member != super::SERVICE_ID {
            return Err(super::super::super::invalid("unknown qualification effect member"));
        }
        match action {
            crate::node_service_hosting::RuntimeAction::Start => {
                self.instance.lifecycle.phase = crate::system_extension::LifecyclePhase::Running;
                self.instance.is_accepting_ingress = true;
                self.instance.state_ref = Some(super::reference("qualification effect started state"));
            }
            crate::node_service_hosting::RuntimeAction::Checkpoint => {
                let checkpoint = super::reference("qualification effect checkpoint");
                self.instance.lifecycle.checkpoint_ref = Some(checkpoint.clone());
                self.instance.checkpoint_ref = Some(checkpoint);
            }
            _ => {
                return Err(super::super::super::invalid(
                    "qualification effect runtime supports only start/checkpoint",
                ));
            }
        }
        self.instance.callback_sequence = self.instance.callback_sequence.saturating_add(1);
        Ok(self.instance.clone())
    }

    fn deliver(
        &mut self,
        member: &str,
        _ingress: &crate::system_extension::NativeIngressEnvelope,
        _logical_tick: u64,
    ) -> crate::error::Result<crate::node_service_hosting::RuntimeDelivery> {
        if member != super::SERVICE_ID {
            return Err(super::super::super::invalid("unknown qualification effect member"));
        }
        let failure = self.effect.execute(&self.effect.shell_resolution())?;
        // Count only effects whose child process actually started.
        if self.effect.take_child_started()? {
            self.attempts.set(self.attempts.get().saturating_add(1));
        }
        if failure.kind == crate::fabric_execution::PortFailureKind::UnknownAfterStart
            && self.effect.reconcile() == crate::fabric_execution::ReconciliationStatus::UnknownRequiresReconciliation
        {
            return Err(crate::error::Failure::classified(
                crate::error::FailureClass::UncertainExternalOutcome,
                "native work process effect started without definitive completion",
            ));
        }
        Err(super::super::super::invalid("qualification effect did not reach its post-start boundary"))
    }

    fn migration(&mut self, _member: &str, _migration_ref: &str) -> crate::error::Result<preserves::IOValue> {
        Err(super::super::super::invalid("qualification effect runtime has no migrations"))
    }
}
