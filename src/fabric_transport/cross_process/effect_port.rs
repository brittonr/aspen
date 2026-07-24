use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::time::Duration;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_transport::*;

const MAX_EFFECT_TIMEOUT_SECONDS: u64 = 300;

#[derive(Debug, Clone)]
pub struct IrohCrossProcessEffectClientConfig {
    pub capability: IrohEndpointCapability,
    pub bind_addr: SocketAddr,
    pub endpoint: CanonicalCrossProcessEndpoint,
    pub expected: ExpectedEndpointBinding,
    pub admission: EndpointAdmissionState,
    pub timeout: Duration,
}

pub struct RegisteredCrossProcessTransportEffectPort {
    control: RegisteredTransportEffectPort<IrohTransportAdapter>,
    profile: CanonicalTransportProfile,
    protocol: ProtocolDescriptor,
    client: IrohCrossProcessEffectClientConfig,
    payloads: BTreeMap<String, Vec<u8>>,
    queued_payload_bytes: u64,
    routed_requests: BTreeSet<String>,
    latest_frame_evidence: Option<CrossProcessFrameEvidence>,
}

impl RegisteredCrossProcessTransportEffectPort {
    // r[impl molten.fabric_transport.cross_process_session]
    // r[impl molten.fabric_transport.distinct_process_evidence]
    pub fn new(
        context: ExtensionTransportContext,
        profile: CanonicalTransportProfile,
        protocol: ProtocolDescriptor,
        client: IrohCrossProcessEffectClientConfig,
    ) -> Result<Self> {
        validate_effect_client_config(&profile, &protocol, &client)?;
        let adapter = IrohTransportAdapter::new(profile.clone())?;
        let control = RegisteredTransportEffectPort::new(adapter, context, profile.clone())?;
        Ok(Self {
            control,
            profile,
            protocol,
            client,
            payloads: BTreeMap::new(),
            queued_payload_bytes: 0,
            routed_requests: BTreeSet::new(),
            latest_frame_evidence: None,
        })
    }

    pub fn register(&mut self, request_ref: String, command: TransportCommand, payload: Option<Vec<u8>>) -> Result<()> {
        let admitted_payload =
            admit_registered_payload(&self.profile.profile, &request_ref, &command, payload.as_deref())?;
        if let Some(payload) = &admitted_payload {
            let payload_bytes = u64::try_from(payload.len())
                .map_err(|_| MoltenError::invalid_harness("queued payload size does not fit u64"))?;
            let next_queued = self
                .queued_payload_bytes
                .checked_add(payload_bytes)
                .ok_or_else(|| MoltenError::invalid_harness("queued payload accounting overflow"))?;
            let queued_count = u64::try_from(self.payloads.len())
                .map_err(|_| MoltenError::invalid_harness("queued payload count does not fit u64"))?;
            if queued_count >= self.profile.profile.limits.max_queued_events
                || next_queued > self.profile.profile.limits.max_queued_bytes
            {
                return Err(MoltenError::invalid_harness(
                    "cross-process effect payload queue exceeds the admitted profile",
                ));
            }
        }
        self.control.register(request_ref.clone(), command)?;
        if let Some(payload) = admitted_payload {
            let payload_bytes = u64::try_from(payload.len())
                .map_err(|_| MoltenError::invalid_harness("queued payload size does not fit u64"))?;
            let prior = self.payloads.insert(request_ref, payload);
            debug_assert!(prior.is_none());
            self.queued_payload_bytes = self
                .queued_payload_bytes
                .checked_add(payload_bytes)
                .ok_or_else(|| MoltenError::invalid_harness("queued payload accounting overflow"))?;
        }
        Ok(())
    }

    pub fn adapter(&self) -> &IrohTransportAdapter {
        self.control.adapter()
    }

    pub fn latest_frame_evidence(&self) -> Option<&CrossProcessFrameEvidence> {
        self.latest_frame_evidence.as_ref()
    }

    fn route_effect(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> std::result::Result<crate::system_extension::PortEffectOutput, String> {
        if self.routed_requests.contains(&effect.request_ref) {
            return Err("cross-process transport effect request replay denied".to_string());
        }
        let (command, submitted) = self.control.execute_effect(binding, effect)?;
        self.routed_requests.insert(effect.request_ref.clone());
        let TransportCommand::SendFrame {
            operation_id,
            session_id,
            stream_id,
            payload_ref,
            payload_bytes,
            ..
        } = command
        else {
            return Ok(effect_output(effect, submitted.transition_ref));
        };
        if submitted.decision != TransportTransitionDecision::Applied {
            self.remove_payload(&effect.request_ref)?;
            return Ok(effect_output(effect, submitted.transition_ref));
        }
        let payload = self.remove_payload(&effect.request_ref)?;
        let exchange = run_effect_exchange(
            &self.profile,
            &self.protocol,
            &self.client,
            &session_id,
            &effect.request_ref,
            &payload,
        );
        let evidence = match exchange {
            Ok(evidence) => evidence,
            Err(_error) => {
                let failed = fail_canonical_session(
                    self.control.adapter_mut(),
                    &operation_id,
                    &session_id,
                    TransportFailureClass::AdapterFailure,
                )?;
                return Ok(effect_output(effect, failed.transition_ref));
            }
        };
        self.latest_frame_evidence = Some(evidence.clone());
        if !matching_exchange_evidence(&evidence, &effect.request_ref, &payload_ref, payload_bytes) {
            let failed = fail_canonical_session(
                self.control.adapter_mut(),
                &operation_id,
                &session_id,
                TransportFailureClass::MalformedInput,
            )?;
            return Ok(effect_output(effect, failed.transition_ref));
        }
        let acknowledged = self
            .control
            .adapter_mut()
            .execute_command(&TransportCommand::AcknowledgeFrame {
                operation_id,
                session_id,
                stream_id,
                payload_bytes,
            })
            .map_err(|error| error.to_string())?;
        Ok(effect_output(effect, acknowledged.transition_ref))
    }

    fn remove_payload(&mut self, request_ref: &str) -> std::result::Result<Vec<u8>, String> {
        let payload = self
            .payloads
            .remove(request_ref)
            .ok_or_else(|| "cross-process transport effect payload is not registered".to_string())?;
        let payload_bytes =
            u64::try_from(payload.len()).map_err(|_| "queued payload size does not fit u64".to_string())?;
        self.queued_payload_bytes = self
            .queued_payload_bytes
            .checked_sub(payload_bytes)
            .ok_or_else(|| "queued payload accounting underflow".to_string())?;
        Ok(payload)
    }
}

// r[impl molten.fabric_transport.cross_process_session]
// r[impl molten.fabric_transport.distinct_process_evidence]
impl crate::system_extension::FabricEffectPort for RegisteredCrossProcessTransportEffectPort {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &crate::system_extension::TypedEffectRequest,
    ) -> std::result::Result<crate::system_extension::PortEffectOutput, String> {
        self.route_effect(binding, effect)
    }
}

fn validate_effect_client_config(
    profile: &CanonicalTransportProfile,
    protocol: &ProtocolDescriptor,
    client: &IrohCrossProcessEffectClientConfig,
) -> Result<()> {
    if profile.profile.adapter_kind != TransportAdapterKind::IrohLive {
        return Err(MoltenError::invalid_harness("cross-process effect port requires an iroh-live profile"));
    }
    if client.timeout.is_zero() || client.timeout > Duration::from_secs(MAX_EFFECT_TIMEOUT_SECONDS) {
        return Err(MoltenError::invalid_harness(format!(
            "cross-process effect timeout must be between one nanosecond and {MAX_EFFECT_TIMEOUT_SECONDS} seconds"
        )));
    }
    admit_endpoint_import(&profile.profile, protocol, &client.endpoint.descriptor, &client.expected, client.admission)
        .map_err(|issues| MoltenError::invalid_harness(format!("cross-process effect endpoint denied: {issues:?}")))?;
    Ok(())
}

fn admit_registered_payload(
    profile: &TransportProfile,
    request_ref: &str,
    command: &TransportCommand,
    payload: Option<&[u8]>,
) -> Result<Option<Vec<u8>>> {
    crate::preserves_rail::validate_content_ref(request_ref)?;
    match (command, payload) {
        (
            TransportCommand::SendFrame {
                payload_ref,
                payload_bytes,
                ..
            },
            Some(payload),
        ) => {
            let observed_bytes = u64::try_from(payload.len())
                .map_err(|_| MoltenError::invalid_harness("cross-process effect payload size does not fit u64"))?;
            if observed_bytes == 0
                || observed_bytes != *payload_bytes
                || observed_bytes > profile.limits.max_frame_bytes
            {
                return Err(MoltenError::invalid_harness(
                    "cross-process effect payload length does not match its admitted command",
                ));
            }
            let expected_ref = cross_process_frame_ref(request_ref, payload);
            if payload_ref != &expected_ref {
                return Err(MoltenError::invalid_harness(
                    "cross-process effect payload ref does not match its request-bound bytes",
                ));
            }
            Ok(Some(payload.to_vec()))
        }
        (TransportCommand::SendFrame { .. }, None) => Err(MoltenError::invalid_harness(
            "cross-process send effect requires explicitly registered payload bytes",
        )),
        (_, Some(_)) => {
            Err(MoltenError::invalid_harness("cross-process non-send effect must not register payload bytes"))
        }
        (_, None) => Ok(None),
    }
}

fn run_effect_exchange(
    profile: &CanonicalTransportProfile,
    protocol: &ProtocolDescriptor,
    client: &IrohCrossProcessEffectClientConfig,
    session_id: &ScopedTransportId,
    request_ref: &str,
    payload: &[u8],
) -> Result<CrossProcessFrameEvidence> {
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
            MoltenError::invalid_harness(format!("cross-process effect runtime creation failed: {error}"))
        })?;
        runtime.block_on(exchange_cross_process_frame(input, &payload, timeout))
    })
    .join()
    .map_err(|_| MoltenError::invalid_harness("cross-process effect worker panicked"))?
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
    }
}
