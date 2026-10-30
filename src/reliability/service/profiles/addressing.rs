//! Endpoint facts are decoded, not authorized. The listener/client must still run endpoint
//! admission.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointAdmissionStateDto {
    pub registration_active: bool,
    pub transport_capability_active: bool,
    pub protocol_capability_active: bool,
    pub profile_active: bool,
    pub listener_ready: bool,
}

impl EndpointAdmissionStateDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_transport::EndpointAdmissionState> {
        Ok(crate::fabric_transport::EndpointAdmissionState {
            registration_active: self.registration_active,
            transport_capability_active: self.transport_capability_active,
            protocol_capability_active: self.protocol_capability_active,
            profile_active: self.profile_active,
            listener_ready: self.listener_ready,
        })
    }
    pub fn into_state(self) -> crate::error::Result<crate::fabric_transport::EndpointAdmissionState> {
        self.into_raw()
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedEndpointBindingDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub profile_ref: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub protocol_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub protocol_version: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub alpn: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub extension_id: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub service_id: String,
    pub generation: u64,
    #[serde(deserialize_with = "super::bounded::text")]
    pub public_endpoint_identity: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub listener_identity_ref: String,
    #[serde(deserialize_with = "super::bounded::text")]
    pub peer_context_ref: String,
    pub observed_tick: u64,
}

impl ExpectedEndpointBindingDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_transport::ExpectedEndpointBinding> {
        Ok(crate::fabric_transport::ExpectedEndpointBinding {
            profile_id: self.profile_id,
            profile_ref: self.profile_ref,
            protocol_id: self.protocol_id,
            protocol_version: self.protocol_version,
            alpn: self.alpn,
            extension_id: self.extension_id,
            service_id: self.service_id,
            generation: self.generation,
            public_endpoint_identity: self.public_endpoint_identity,
            listener_identity_ref: self.listener_identity_ref,
            peer_context_ref: self.peer_context_ref,
            observed_tick: self.observed_tick,
        })
    }
    pub fn into_binding(self) -> crate::error::Result<crate::fabric_transport::ExpectedEndpointBinding> {
        self.into_raw()
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointDisclosurePolicyDto {
    #[serde(deserialize_with = "super::bounded::texts")]
    pub explicit_handoff_classes: Vec<String>,
    pub default_readback_redacted: bool,
}

impl EndpointDisclosurePolicyDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_transport::EndpointDisclosurePolicy> {
        Ok(crate::fabric_transport::EndpointDisclosurePolicy {
            explicit_handoff_classes: super::choices(
                self.explicit_handoff_classes,
                &[
                    crate::fabric_transport::EndpointLocatorClass::Ip,
                    crate::fabric_transport::EndpointLocatorClass::Relay,
                    crate::fabric_transport::EndpointLocatorClass::Custom,
                    crate::fabric_transport::EndpointLocatorClass::Private,
                ],
                crate::fabric_transport::EndpointLocatorClass::as_str,
            )?,
            default_readback_redacted: self.default_readback_redacted,
        })
    }
    pub fn into_policy(self) -> crate::error::Result<crate::fabric_transport::EndpointDisclosurePolicy> {
        self.into_raw()
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointValidityCohortDto {
    #[serde(deserialize_with = "super::bounded::text")]
    pub cohort_ref: String,
    pub not_before_tick: u64,
    pub expires_at_tick: u64,
}

impl EndpointValidityCohortDto {
    fn into_raw(self) -> crate::error::Result<crate::fabric_transport::EndpointValidityCohort> {
        Ok(crate::fabric_transport::EndpointValidityCohort {
            cohort_ref: self.cohort_ref,
            not_before_tick: self.not_before_tick,
            expires_at_tick: self.expires_at_tick,
        })
    }
    pub fn into_cohort(self) -> crate::error::Result<crate::fabric_transport::EndpointValidityCohort> {
        if self.not_before_tick >= self.expires_at_tick {
            return Err(super::invalid("endpoint validity cohort is empty or reversed"));
        }
        self.into_raw()
    }
}
