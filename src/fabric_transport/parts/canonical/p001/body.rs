
impl ExtensionTransportContext {
    // r[impl molten.fabric_transport.protocol_registration]
    // r[impl molten.fabric_transport.session_streams]
    pub fn from_host<E: SystemExtensionExecutor>(
        host: &crate::system_extension::SystemExtensionHost<E>,
        profile: &CanonicalTransportProfile,
    ) -> crate::error::Result<Self> {
        let key = crate::fabric::FabricPortKey {
            port_id: FABRIC_TRANSPORT_PORT_ID.to_string(),
            version: FABRIC_TRANSPORT_PORT_VERSION.to_string(),
        };
        let binding = host.manifest().binding_for(&key).ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("system extension has no admitted transport port binding")
        })?;
        if binding.binding.implementation_profile != profile.profile.profile_id {
            return Err(crate::error::MoltenError::invalid_harness(
                "system-extension transport profile substitution denied",
            ));
        }
        Ok(Self {
            service_id: host.manifest().manifest().service_id.clone(),
            generation: host.state().generation,
            profile_id: profile.profile.profile_id.clone(),
            max_frame_bytes: profile.profile.limits.max_frame_bytes,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_test_snapshot(service_id: &str, generation: u64, profile: &CanonicalTransportProfile) -> Self {
        Self {
            service_id: service_id.to_string(),
            generation,
            profile_id: profile.profile.profile_id.clone(),
            max_frame_bytes: profile.profile.limits.max_frame_bytes,
        }
    }

    pub fn admit_command(
        &self,
        profile: &CanonicalTransportProfile,
        command: &TransportCommand,
        accounted_bytes: u64,
    ) -> crate::error::Result<()> {
        if self.profile_id != profile.profile.profile_id {
            return Err(crate::error::MoltenError::invalid_harness("transport profile substitution denied"));
        }
        if command.generation() != self.generation {
            return Err(crate::error::MoltenError::invalid_harness(
                "transport command uses a stale service generation",
            ));
        }
        let command_service = command_service_id(command);
        if command_service != self.service_id {
            return Err(crate::error::MoltenError::invalid_harness("transport command service identity mismatch"));
        }
        if accounted_bytes > self.max_frame_bytes {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "transport command bytes {accounted_bytes} exceed {}",
                self.max_frame_bytes
            )));
        }
        Ok(())
    }
}

fn command_service_id(command: &TransportCommand) -> &str {
    match command {
        TransportCommand::Register { descriptor, .. } | TransportCommand::TransferOwnership { descriptor, .. } => {
            &descriptor.service_id
        }
        TransportCommand::BeginDrain { service_id, .. } | TransportCommand::CleanupListener { service_id, .. } => {
            service_id
        }
        TransportCommand::OpenSession { session_id, .. }
        | TransportCommand::OpenStream { session_id, .. }
        | TransportCommand::SendFrame { session_id, .. }
        | TransportCommand::ReceiveFrame { session_id, .. }
        | TransportCommand::AcknowledgeFrame { session_id, .. }
        | TransportCommand::SendDatagram { session_id, .. }
        | TransportCommand::CompleteDatagram { session_id, .. }
        | TransportCommand::GrantCredit { session_id, .. }
        | TransportCommand::HalfCloseStream { session_id, .. }
        | TransportCommand::CloseStream { session_id, .. }
        | TransportCommand::CloseSession { session_id, .. }
        | TransportCommand::FailSession { session_id, .. } => &session_id.service_id,
        TransportCommand::Cancel { target, .. } => match target {
            CancelTarget::Session(session_id) | CancelTarget::Stream { session_id, .. } => &session_id.service_id,
        },
    }
}

fn canonical_event(event: TransportEvent) -> crate::error::Result<CanonicalTransportEvent> {
    let value = crate::preserves_rail::record(TRANSPORT_EVENT_RECORD, vec![
        crate::preserves_rail::string(TRANSPORT_EVENT_SCHEMA),
        field("kind", crate::preserves_rail::string(event.kind.as_str())),
        field("operation-id", crate::preserves_rail::string(&event.operation_id)),
        field("protocol-id", crate::preserves_rail::string(&event.protocol_id)),
        field("session-id", optional_string(event.session_id.as_deref())),
        field("stream-id", optional_string(event.stream_id.as_deref())),
        field("generation", crate::preserves_rail::u64_value(event.generation)),
        field("sequence", optional_u64(event.sequence)),
        field("payload-ref", optional_string(event.payload_ref.as_deref())),
        field("payload-bytes", crate::preserves_rail::u64_value(event.payload_bytes)),
        field(
            "transport-identity-ref",
            optional_string(event.peer.as_ref().map(|peer| peer.transport_identity_ref.as_str())),
        ),
        field(
            "membership-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.membership_ref.as_deref())),
        ),
        field(
            "application-principal-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.application_principal_ref.as_deref())),
        ),
        field(
            "trust-decision-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.trust_decision_ref.as_deref())),
        ),
        field(
            "capability-authority-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.capability_authority_ref.as_deref())),
        ),
        field(
            "bootstrap-policy-ref",
            optional_string(event.peer.as_ref().and_then(|peer| peer.bootstrap_policy_ref.as_deref())),
        ),
        field("failure", optional_string(event.failure.map(|failure| failure.as_str()))),
        field("delivery", crate::preserves_rail::string(event.delivery.as_str())),
        field("retry", crate::preserves_rail::string(event.retry.as_str())),
        field("terminal", crate::preserves_rail::bool_value(event.terminal)),
        checks(&[
            "opaque-generation-scoped-handles",
            "identity-classes-separated",
            "delivery-semantics-explicit",
            "payload-bytes-excluded",
        ]),
    ]);
    let event_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalTransportEvent {
        event_ref,
        event,
        value,
    })
}

fn transport_profile_value(profile: &TransportProfile) -> preserves::IOValue {
    crate::preserves_rail::record(TRANSPORT_PROFILE_RECORD, vec![
        crate::preserves_rail::string(TRANSPORT_PROFILE_SCHEMA),
        field("profile-id", crate::preserves_rail::string(&profile.profile_id)),
        field("declared-profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("adapter-kind", crate::preserves_rail::string(profile.adapter_kind.as_str())),
        field("capabilities", strings_value(profile.capabilities.iter().map(|capability| capability.as_str()))),
        field("max-listeners", crate::preserves_rail::u64_value(profile.limits.max_listeners)),
        field("max-sessions", crate::preserves_rail::u64_value(profile.limits.max_sessions)),
        field("max-streams-per-session", crate::preserves_rail::u64_value(profile.limits.max_streams_per_session)),
        field("max-frame-bytes", crate::preserves_rail::u64_value(profile.limits.max_frame_bytes)),
        field("max-datagram-bytes", crate::preserves_rail::u64_value(profile.limits.max_datagram_bytes)),
        field("max-queued-events", crate::preserves_rail::u64_value(profile.limits.max_queued_events)),
        field("max-queued-bytes", crate::preserves_rail::u64_value(profile.limits.max_queued_bytes)),
        field("max-inflight-bytes", crate::preserves_rail::u64_value(profile.limits.max_inflight_bytes)),
        field(
            "operation-deadline-ticks",
            crate::preserves_rail::u64_value(profile.limits.operation_deadline_ticks),
        ),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "canonical-adapter-neutral-profile",
            "framing-and-resource-bounds-explicit",
            "capabilities-versioned",
            "delivery-non-claims-complete",
        ]),
    ])
}

fn field(name: &str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record("field", vec![crate::preserves_rail::string(name), value])
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.map(crate::preserves_rail::string).collect())
}

fn checks(values: &[&str]) -> preserves::IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn count_value(value: usize) -> crate::error::Result<preserves::IOValue> {
    count(value).map(crate::preserves_rail::u64_value)
}

fn count(value: usize) -> crate::error::Result<u64> {
    u64::try_from(value).map_err(|_| crate::error::MoltenError::invalid_harness("transport collection count overflow"))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation denied: {issues:?}"))
}
