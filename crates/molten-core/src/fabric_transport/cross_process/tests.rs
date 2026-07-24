use super::*;

const PROFILE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FRAMING_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const AUTHORITY_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DESCRIPTOR_REF: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const LISTENER_REF: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const PEER_CONTEXT_REF: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const LOCATOR_COHORT_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const VALIDITY_COHORT_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const SESSION_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";
const CLEANUP_REF: &str = "blake3:4444444444444444444444444444444444444444444444444444444444444444";
const ENDPOINT_IDENTITY: &str = "iroh:abcdefghijklmnopqrstuvwxyz234567";
const IP_LOCATOR: &str = "ip:127.0.0.1:49152";
const RELAY_LOCATOR: &str = "relay:https://relay.example.invalid";
const GENERATION: u64 = 1;
const REPLACEMENT_GENERATION: u64 = GENERATION + 1;
const STALE_GENERATION: u64 = REPLACEMENT_GENERATION;
const LISTENER_LIMIT: u64 = 2;
const SESSION_LIMIT: u64 = 2;
const STREAM_LIMIT: u64 = 4;
const FRAME_LIMIT: u64 = 1_024;
const DATAGRAM_LIMIT: u64 = 512;
const QUEUE_EVENT_LIMIT: u64 = 8;
const QUEUE_BYTE_LIMIT: u64 = 4_096;
const INFLIGHT_LIMIT: u64 = 2_048;
const DEADLINE_WINDOW: u64 = 32;
const LENGTH_PREFIX_BYTES: u64 = 4;
const VALID_FROM_TICK: u64 = 10;
const VALID_UNTIL_TICK: u64 = 100;
const OBSERVED_TICK: u64 = 20;
const FRAME_BYTES: u64 = 64;
const OVERSIZED_FRAME_BYTES: u64 = FRAME_LIMIT + 1;

fn profile() -> TransportProfile {
    TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "iroh-cross-process-v1".to_string(),
        profile_ref: PROFILE_REF.to_string(),
        adapter_kind: TransportAdapterKind::IrohLive,
        capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        limits: TransportLimits {
            max_listeners: LISTENER_LIMIT,
            max_sessions: SESSION_LIMIT,
            max_streams_per_session: STREAM_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_datagram_bytes: DATAGRAM_LIMIT,
            max_queued_events: QUEUE_EVENT_LIMIT,
            max_queued_bytes: QUEUE_BYTE_LIMIT,
            max_inflight_bytes: INFLIGHT_LIMIT,
            operation_deadline_ticks: DEADLINE_WINDOW,
        },
        non_claims: REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    }
}

fn protocol() -> ProtocolDescriptor {
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "cross-process-echo".to_string(),
        version: "v1".to_string(),
        alpn: "molten/cross-process-echo/1".to_string(),
        extension_id: "echo-extension".to_string(),
        service_id: "echo-service".to_string(),
        generation: GENERATION,
        listener_limit: 1,
        requested_capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        framing: FramingProfile {
            profile_id: "length-delimited-blake3-v1".to_string(),
            profile_ref: FRAMING_REF.to_string(),
            max_frame_bytes: FRAME_LIMIT,
            length_prefix_bytes: LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: DEADLINE_WINDOW,
        },
        registration_authority_ref: AUTHORITY_REF.to_string(),
        profile_ref: PROFILE_REF.to_string(),
    }
}

fn descriptor() -> CrossProcessEndpointDescriptor {
    CrossProcessEndpointDescriptor {
        schema: CROSS_PROCESS_ENDPOINT_SCHEMA.to_string(),
        descriptor_ref: DESCRIPTOR_REF.to_string(),
        profile_id: profile().profile_id,
        profile_ref: PROFILE_REF.to_string(),
        protocol_id: protocol().protocol_id,
        protocol_version: protocol().version,
        alpn: protocol().alpn,
        extension_id: protocol().extension_id,
        service_id: protocol().service_id,
        generation: GENERATION,
        public_endpoint_identity: ENDPOINT_IDENTITY.to_string(),
        listener_identity_ref: LISTENER_REF.to_string(),
        expected_peer_context_ref: PEER_CONTEXT_REF.to_string(),
        locator_cohort_ref: LOCATOR_COHORT_REF.to_string(),
        locators: vec![
            EndpointLocator {
                class: EndpointLocatorClass::Ip,
                value: IP_LOCATOR.to_string(),
            },
            EndpointLocator {
                class: EndpointLocatorClass::Relay,
                value: RELAY_LOCATOR.to_string(),
            },
        ],
        disclosure: EndpointDisclosurePolicy {
            explicit_handoff_classes: vec![EndpointLocatorClass::Ip, EndpointLocatorClass::Relay],
            default_readback_redacted: true,
        },
        framing_profile_ref: FRAMING_REF.to_string(),
        resources: EndpointResourceBounds {
            max_sessions: SESSION_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_queued_bytes: QUEUE_BYTE_LIMIT,
            max_inflight_bytes: INFLIGHT_LIMIT,
        },
        validity: EndpointValidityCohort {
            cohort_ref: VALIDITY_COHORT_REF.to_string(),
            not_before_tick: VALID_FROM_TICK,
            expires_at_tick: VALID_UNTIL_TICK,
        },
        non_claims: REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    }
}

fn expected_binding() -> ExpectedEndpointBinding {
    let descriptor = descriptor();
    ExpectedEndpointBinding {
        profile_id: descriptor.profile_id,
        profile_ref: descriptor.profile_ref,
        protocol_id: descriptor.protocol_id,
        protocol_version: descriptor.protocol_version,
        alpn: descriptor.alpn,
        extension_id: descriptor.extension_id,
        service_id: descriptor.service_id,
        generation: descriptor.generation,
        public_endpoint_identity: descriptor.public_endpoint_identity,
        listener_identity_ref: descriptor.listener_identity_ref,
        peer_context_ref: descriptor.expected_peer_context_ref,
        observed_tick: OBSERVED_TICK,
    }
}

fn planned_listener() -> CrossProcessListenerState {
    plan_cross_process_listener(&profile(), &protocol(), &descriptor(), &[]).expect("listener plan")
}

fn ready_listener() -> CrossProcessListenerState {
    let starting = apply_cross_process_listener_command(&planned_listener(), &CrossProcessListenerCommand::Start)
        .expect("listener start")
        .next;
    apply_cross_process_listener_command(
        &starting,
        &CrossProcessListenerCommand::MarkReady(ListenerReadinessObservation::fully_ready()),
    )
    .expect("listener ready")
    .next
}

fn dial_plan() -> EndpointDialPlan {
    admit_endpoint_import(
        &profile(),
        &protocol(),
        &descriptor(),
        &expected_binding(),
        EndpointAdmissionState::fully_active(),
    )
    .expect("endpoint import")
}

fn active_session() -> CrossProcessSessionState {
    let planned =
        plan_cross_process_session(&dial_plan(), SESSION_REF, EndpointParticipantRole::Client).expect("session plan");
    let dialing = apply_cross_process_session_command(&planned, &CrossProcessSessionCommand::BeginDial {
        observed_descriptor_ref: DESCRIPTOR_REF.to_string(),
        callback_generation: GENERATION,
    })
    .expect("begin dial")
    .next;
    apply_cross_process_session_command(&dialing, &CrossProcessSessionCommand::Established {
        observed_peer_context_ref: PEER_CONTEXT_REF.to_string(),
        callback_generation: GENERATION,
    })
    .expect("establish session")
    .next
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn exact_endpoint_exports_imports_and_redacts_default_status() {
    let descriptor = descriptor();
    let listener = ready_listener();
    let exported = plan_endpoint_export(
        &profile(),
        &protocol(),
        &descriptor,
        &listener,
        EndpointAdmissionState::fully_active(),
        OBSERVED_TICK,
    )
    .expect("endpoint export");
    assert_eq!(exported.descriptor, descriptor);
    assert_eq!(exported.status.locator_cohort_ref, LOCATOR_COHORT_REF);
    assert_eq!(exported.status.locator_classes, vec![EndpointLocatorClass::Ip, EndpointLocatorClass::Relay]);

    let dial = admit_endpoint_import(
        &profile(),
        &protocol(),
        &descriptor,
        &expected_binding(),
        EndpointAdmissionState::fully_active(),
    )
    .expect("endpoint import");
    assert_eq!(dial.public_endpoint_identity, ENDPOINT_IDENTITY);
    assert_eq!(dial.locators.len(), descriptor.locators.len());
    assert_eq!(dial.peer_context_ref, PEER_CONTEXT_REF);
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn endpoint_import_denies_exact_binding_validity_and_disclosure_mismatches() {
    let mut wrong_profile = expected_binding();
    wrong_profile.profile_id = "other-profile".to_string();
    let issues = admit_endpoint_import(
        &profile(),
        &protocol(),
        &descriptor(),
        &wrong_profile,
        EndpointAdmissionState::fully_active(),
    )
    .expect_err("wrong profile must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::ProfileIdentityMismatch));

    let mut wrong_peer = expected_binding();
    wrong_peer.peer_context_ref = CLEANUP_REF.to_string();
    let issues = admit_endpoint_import(
        &profile(),
        &protocol(),
        &descriptor(),
        &wrong_peer,
        EndpointAdmissionState::fully_active(),
    )
    .expect_err("wrong peer must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::PeerContextMismatch));

    let mut stale = expected_binding();
    stale.generation = STALE_GENERATION;
    let issues =
        admit_endpoint_import(&profile(), &protocol(), &descriptor(), &stale, EndpointAdmissionState::fully_active())
            .expect_err("stale generation must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::GenerationMismatch));

    let mut expired = expected_binding();
    expired.observed_tick = VALID_UNTIL_TICK;
    let issues =
        admit_endpoint_import(&profile(), &protocol(), &descriptor(), &expired, EndpointAdmissionState::fully_active())
            .expect_err("expired cohort must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::ValidityCohortExpired));

    let mut undisclosed = descriptor();
    undisclosed.disclosure.explicit_handoff_classes = vec![EndpointLocatorClass::Relay];
    let issues = validate_cross_process_endpoint(&profile(), &protocol(), &undisclosed)
        .expect_err("undeclared locator must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::UndeclaredLocatorClass));
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn endpoint_validation_denies_private_secret_handle_and_over_bound_inputs() {
    let mut private = descriptor();
    private.locators = vec![EndpointLocator {
        class: EndpointLocatorClass::Private,
        value: "private:127.0.0.1".to_string(),
    }];
    private.disclosure.explicit_handoff_classes = vec![EndpointLocatorClass::Private];
    let issues =
        validate_cross_process_endpoint(&profile(), &protocol(), &private).expect_err("private locator must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::PrivateLocatorDisclosure));

    let mut secret = descriptor();
    secret.locators[0].value = "ip:private-key-material".to_string();
    let issues = validate_cross_process_endpoint(&profile(), &protocol(), &secret)
        .expect_err("secret-bearing locator must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::SecretBearingLocator));

    let mut handle = descriptor();
    handle.locators[0].value = "ip:iroh::Endpoint".to_string();
    let issues = validate_cross_process_endpoint(&profile(), &protocol(), &handle).expect_err("raw handle must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::RawHandleLocator));

    let mut over_bound = descriptor();
    over_bound.resources.max_sessions = SESSION_LIMIT + 1;
    let issues = validate_cross_process_endpoint(&profile(), &protocol(), &over_bound)
        .expect_err("over-bound descriptor must deny");
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, CrossProcessTransportIssue::ResourceBoundExceeded("max-sessions")))
    );
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn endpoint_validation_denies_malformed_duplicate_and_unredacted_descriptors() {
    let mut malformed = descriptor();
    malformed.schema = "molten.fabric.transport.cross-process-endpoint.v0".to_string();
    malformed.service_id.clear();
    malformed.locators.push(malformed.locators[0].clone());
    malformed.disclosure.default_readback_redacted = false;
    let issues = validate_cross_process_endpoint(&profile(), &protocol(), &malformed)
        .expect_err("malformed descriptor must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::EndpointSchemaMismatch));
    assert!(issues.contains(&CrossProcessTransportIssue::EmptyField("endpoint-service-id")));
    assert!(issues.contains(&CrossProcessTransportIssue::DuplicateLocator));
    assert!(issues.contains(&CrossProcessTransportIssue::DefaultReadbackNotRedacted));

    let mut missing_locator = descriptor();
    missing_locator.locators.clear();
    let issues = validate_cross_process_endpoint(&profile(), &protocol(), &missing_locator)
        .expect_err("missing locator must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::MissingLocator));
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn endpoint_import_denies_protocol_owner_endpoint_listener_and_revocation_mismatches() {
    let mut wrong_protocol = expected_binding();
    wrong_protocol.protocol_id = "wrong-protocol".to_string();
    wrong_protocol.alpn = "wrong/alpn/1".to_string();
    let issues = admit_endpoint_import(
        &profile(),
        &protocol(),
        &descriptor(),
        &wrong_protocol,
        EndpointAdmissionState::fully_active(),
    )
    .expect_err("wrong protocol must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::ProtocolIdentityMismatch));
    assert!(issues.contains(&CrossProcessTransportIssue::AlpnMismatch));

    let mut wrong_owner = expected_binding();
    wrong_owner.extension_id = "wrong-extension".to_string();
    wrong_owner.service_id = "wrong-service".to_string();
    let issues = admit_endpoint_import(
        &profile(),
        &protocol(),
        &descriptor(),
        &wrong_owner,
        EndpointAdmissionState::fully_active(),
    )
    .expect_err("wrong owner must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::ExtensionIdentityMismatch));
    assert!(issues.contains(&CrossProcessTransportIssue::ServiceIdentityMismatch));

    let mut wrong_endpoint = expected_binding();
    wrong_endpoint.public_endpoint_identity = "iroh:wrongendpoint234567".to_string();
    wrong_endpoint.listener_identity_ref = CLEANUP_REF.to_string();
    let issues = admit_endpoint_import(
        &profile(),
        &protocol(),
        &descriptor(),
        &wrong_endpoint,
        EndpointAdmissionState::fully_active(),
    )
    .expect_err("wrong endpoint identity must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::EndpointIdentityMismatch));
    assert!(issues.contains(&CrossProcessTransportIssue::ListenerIdentityMismatch));

    for (admission, expected_issue) in [
        (
            EndpointAdmissionState {
                registration_active: false,
                ..EndpointAdmissionState::fully_active()
            },
            CrossProcessTransportIssue::RegistrationRevoked,
        ),
        (
            EndpointAdmissionState {
                transport_capability_active: false,
                ..EndpointAdmissionState::fully_active()
            },
            CrossProcessTransportIssue::TransportCapabilityRevoked,
        ),
        (
            EndpointAdmissionState {
                protocol_capability_active: false,
                ..EndpointAdmissionState::fully_active()
            },
            CrossProcessTransportIssue::ProtocolCapabilityRevoked,
        ),
        (
            EndpointAdmissionState {
                profile_active: false,
                ..EndpointAdmissionState::fully_active()
            },
            CrossProcessTransportIssue::ProfileRevoked,
        ),
    ] {
        let issues = admit_endpoint_import(&profile(), &protocol(), &descriptor(), &expected_binding(), admission)
            .expect_err("revoked admission must deny");
        assert!(issues.contains(&expected_issue));
    }
}

// r[verify molten.fabric_transport.cross_process_listener]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn listener_publication_is_atomic_and_revocation_stops_accepts() {
    let planned = planned_listener();
    let publication = plan_endpoint_export(
        &profile(),
        &protocol(),
        &descriptor(),
        &planned,
        EndpointAdmissionState::fully_active(),
        OBSERVED_TICK,
    )
    .expect_err("publication before readiness must deny");
    assert!(publication.contains(&CrossProcessTransportIssue::ListenerNotReady));

    let starting = apply_cross_process_listener_command(&planned, &CrossProcessListenerCommand::Start)
        .expect("start listener")
        .next;
    let mut incomplete = ListenerReadinessObservation::fully_ready();
    incomplete.exact_alpn_active = false;
    let issues = apply_cross_process_listener_command(&starting, &CrossProcessListenerCommand::MarkReady(incomplete))
        .expect_err("incomplete readiness must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::ListenerReadinessIncomplete));

    let ready = ready_listener();
    let accepted = apply_cross_process_listener_command(&ready, &CrossProcessListenerCommand::AcceptSession {
        callback_generation: GENERATION,
    })
    .expect("accept session")
    .next;
    let draining = apply_cross_process_listener_command(&accepted, &CrossProcessListenerCommand::BeginDrain {
        reason: ListenerDrainReason::RegistrationRevoked,
    })
    .expect("begin revoked drain")
    .next;
    assert_eq!(draining.phase, CrossProcessListenerPhase::Draining);
    assert_eq!(draining.terminal_class, Some(ListenerTerminalClass::Revoked));
    let issues = apply_cross_process_listener_command(&draining, &CrossProcessListenerCommand::AcceptSession {
        callback_generation: GENERATION,
    })
    .expect_err("draining listener must stop accepts");
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, CrossProcessTransportIssue::InvalidListenerTransition { .. }))
    );
}

// r[verify molten.fabric_transport.cross_process_listener]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn listener_fences_stale_callbacks_duplicate_owners_and_unclean_replacement() {
    let ready = ready_listener();
    let issues = apply_cross_process_listener_command(&ready, &CrossProcessListenerCommand::AcceptSession {
        callback_generation: STALE_GENERATION,
    })
    .expect_err("stale accept must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::StaleListenerCallback));

    let issues = plan_cross_process_listener(&profile(), &protocol(), &descriptor(), std::slice::from_ref(&ready))
        .expect_err("duplicate listener must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::DuplicateListener));

    let closing = apply_cross_process_listener_command(&ready, &CrossProcessListenerCommand::Close)
        .expect("close listener")
        .next;
    let cleaning = apply_cross_process_listener_command(&closing, &CrossProcessListenerCommand::BeginCleanup)
        .expect("begin cleanup")
        .next;
    let closed = apply_cross_process_listener_command(&cleaning, &CrossProcessListenerCommand::CompleteCleanup {
        cleanup_evidence_ref: CLEANUP_REF.to_string(),
    })
    .expect("complete cleanup")
    .next;
    let issues = apply_cross_process_listener_command(&closed, &CrossProcessListenerCommand::Replace {
        replacement_generation: REPLACEMENT_GENERATION,
        cleanup_evidence_ref: DESCRIPTOR_REF.to_string(),
    })
    .expect_err("replacement without matching cleanup must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::CleanupEvidenceRequired));
    let replaced = apply_cross_process_listener_command(&closed, &CrossProcessListenerCommand::Replace {
        replacement_generation: REPLACEMENT_GENERATION,
        cleanup_evidence_ref: CLEANUP_REF.to_string(),
    })
    .expect("replace listener")
    .next;
    assert_eq!(replaced.phase, CrossProcessListenerPhase::Replaced);
}

// r[verify molten.fabric_transport.cross_process_session]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn session_accounts_queue_submission_acknowledgement_and_cleanup() {
    let active = active_session();
    let queued = apply_cross_process_session_command(&active, &CrossProcessSessionCommand::QueueFrame {
        payload_bytes: FRAME_BYTES,
        callback_generation: GENERATION,
    })
    .expect("queue frame")
    .next;
    assert_eq!(queued.queued_bytes, FRAME_BYTES);
    let submitted = apply_cross_process_session_command(&queued, &CrossProcessSessionCommand::FrameSubmitted {
        payload_bytes: FRAME_BYTES,
        callback_generation: GENERATION,
    })
    .expect("submit frame")
    .next;
    assert_eq!(submitted.inflight_bytes, FRAME_BYTES);
    assert_eq!(submitted.delivery, DeliveryOutcome::Pending);
    let acknowledged = apply_cross_process_session_command(&submitted, &CrossProcessSessionCommand::AcknowledgeFrame {
        payload_bytes: FRAME_BYTES,
        callback_generation: GENERATION,
    })
    .expect("acknowledge frame")
    .next;
    assert_eq!(acknowledged.inflight_bytes, 0);
    assert_eq!(acknowledged.delivery, DeliveryOutcome::Delivered);
    assert_eq!(acknowledged.automatic_retry_count, 0);

    let closing = apply_cross_process_session_command(&acknowledged, &CrossProcessSessionCommand::Close)
        .expect("close session")
        .next;
    let cleaning = apply_cross_process_session_command(&closing, &CrossProcessSessionCommand::BeginCleanup)
        .expect("begin session cleanup")
        .next;
    let closed = apply_cross_process_session_command(&cleaning, &CrossProcessSessionCommand::CompleteCleanup {
        cleanup_evidence_ref: CLEANUP_REF.to_string(),
    })
    .expect("complete session cleanup")
    .next;
    assert!(closed.phase.is_terminal());
    let issues = apply_cross_process_session_command(&closed, &CrossProcessSessionCommand::Cancel)
        .expect_err("terminal callback must deny");
    assert_eq!(issues, vec![CrossProcessTransportIssue::StaleSessionCallback]);
}

// r[verify molten.fabric_transport.cross_process_session]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn session_role_binding_denies_client_accept_and_listener_dial_substitution() {
    let client = plan_cross_process_session(&dial_plan(), SESSION_REF, EndpointParticipantRole::Client)
        .expect("client session plan");
    let issues = apply_cross_process_session_command(&client, &CrossProcessSessionCommand::BeginAccept {
        observed_descriptor_ref: DESCRIPTOR_REF.to_string(),
        callback_generation: GENERATION,
    })
    .expect_err("client role cannot accept");
    assert!(issues.contains(&CrossProcessTransportIssue::ParticipantRoleMismatch));

    let listener = plan_cross_process_session(&dial_plan(), SESSION_REF, EndpointParticipantRole::Listener)
        .expect("listener session plan");
    let issues = apply_cross_process_session_command(&listener, &CrossProcessSessionCommand::BeginDial {
        observed_descriptor_ref: DESCRIPTOR_REF.to_string(),
        callback_generation: GENERATION,
    })
    .expect_err("listener role cannot dial");
    assert!(issues.contains(&CrossProcessTransportIssue::ParticipantRoleMismatch));
}

// r[verify molten.fabric_transport.cross_process_session]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn session_denies_stale_oversized_and_misaccounted_frames_and_preserves_uncertainty() {
    let active = active_session();
    let issues = apply_cross_process_session_command(&active, &CrossProcessSessionCommand::QueueFrame {
        payload_bytes: OVERSIZED_FRAME_BYTES,
        callback_generation: GENERATION,
    })
    .expect_err("oversized frame must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::SessionFrameLimitExceeded));

    let issues = apply_cross_process_session_command(&active, &CrossProcessSessionCommand::QueueFrame {
        payload_bytes: FRAME_BYTES,
        callback_generation: STALE_GENERATION,
    })
    .expect_err("stale frame callback must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::StaleSessionCallback));

    let queued = apply_cross_process_session_command(&active, &CrossProcessSessionCommand::QueueFrame {
        payload_bytes: FRAME_BYTES,
        callback_generation: GENERATION,
    })
    .expect("queue frame")
    .next;
    let issues = apply_cross_process_session_command(&queued, &CrossProcessSessionCommand::FrameSubmitted {
        payload_bytes: FRAME_BYTES + 1,
        callback_generation: GENERATION,
    })
    .expect_err("partial accounting mismatch must deny");
    assert!(issues.contains(&CrossProcessTransportIssue::SessionAccountingMismatch));

    let submitted = apply_cross_process_session_command(&queued, &CrossProcessSessionCommand::FrameSubmitted {
        payload_bytes: FRAME_BYTES,
        callback_generation: GENERATION,
    })
    .expect("submit frame")
    .next;
    let failed = apply_cross_process_session_command(&submitted, &CrossProcessSessionCommand::Fail {
        class: SessionTerminalClass::Disconnect,
        delivery_definitive: false,
    })
    .expect("classify disconnect")
    .next;
    assert_eq!(failed.delivery, DeliveryOutcome::Uncertain);
    assert_eq!(failed.retry, RetryDisposition::UnsafeWithoutReconciliation);
    assert_eq!(failed.automatic_retry_count, 0);
}

fn participant(role: EndpointParticipantRole, invocation_ref: &str) -> DistinctProcessParticipantEvidence {
    DistinctProcessParticipantEvidence {
        role,
        invocation_ref: invocation_ref.to_string(),
        parent_start_ref: PROFILE_REF.to_string(),
        terminal_ref: SESSION_REF.to_string(),
        cleanup_ref: CLEANUP_REF.to_string(),
        descriptor_ref: DESCRIPTOR_REF.to_string(),
        profile_id: profile().profile_id,
        protocol_id: protocol().protocol_id,
        alpn: protocol().alpn,
        service_id: protocol().service_id,
        generation: GENERATION,
        request_ref: AUTHORITY_REF.to_string(),
        payload_ref: VALIDITY_COHORT_REF.to_string(),
        acknowledgement_ref: VALIDITY_COHORT_REF.to_string(),
        parent_observed_start: true,
        parent_observed_terminal: true,
        parent_observed_exit: true,
        automatic_retry_count: 0,
    }
}

fn distinct_process_evidence() -> DistinctProcessTransportEvidenceInput {
    DistinctProcessTransportEvidenceInput {
        listener: participant(EndpointParticipantRole::Listener, LISTENER_REF),
        client: participant(EndpointParticipantRole::Client, LOCATOR_COHORT_REF),
        handoff_ref: PEER_CONTEXT_REF.to_string(),
        child_handles_distinct: true,
        handoff_observed_before_client_start: true,
        cleanup_succeeded: true,
        same_process_loopback: false,
        child_only_separation_claim: false,
        default_readback_redacted: true,
        payloads_excluded: true,
        accepted_sessions: 1,
        max_sessions: SESSION_LIMIT,
        exchanged_bytes: FRAME_BYTES,
        max_frame_bytes: FRAME_LIMIT,
    }
}

// r[verify molten.fabric_transport.distinct_process_evidence]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn matching_parent_observed_distinct_process_evidence_is_admitted() {
    let assessment = assess_distinct_process_transport_evidence(&distinct_process_evidence());
    assert!(assessment.admitted);
    assert!(assessment.issues.is_empty());
}

// r[verify molten.fabric_transport.distinct_process_evidence]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn same_process_child_only_stale_and_unclean_claims_are_denied() {
    let mut evidence = distinct_process_evidence();
    evidence.client.invocation_ref = evidence.listener.invocation_ref.clone();
    evidence.client.generation = REPLACEMENT_GENERATION;
    evidence.client.parent_observed_start = false;
    evidence.listener.parent_observed_exit = false;
    evidence.child_handles_distinct = false;
    evidence.handoff_observed_before_client_start = false;
    evidence.cleanup_succeeded = false;
    evidence.same_process_loopback = true;
    evidence.child_only_separation_claim = true;
    evidence.default_readback_redacted = false;
    evidence.payloads_excluded = false;
    evidence.client.automatic_retry_count = 1;
    let assessment = assess_distinct_process_transport_evidence(&evidence);
    assert!(!assessment.admitted);
    for expected in [
        DistinctProcessEvidenceIssue::DuplicateInvocation,
        DistinctProcessEvidenceIssue::GenerationMismatch,
        DistinctProcessEvidenceIssue::ParentStartMissing,
        DistinctProcessEvidenceIssue::ParentExitMissing,
        DistinctProcessEvidenceIssue::ChildHandlesNotDistinct,
        DistinctProcessEvidenceIssue::HandoffOrderInvalid,
        DistinctProcessEvidenceIssue::CleanupFailed,
        DistinctProcessEvidenceIssue::SameProcessLoopbackInsufficient,
        DistinctProcessEvidenceIssue::ChildOnlyClaimInsufficient,
        DistinctProcessEvidenceIssue::DefaultReadbackLeaksLocators,
        DistinctProcessEvidenceIssue::PayloadEvidenceLeak,
        DistinctProcessEvidenceIssue::AutomaticRetryObserved,
    ] {
        assert!(assessment.issues.contains(&expected), "missing issue {expected:?}");
    }
}
