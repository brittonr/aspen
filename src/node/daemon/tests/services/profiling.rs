//! Live profile preflight and receipt tests.

fn live_profile_send_case()
-> (std::path::PathBuf, super::super::entitlements::ControlLiveTicket, super::transfers::SendMaterial) {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
    runtime.block_on(async {
        let (root, identity) = super::transfers::init_send_case();
        let state_root = crate::node_state::DirectoryRoot::open(&root).expect("open node state root");
        let lookup = iroh::address_lookup::memory::MemoryLookup::new();
        let receiver_endpoint = super::super::endpoints::live_gossip_endpoint(
            &lookup,
            Some(
                super::super::endpoints::stable_live_endpoint_secret(&state_root, &identity).expect("transport secret"),
            ),
        )
        .await
        .expect("receiver endpoint");
        let receiver_addr = receiver_endpoint.addr();
        let ticket = super::super::endpoints::live_ticket_for_bound_endpoint(
            &state_root,
            &identity,
            super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            &receiver_addr,
        )
        .expect("live ticket");
        let material = super::transfers::send_material(&root, &ticket);
        (root, ticket, material)
    })
}

#[test]
fn live_profiles_preflight_passes_and_receipt_binds_refs() {
    let (root, ticket, material) = live_profile_send_case();
    let topology_ref = super::super::storage::local_ref("live-topology-profile", "send").expect("topology ref");
    let transport_ref = super::super::storage::local_ref("live-transport-profile", "send").expect("transport ref");
    let admitted_ticket_refs = vec![ticket.ticket_ref.clone()];
    let admitted_peer_refs = vec![material.admission.admission_ref.clone()];
    let allowed_alpns = [super::super::settings::LIVE_CONTROL_INGRESS_TRANSPORT];
    let topology = super::super::offers::LiveTopologyProfile {
        profile_ref: &topology_ref,
        expected_node: "node:live-send",
        expected_peer: "peer:external-send",
        expected_topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        expected_endpoint: Some(&ticket.live_endpoint_id),
        allowed_alpns: &allowed_alpns,
        ticket_refs: &admitted_ticket_refs,
        peer_admission_refs: &admitted_peer_refs,
        role: Some("control-live-send"),
    };
    let profiled_attempts = super::super::settings::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS + 1;
    const PROFILE_TIMEOUT_DIVISOR: u64 = 2;
    let profiled_timeout_ms = super::super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS / PROFILE_TIMEOUT_DIVISOR;
    let transport = super::super::offers::LiveTransportProfile {
        profile_ref: &transport_ref,
        max_attempts: profiled_attempts,
        join_timeout_ms: profiled_timeout_ms,
        publish_timeout_ms: profiled_timeout_ms,
        relay_preference: super::super::shaping::LIVE_PROFILE_RELAY_AUTO,
    };
    let mut input = super::transfers::build_send_input(&root, &ticket, &material);
    input.topology_profile = Some(&topology);
    input.transport_profile = Some(&transport);

    let profile = super::super::shaping::preflight_live_profiles(&input).expect("profile preflight");
    assert_eq!(profile.decision, "pass");
    assert_eq!(profile.topology_profile_ref.as_deref(), Some(topology_ref.as_str()));
    assert_eq!(profile.transport_profile_ref.as_deref(), Some(transport_ref.as_str()));
    assert_eq!(profile.effective_max_attempts, profiled_attempts);
    assert_eq!(profile.effective_join_timeout_ms, profiled_timeout_ms);
    assert!(profile.diagnostics.is_empty());

    let envelope = super::super::vetting::send_envelope(&input, &ticket).expect("send envelope");
    let receipt = super::super::sends::live_send_receipt_value(&super::super::sends::LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.from_peer,
        ticket: &ticket,
        envelope: &envelope,
        transport_receipt_ref: None,
        topology_profile_ref: super::super::tuning::selected_topology_profile_ref(&input),
        transport_profile_ref: super::super::tuning::selected_transport_profile_ref(&input),
        effective_max_attempts: super::super::tuning::effective_live_send_max_attempts(&input),
        effective_join_timeout_ms: super::super::tuning::effective_live_send_join_timeout_ms(&input),
        diagnostics: &[],
    })
    .expect("send receipt");
    let text = crate::preserves_rail::to_text(&receipt).expect("receipt text");
    assert!(text.contains("live-profiles"));
    assert!(text.contains(&topology_ref));
    assert!(text.contains(&transport_ref));
    assert!(text.contains(&profiled_attempts.to_string()));
    assert!(text.contains(&profiled_timeout_ms.to_string()));

    assert_transport_receipt_binds_profiles(&input, &envelope, &topology_ref, &transport_ref);
}

/// The publish transport receipt carries the selected live topology and transport profile refs.
fn assert_transport_receipt_binds_profiles(
    input: &super::super::offers::ControlLiveSendInput<'_>,
    envelope: &super::super::transits::ControlIngressEnvelope,
    topology_ref: &str,
    transport_ref: &str,
) {
    let transport_receipt = super::super::transports::live_transport_receipt_value(
        &super::super::transports::LiveTransportReceiptValueInput {
            operation: "publish",
            decision: "pass",
            node_id: input.from_peer,
            delivered_from: None,
            envelope,
            ingress_receipt_ref: None,
            topology_profile_ref: super::super::tuning::selected_topology_profile_ref(input),
            transport_profile_ref: super::super::tuning::selected_transport_profile_ref(input),
            effective_max_attempts: Some(super::super::tuning::effective_live_send_max_attempts(input)),
            effective_join_timeout_ms: Some(super::super::tuning::effective_live_send_join_timeout_ms(input)),
            diagnostics: &[],
        },
    )
    .expect("transport receipt");
    let transport_text = crate::preserves_rail::to_text(&transport_receipt).expect("transport text");
    assert!(transport_text.contains("live-profiles"));
    assert!(transport_text.contains(topology_ref));
    assert!(transport_text.contains(transport_ref));
}

#[test]
fn live_profiles_no_profile_receipt_records_explicit_caveat() {
    let (root, ticket, material) = live_profile_send_case();
    let input = super::transfers::build_send_input(&root, &ticket, &material);
    let envelope = super::super::vetting::send_envelope(&input, &ticket).expect("send envelope");
    let receipt = super::super::sends::live_send_receipt_value(&super::super::sends::LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.from_peer,
        ticket: &ticket,
        envelope: &envelope,
        transport_receipt_ref: None,
        topology_profile_ref: super::super::tuning::selected_topology_profile_ref(&input),
        transport_profile_ref: super::super::tuning::selected_transport_profile_ref(&input),
        effective_max_attempts: super::super::tuning::effective_live_send_max_attempts(&input),
        effective_join_timeout_ms: super::super::tuning::effective_live_send_join_timeout_ms(&input),
        diagnostics: &[],
    })
    .expect("send receipt");
    let text = crate::preserves_rail::to_text(&receipt).expect("receipt text");
    assert!(text.contains("explicit-flags-no-profile"));
    assert!(text.contains(&super::super::settings::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS.to_string()));
    assert!(text.contains(&input.join_timeout_ms.to_string()));
}

#[test]
fn live_profiles_preflight_denies_mismatched_caps_and_non_authority() {
    let (root, ticket, material) = live_profile_send_case();
    let topology_ref = super::super::storage::local_ref("live-topology-profile", "wrong").expect("topology ref");
    let transport_ref = super::super::storage::local_ref("live-transport-profile", "over-cap").expect("transport ref");
    let admitted_ticket_refs = vec![ticket.ticket_ref.clone()];
    let admitted_peer_refs = vec![material.admission.admission_ref.clone()];
    let wrong_alpns = ["not-live-gossip"];
    let wrong_endpoint = "node:wrong-live-endpoint";
    let wrong_peer = "peer:wrong-external-send";
    let wrong_topic = "node-control-wrong-topic";
    let topology = super::super::offers::LiveTopologyProfile {
        profile_ref: &topology_ref,
        expected_node: "node:live-send",
        expected_peer: wrong_peer,
        expected_topic: wrong_topic,
        expected_endpoint: Some(wrong_endpoint),
        allowed_alpns: &wrong_alpns,
        ticket_refs: &admitted_ticket_refs,
        peer_admission_refs: &admitted_peer_refs,
        role: Some("control-live-send"),
    };
    let over_attempt_cap = super::super::settings::MAX_CONTROL_LIVE_SEND_ATTEMPTS + 1;
    let over_timeout_cap = super::super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS + 1;
    let transport = super::super::offers::LiveTransportProfile {
        profile_ref: &transport_ref,
        max_attempts: over_attempt_cap,
        join_timeout_ms: over_timeout_cap,
        publish_timeout_ms: over_timeout_cap,
        relay_preference: super::super::shaping::LIVE_PROFILE_RELAY_AUTO,
    };
    let empty_refs: Vec<String> = Vec::new();
    let mut input = super::transfers::build_send_input(&root, &ticket, &material);
    input.topology_profile = Some(&topology);
    input.transport_profile = Some(&transport);
    input.peer_bootstrap_refs = &empty_refs;
    input.authority_refs = &empty_refs;
    input.policy_refs = &empty_refs;
    input.resource_refs = &empty_refs;

    let profile = super::super::shaping::preflight_live_profiles(&input).expect("profile preflight");
    assert_eq!(profile.decision, "deny");
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("peer")));
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("topic")));
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("endpoint")));
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("ALPN")));
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("attempts")));
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("join timeout")));
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("operation authority")));
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("policy admission")));
    assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource authority")));
}
