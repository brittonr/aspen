mod failures;
mod support;

use crate::system_extension::NativeCallbackValuePort;

// r[verify molten.wasm_component_adoption.hostcalls]
// r[verify molten.wasm_component_adoption.validation]
#[test]
fn signed_request_reads_actual_bytes_and_replays_exactly() {
    let fixture = support::Fixture::new();
    let bytes = fixture.request(0);
    let signature = fixture.sign(&bytes);
    let services = fixture.services();
    let current = fixture.current(&services, &signature);
    let mut values = fixture.values();
    let outcome = fixture.session().invoke(&bytes, &current, &mut values).expect("admitted read");
    let request = crate::wasm_component::decode_host_request(&bytes).expect("request");
    let response = crate::wasm_component::decode_host_response(&outcome.response_bytes, &request).expect("response");
    assert_eq!(response.response().decision, "pass");
    assert_eq!(response.output(), Some(&fixture.output));
    let stored = values
        .materialize(&outcome.record.value_ref, crate::wasm_component::HOST_PAYLOAD_BYTES)
        .expect("completion was actually stored");
    assert_eq!(stored, outcome.record);
    let mut replay = super::Replay::new(fixture.scope.clone(), 3, 5).expect("replay");
    let replayed = replay
        .consume(super::ReplayInput {
            request_bytes: &bytes,
            expected_record_ref: &outcome.record.value_ref,
            record: &stored,
            response_bytes: &outcome.response_bytes,
        })
        .expect("exact playback");
    assert_eq!(replayed, response);
    let validation =
        crate::deterministic_replay::validate_effect_log(crate::deterministic_replay::EffectLogValidationInput {
            expected_run_identity_ref: &fixture.scope.run_ref,
            expected_handler_profile_ref: &fixture.scope.handler_profile_ref,
            entries: std::slice::from_ref(&outcome.entry),
            consumed: &[crate::deterministic_replay::ConsumedEffect {
                sequence: 0,
                effect_kind: "blob-get".to_string(),
                request_ref: request.request_ref().to_string(),
                response_ref: outcome.entry.response_ref.clone(),
                boundary_ref: fixture.scope.boundary_ref.clone(),
                used_live_fallback: false,
            }],
        })
        .expect("existing effect log validator");
    assert_eq!(validation.decision, "pass");
}

// r[verify molten.wasm_component_adoption.hostcalls]
#[test]
fn holder_signature_nonce_and_current_time_deny_before_blob_access() {
    for mode in ["signature", "expired", "not-yet-valid", "nonce"] {
        let fixture = support::Fixture::new();
        let bytes = fixture.request(0);
        let original = fixture.sign(&bytes);
        let signature = if mode == "signature" {
            basalt::ucan::SignerSignature::from_bytes([0; 64])
        } else {
            original
        };
        let mut services = fixture.services();
        if mode == "expired" {
            services.verification_time = basalt::ucan::VerificationTime::from_unix_seconds(201);
        }
        if mode == "not-yet-valid" {
            services.verification_time = basalt::ucan::VerificationTime::from_unix_seconds(99);
        }
        let mut values = failures::Observed::new(fixture.values());
        let mut session = fixture.session();
        if mode == "nonce" {
            session.invoke(&bytes, &fixture.current(&services, &signature), &mut values).expect("first read");
        }
        let denied_bytes = fixture.request(if mode == "nonce" { 1 } else { 0 });
        let second_signature = fixture.sign(&denied_bytes);
        let selected = if mode == "nonce" { &second_signature } else { &signature };
        let before = values.reads;
        let denied = session
            .invoke(&denied_bytes, &fixture.current(&services, selected), &mut values)
            .expect("deny record");
        let request = crate::wasm_component::decode_host_request(&denied_bytes).expect("denied request");
        let response =
            crate::wasm_component::decode_host_response(&denied.response_bytes, &request).expect("deny response");
        assert_eq!(response.response().decision, "deny", "{mode}");
        assert_eq!(response.output(), None, "{mode}");
        assert_eq!(response.response().diagnostics, vec!["authority-denial"], "{mode}");
        assert_eq!(values.reads, before, "{mode} must not fetch a blob");
    }
}

// r[verify molten.wasm_component_adoption.boundary]
#[test]
fn stale_generation_resources_and_capabilities_never_reach_the_blob_port() {
    for mode in [
        "generation",
        "incarnation",
        "resources",
        "capabilities",
        "policy",
        "issuer",
        "holder",
    ] {
        let fixture = support::Fixture::new();
        let bytes = fixture.request(0);
        let signature = fixture.sign(&bytes);
        let services = fixture.services();
        let mut current = fixture.current(&services, &signature);
        match mode {
            "generation" => current.generation += 1,
            "incarnation" => current.incarnation += 1,
            "resources" => current.limits.max_output_bytes -= 1,
            "capabilities" => current.capability_refs = &[],
            "policy" => current.authority.policy_json = "{}",
            "issuer" => current.authority.trusted_issuer = "did:key:untrusted",
            "holder" => current.authority.expected_holder = "did:key:another-holder",
            _ => unreachable!("closed cases"),
        }
        let mut values = failures::Observed::new(fixture.values());
        let denied = fixture.session().invoke(&bytes, &current, &mut values).expect("deny record");
        let request = crate::wasm_component::decode_host_request(&bytes).expect("request");
        let response = crate::wasm_component::decode_host_response(&denied.response_bytes, &request).expect("denial");
        assert_eq!(response.response().decision, "deny", "{mode}");
        assert_eq!(values.reads, 0, "{mode}");
    }
}

// r[verify molten.wasm_component_adoption.hostcalls]
#[test]
fn replay_stops_on_scope_identity_and_materialized_response_divergence() {
    let fixture = support::Fixture::new();
    let bytes = fixture.request(0);
    let signature = fixture.sign(&bytes);
    let services = fixture.services();
    let outcome = fixture
        .session()
        .invoke(&bytes, &fixture.current(&services, &signature), &mut fixture.values())
        .expect("recorded read");
    for mode in [
        "run",
        "handler",
        "turn",
        "boundary",
        "generation",
        "incarnation",
        "record",
        "request",
        "response",
    ] {
        let mut scope = fixture.scope.clone();
        let changed = support::reference("changed");
        match mode {
            "run" => scope.run_ref = changed,
            "handler" => scope.handler_profile_ref = changed,
            "turn" => scope.turn_ref = changed,
            "boundary" => scope.boundary_ref = changed,
            _ => {}
        }
        let mut replay = super::Replay::new(
            scope,
            if mode == "generation" { 4 } else { 3 },
            if mode == "incarnation" { 6 } else { 5 },
        )
        .expect("replay");
        let alternate = fixture.request(1);
        let response = if mode == "response" {
            &bytes
        } else {
            &outcome.response_bytes
        };
        let expected = support::reference("wrong-record");
        let input = || super::ReplayInput {
            request_bytes: if mode == "request" { &alternate } else { &bytes },
            expected_record_ref: if mode == "record" {
                &expected
            } else {
                &outcome.record.value_ref
            },
            record: &outcome.record,
            response_bytes: response,
        };
        assert!(replay.consume(input()).is_err(), "{mode}");
        assert!(
            replay
                .consume(super::ReplayInput {
                    request_bytes: &bytes,
                    expected_record_ref: &outcome.record.value_ref,
                    record: &outcome.record,
                    response_bytes: &outcome.response_bytes,
                })
                .is_err(),
            "{mode} cannot resume after divergence"
        );
    }
}

// r[verify molten.wasm_component_adoption.hostcalls]
#[test]
fn current_basalt_policy_can_deny_a_valid_signed_ucan_without_reading() {
    let mut policy: serde_json::Value = serde_json::from_str(support::POLICY).expect("exported policy");
    policy["contracts"]["component-blob-get"]["abilities"] = serde_json::json!(["blob/put"]);
    let fixture = support::Fixture::with_policy(serde_json::to_string(&policy).expect("current policy"));
    let bytes = fixture.request(0);
    let signature = fixture.sign(&bytes);
    let services = fixture.services();
    let mut values = failures::Observed::new(fixture.values());
    let outcome = fixture
        .session()
        .invoke(&bytes, &fixture.current(&services, &signature), &mut values)
        .expect("policy denial is recorded");
    let request = crate::wasm_component::decode_host_request(&bytes).expect("request");
    let response = crate::wasm_component::decode_host_response(&outcome.response_bytes, &request).expect("denial");
    assert_eq!(response.response().decision, "deny");
    assert_eq!(response.output(), None);
    assert_eq!(values.reads, 0);
}
