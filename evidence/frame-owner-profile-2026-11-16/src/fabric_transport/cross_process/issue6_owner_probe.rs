// Task-owned diagnostic test; only included in the staged-index-derived scratch copy.
// Network endpoints and canonical setup reuse the existing real loopback fixtures.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn issue6_owner_held_real_delivered_profile() {
    use crate::system_extension::FabricEffectPort;
    use std::io::Write;
    use std::time::{Duration, Instant};

    const WARMUPS: usize = 3;
    const TRIALS: usize = 20;
    const TYPICAL_BYTES: usize = 1024;
    let output_path = std::env::var_os("ISSUE6_SAMPLES").expect("ISSUE6_SAMPLES must name a new file");
    let mut output_file = std::fs::OpenOptions::new().write(true).create_new(true)
        .open(output_path).expect("exclusive sample file");
    let ns = |duration: Duration| u64::try_from(duration.as_nanos()).expect("bounded elapsed duration");
    let max_bytes = usize::try_from(profile().profile.limits.max_frame_bytes).expect("maximum usize");
    assert_eq!(max_bytes, 4096, "predeclared fixture maximum changed; reject profile drift");
    assert_eq!(protocol().framing.max_frame_bytes, max_bytes as u64);
    assert!(TYPICAL_BYTES < max_bytes);

    // Fresh canonical owner/listener per operation avoids replay/session credit accumulation.
    // Alternation prevents every maximum-size observation following the typical-size cohort.
    for trial in 0..WARMUPS + TRIALS {
        let sizes = if trial % 2 == 0 { [TYPICAL_BYTES, max_bytes] } else { [max_bytes, TYPICAL_BYTES] };
        for payload_len in sizes {
            let listener = listener().await;
            let endpoint = listener.handoff().clone();
            assert_eq!(endpoint.descriptor.resources.max_frame_bytes, max_bytes as u64,
                "published endpoint must admit the predeclared actual maximum");
            assert!(endpoint.descriptor.locators.iter().all(|locator| {
                locator.class == EndpointLocatorClass::Ip && locator.value.starts_with("ip:127.0.0.1:")
            }), "fixture must publish only task-owned IPv4 loopback locators");
            let descriptor_ref = endpoint.descriptor_ref.clone();
            let listener_task = tokio::spawn(accept_one_and_drain(listener));
            let live_profile = profile();
            let profile_ref = live_profile.profile_ref.clone();
            let profile_canonical = crate::preserves_rail::to_text(&live_profile.value).expect("profile text");
            let context = ExtensionContext::from_test_snapshot(&protocol().service_id, GENERATION, &live_profile);
            let client = IrohClientConfig {
                capability: capability(CLIENT_SECRET_BYTE, CLIENT_CAPABILITY_REF),
                bind_addr: bind_addr(),
                expected: expected(&endpoint),
                endpoint,
                admission: EndpointAdmissionState::fully_active(),
                timeout: Duration::from_secs(TEST_TIMEOUT_SECONDS),
            };
            let mut port = Registered::new(context, live_profile.clone(), protocol(), client).expect("live owner");
            let mut reference = IrohAdapter::new(live_profile.clone()).expect("canonical comparison adapter");
            let binding = effect_binding(&live_profile);
            let (session_id, stream_id) = effect_port_ids();
            for (request_ref, operation, command) in setup_commands(&session_id, &stream_id) {
                let canonical = reference.execute_command(&command).expect("reference setup");
                assert_eq!(canonical.decision, TransportTransitionDecision::Applied);
                port.register(request_ref.to_string(), command, None).expect("register setup");
                let effect = effect_request(&binding, operation, request_ref, 0);
                let actual = port.route(&binding, &effect).expect("route setup");
                assert_eq!(actual.output_ref, canonical.transition_ref);
            }
            let payload: Vec<u8> = (0..payload_len).map(|index| (index % 251) as u8).collect();
            let payload_ref = frame_ref(REQUEST_REF, &payload);
            let payload_bytes = payload.len() as u64;
            let send = TransportCommand::SendFrame {
                operation_id: REQUEST_REF.to_string(),
                session_id: session_id.clone(),
                stream_id: stream_id.clone(),
                payload_ref: payload_ref.clone(),
                payload_bytes,
                observed_tick: OBSERVED_TICK,
            };
            let send_for_port = send.clone();
            let registered_payload = payload.clone();
            let request_ref = REQUEST_REF.to_string();
            let effect = effect_request(&binding, "send-frame", REQUEST_REF, payload_bytes);
            super::effect_port::issue6_probe::reset();
            let whole_start = Instant::now();
            let register_start = Instant::now();
            let registered = port.register(request_ref, send_for_port, Some(registered_payload));
            let register_call_ns = ns(register_start.elapsed());
            registered.expect("register actual admitted frame, not a denial");
            let route_start = Instant::now();
            let routed = port.route(&binding, &effect);
            let route_call_ns = ns(route_start.elapsed());
            let whole_operation_ns = ns(whole_start.elapsed());
            let actual = routed.expect("actual real route");
            let owner_held_ns = register_call_ns.checked_add(route_call_ns).expect("owner sum");

            // Await server completion before collecting server-side hash instrumentation.
            // This await and all assertions/serialization are outside the owner-held timers.
            let (server, cleanup) = listener_task.await.expect("real listener task");
            let client = port.latest_frame_evidence().cloned().expect("actual client ACK evidence");
            for evidence in [&client, &server] {
                assert_eq!(evidence.delivery, DeliveryOutcome::Delivered);
                assert_eq!(evidence.retry, RetryDisposition::NotApplicable);
                assert_eq!(evidence.automatic_retry_count, 0);
                assert_eq!(evidence.request_ref, REQUEST_REF);
                assert_eq!(evidence.session_ref, SESSION_REF);
                assert_eq!(evidence.payload_ref, payload_ref);
                assert_eq!(evidence.acknowledgement_ref, payload_ref);
                assert_eq!(evidence.payload_bytes, payload_bytes);
            }
            assert_eq!(client.role, EndpointParticipantRole::Client);
            assert_eq!(server.role, EndpointParticipantRole::Listener);
            assert_eq!(cleanup.terminal_class, ListenerTerminalClass::Clean);
            assert_eq!(port.adapter().state().sessions[SESSION_REF].inflight_bytes, 0);

            // A direct canonical reference is only a comparison oracle, never the measured path.
            // Actual route receipt must be the canonical acknowledged receipt, not merely any hash.
            let pending = reference.execute_command(&send).expect("reference SendFrame");
            assert_eq!(pending.decision, TransportTransitionDecision::Applied);
            assert_eq!(pending.events[0].event.delivery, DeliveryOutcome::Pending);
            let acknowledged = reference.execute_command(&TransportCommand::AcknowledgeFrame {
                operation_id: REQUEST_REF.to_string(), session_id, stream_id, payload_bytes,
            }).expect("reference AcknowledgeFrame");
            assert_eq!(acknowledged.decision, TransportTransitionDecision::Applied);
            assert_eq!(acknowledged.events[0].event.delivery, DeliveryOutcome::Delivered);
            assert_eq!(actual.output_ref, acknowledged.transition_ref, "actual receipt must acknowledge Delivery");
            assert_eq!(port.adapter().state(), reference.state());
            let stages = super::effect_port::issue6_probe::take();
            for required in [
                "register_validation_before_hash_ns", "register_validation_after_hash_ns", "register_hash_ns",
                "register_copy_ns", "register_body_ns", "route_body_ns", "submit_transition_ns",
                "exchange_total_ns", "exchange_input_copy_ns", "worker_dispatch_ns", "runtime_setup_ns",
                "exchange_block_on_ns", "runtime_shutdown_ns", "evidence_matching_ns", "ack_transition_ns",
                "client_frame_hash_ns", "server_frame_hash_ns",
            ] {
                assert!(stages.contains_key(required), "missing stage {required}; do not treat denial as a sample");
            }
            let row = serde_json::json!({
                "schema": "aspen.issue6.owner-profile.v1",
                "source_tree": "bae6acf4fa500369ca25cd7a1e094add419efe53",
                "profile_kind": "diagnostic-native-loopback-current-per-frame-runtime",
                "production_representative_runtime_reuse": false,
                "phase": if trial < WARMUPS { "warmup" } else { "measured" },
                "trial": if trial < WARMUPS { trial } else { trial - WARMUPS },
                "size_class": if payload_len == TYPICAL_BYTES { "typical-assumed" } else { "admitted-maximum" },
                "payload_bytes": payload_bytes, "max_admitted_frame_bytes": max_bytes,
                "register_call_ns": register_call_ns, "route_call_ns": route_call_ns,
                "owner_held_ns": owner_held_ns, "whole_operation_ns": whole_operation_ns,
                "register_hash_owner_fraction": stages["register_hash_ns"] as f64 / owner_held_ns as f64,
                "stages_ns": stages,
                "profile_ref": profile_ref, "profile_canonical": profile_canonical,
                "endpoint_descriptor_ref": descriptor_ref,
                "request_ref": REQUEST_REF, "payload_ref": payload_ref,
                "client_ack_ref": client.acknowledgement_ref, "server_ack_ref": server.acknowledgement_ref,
                "actual_ack_transition_ref": actual.output_ref,
                "canonical_pending_transition_ref": pending.transition_ref,
                "canonical_ack_transition_ref": acknowledged.transition_ref,
                "canonical_ack_receipt": crate::preserves_rail::to_text(&acknowledged.value).expect("ack receipt text"),
                "checks": {"real_client_delivered": true, "real_server_delivered": true,
                    "request_bound_ack_matches": true, "zero_inflight_bytes": true,
                    "actual_ack_receipt_matches_canonical": true, "actual_state_matches_canonical": true,
                    "clean_listener_cleanup": true, "automatic_retry_count": 0},
                "decision": "adoption-gate-closed-missing-production-runtime-reuse-baseline"
            });
            serde_json::to_writer(&mut output_file, &row).expect("write sample");
            writeln!(output_file).expect("sample newline");
            output_file.flush().expect("flush sample");
        }
    }
}
