#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportLimitsDto {
    pub max_listeners: u64,
    pub max_sessions: u64,
    pub max_streams_per_session: u64,
    pub max_frame_bytes: u64,
    pub max_datagram_bytes: u64,
    pub max_queued_events: u64,
    pub max_queued_bytes: u64,
    pub max_inflight_bytes: u64,
    pub operation_deadline_ticks: u64,
}

impl TransportLimitsDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_transport::TransportLimits> {
        Ok(crate::fabric_transport::TransportLimits {
            max_listeners: self.max_listeners,
            max_sessions: self.max_sessions,
            max_streams_per_session: self.max_streams_per_session,
            max_frame_bytes: self.max_frame_bytes,
            max_datagram_bytes: self.max_datagram_bytes,
            max_queued_events: self.max_queued_events,
            max_queued_bytes: self.max_queued_bytes,
            max_inflight_bytes: self.max_inflight_bytes,
            operation_deadline_ticks: self.operation_deadline_ticks,
        })
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportProfileDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub schema: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_ref: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub adapter_kind: String,
    #[serde(deserialize_with = "super::bounded::texts")]
    pub capabilities: Vec<String>,
    pub limits: TransportLimitsDto,
    #[serde(deserialize_with = "super::bounded::texts")]
    pub non_claims: Vec<String>,
}

impl TransportProfileDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_transport::TransportProfile> {
        Ok(crate::fabric_transport::TransportProfile {
            schema: self.schema,
            profile_id: self.profile_id,
            profile_ref: self.profile_ref,
            adapter_kind: super::choice(
                &self.adapter_kind,
                &[
                    crate::fabric_transport::TransportAdapterKind::IrohLive,
                    crate::fabric_transport::TransportAdapterKind::DeterministicSimulation,
                ],
                crate::fabric_transport::TransportAdapterKind::as_str,
            )?,
            capabilities: super::choices(
                self.capabilities,
                &[
                    crate::fabric_transport::TransportCapability::BidirectionalStreams,
                    crate::fabric_transport::TransportCapability::UnidirectionalStreams,
                    crate::fabric_transport::TransportCapability::Datagrams,
                ],
                crate::fabric_transport::TransportCapability::as_str,
            )?,
            limits: self.limits.into_raw()?,
            non_claims: super::choices(
                self.non_claims,
                &crate::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS,
                crate::fabric_transport::TransportNonClaim::as_str,
            )?,
        })
    }
    pub fn into_profile(self) -> crate::error::Result<crate::fabric_transport::Profile> {
        crate::fabric_transport::profile(&self.into_raw()?)
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FramingProfileDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_ref: String,
    pub max_frame_bytes: u64,
    pub length_prefix_bytes: u64,
    pub payload_hash_required: bool,
}

impl FramingProfileDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_transport::FramingProfile> {
        Ok(crate::fabric_transport::FramingProfile {
            profile_id: self.profile_id,
            profile_ref: self.profile_ref,
            max_frame_bytes: self.max_frame_bytes,
            length_prefix_bytes: self.length_prefix_bytes,
            payload_hash_required: self.payload_hash_required,
        })
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ListenerCleanupPolicyDto {
    Immediate,
    BoundedDrain { grace_ticks: u64 },
}

impl ListenerCleanupPolicyDto {
    fn into_raw(self) -> crate::fabric_transport::ListenerCleanupPolicy {
        match self {
            Self::Immediate => crate::fabric_transport::ListenerCleanupPolicy::Immediate,
            Self::BoundedDrain { grace_ticks } => {
                crate::fabric_transport::ListenerCleanupPolicy::BoundedDrain { grace_ticks }
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolDescriptorDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub schema: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub protocol_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub version: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub alpn: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub extension_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub service_id: String,
    pub generation: u64,
    pub listener_limit: u64,
    #[serde(deserialize_with = "super::bounded::texts")]
    pub requested_capabilities: Vec<String>,
    pub framing: FramingProfileDto,
    pub cleanup_policy: ListenerCleanupPolicyDto,
    #[serde(deserialize_with = "super::bounded::text")]
    pub registration_authority_ref: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_ref: String,
}

impl ProtocolDescriptorDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_transport::ProtocolDescriptor> {
        Ok(crate::fabric_transport::ProtocolDescriptor {
            schema: self.schema,
            protocol_id: self.protocol_id,
            version: self.version,
            alpn: self.alpn,
            extension_id: self.extension_id,
            service_id: self.service_id,
            generation: self.generation,
            listener_limit: self.listener_limit,
            requested_capabilities: super::choices(
                self.requested_capabilities,
                &[
                    crate::fabric_transport::TransportCapability::BidirectionalStreams,
                    crate::fabric_transport::TransportCapability::UnidirectionalStreams,
                    crate::fabric_transport::TransportCapability::Datagrams,
                ],
                crate::fabric_transport::TransportCapability::as_str,
            )?,
            framing: self.framing.into_raw()?,
            cleanup_policy: self.cleanup_policy.into_raw(),
            registration_authority_ref: self.registration_authority_ref,
            profile_ref: self.profile_ref,
        })
    }
    pub fn into_descriptor(
        self,
        profile: &crate::fabric_transport::Profile,
    ) -> crate::error::Result<crate::fabric_transport::ProtocolDescriptor> {
        let descriptor = self.into_raw()?;
        crate::fabric_transport::validate_protocol_descriptor(&profile.profile, &descriptor)
            .map_err(|issues| super::invalid(format!("invalid protocol descriptor: {issues:?}")))?;
        Ok(descriptor)
    }
}
