use super::*;
use crate::fabric_consistency::raft::CommittedBatchHandler;

fn declared_current(application: &Application) -> CurrentState {
    CurrentState {
        currentness: application.state().currentness.clone(),
        state: application.state_ref().clone(),
    }
}

// r[verify molten.collaboration_scope.validation]
#[test]
fn real_signed_mutation_is_durable_and_identical_replay_retains_original_receipt() {
    let workspace = crate::test_support::process_workspace("collaboration-authority-durability").expect("workspace");
    let store_root = workspace.join("scope");
    let values_root = workspace.join("values");
    let replay_root = workspace.join("replay");
    let mut application = fixtures::open_application(&store_root, &values_root).expect("real application");
    let authority = fixtures::Signers::new();
    let replay = NonceLedger::new(fixtures::open_store(&replay_root).expect("real replay storage"));
    let before = application.state_ref().clone();
    let current = declared_current(&application);
    let mutation = fixtures::add_member(application.state(), "member-one").expect("membership request");
    let command = authority
        .stage(&mut application, current, mutation, &replay)
        .expect("actual UCAN and Basalt admission");
    let receipt = application.apply_batch(std::slice::from_ref(&command)).expect("actual durable commit");
    let committed = application.state_ref().clone();
    assert_ne!(committed, before);
    assert_eq!(application.state().scopes[&fixtures::space_ref()].membership_epoch, 2);
    assert_eq!(application.apply_batch(std::slice::from_ref(&command)).expect("exact replay"), receipt);
    drop(application);
    let mut reopened = fixtures::open_application(&store_root, &values_root).expect("reopen actual Redb state");
    assert_eq!(reopened.state_ref(), &committed);
    assert_eq!(reopened.apply_batch(std::slice::from_ref(&command)).expect("restart replay"), receipt);
    assert_eq!(reopened.state_ref(), &committed);
}

// r[verify molten.collaboration_scope.validation]
#[test]
fn independently_signed_conflicting_mutations_are_serialized_into_one_allow_and_one_deny() {
    let workspace = crate::test_support::process_workspace("collaboration-ordered-conflict").expect("workspace");
    let mut application =
        fixtures::open_application(&workspace.join("scope"), &workspace.join("values")).expect("application");
    let authority = fixtures::Signers::new();
    let replay = NonceLedger::new(fixtures::open_store(&workspace.join("replay")).expect("replay storage"));
    let first_current = declared_current(&application);
    let second_current = declared_current(&application);
    let first = fixtures::add_member(application.state(), "first-member").expect("first request");
    let second = fixtures::add_member(application.state(), "second-member").expect("concurrent request");
    let first = authority.stage(&mut application, first_current, first, &replay).expect("first real signature");
    let second = authority.stage(&mut application, second_current, second, &replay).expect("second real signature");
    application.apply_batch(&[first, second]).expect("ordered application with canonical denial");
    let scope = &application.state().scopes[&fixtures::space_ref()];
    assert_eq!(scope.membership_epoch, 2);
    assert_eq!(scope.members.len(), 2);
    assert!(
        scope
            .members
            .contains_key(&SubjectRef::new(fixtures::reference("subject-first-member")).expect("subject"))
    );
    assert!(
        !scope
            .members
            .contains_key(&SubjectRef::new(fixtures::reference("subject-second-member")).expect("subject"))
    );
    assert_eq!(application.state().currentness.committed_index, 2);
}

// r[verify molten.collaboration_scope.validation]
#[test]
fn strict_projection_rejects_unknown_payload_and_raw_identity_fields() {
    let state = fixtures::initial_state().expect("declared state");
    let mut value = state_value(&state);
    let mut fields: Vec<_> = value
        .collect_simple_record("collaboration-state-v1", None)
        .expect("state record")
        .fields_iter()
        .map(crate::preserves_rail::value_to_iovalue)
        .collect();
    fields.push(crate::preserves_rail::string("raw-token-or-resource-bytes"));
    value = crate::preserves_rail::record("collaboration-state-v1", fields);
    assert!(parse_state(&value).is_err());
    let scope = state.scopes.values().next().expect("scope");
    let mut fields: Vec<_> = space_value(scope)
        .collect_simple_record("collaboration-scope-v1", None)
        .expect("scope record")
        .fields_iter()
        .map(crate::preserves_rail::value_to_iovalue)
        .collect();
    fields[2] = crate::preserves_rail::string("did:key:unadmitted-identity-document");
    assert!(parse_space(&crate::preserves_rail::record("collaboration-scope-v1", fields)).is_err());
}

// r[verify molten.collaboration_scope.validation]
#[test]
fn real_verifier_denies_invalid_signature_expiry_revocation_and_wrong_ability_before_state_effects() {
    let workspace = crate::test_support::process_workspace("collaboration-verifier-negatives").expect("workspace");
    let mut application =
        fixtures::open_application(&workspace.join("scope"), &workspace.join("values")).expect("application");
    let authority = fixtures::Signers::new();
    let replay = NonceLedger::new(fixtures::open_store(&workspace.join("replay")).expect("durable replay"));
    let original = application.state().clone();
    for fault in [
        fixtures::TokenFault::Signature,
        fixtures::TokenFault::Expired,
        fixtures::TokenFault::Revoked,
        fixtures::TokenFault::WrongAbility,
    ] {
        let current = declared_current(&application);
        let mutation = fixtures::add_member(application.state(), "verified-member").expect("request");
        assert!(
            authority
                .stage_case(
                    &mut application,
                    current,
                    fixtures::StageCase {
                        mutation,
                        fault: Some(fault)
                    },
                    &replay
                )
                .is_err()
        );
        assert_eq!(application.state(), &original);
    }
    let current = declared_current(&application);
    let mutation = fixtures::add_member(application.state(), "verified-member").expect("request");
    let command = authority
        .stage(&mut application, current, mutation, &replay)
        .expect("failed verifications did not spend nonce");
    application.apply_batch(&[command]).expect("later valid commit");
    assert_eq!(application.state().scopes[&fixtures::space_ref()].membership_epoch, 2);
}

// r[verify molten.collaboration_scope.validation]
// r[verify molten.collaboration_scope.authority_snapshot.replay]
#[test]
fn actual_allowed_audience_is_not_replayable_and_membership_commit_invalidates_its_snapshot() {
    let workspace = crate::test_support::process_workspace("collaboration-safe-snapshot").expect("workspace");
    let mut application =
        fixtures::open_application(&workspace.join("scope"), &workspace.join("values")).expect("application");
    let authority = fixtures::Signers::new();
    let replay = NonceLedger::new(fixtures::open_store(&workspace.join("replay")).expect("replay"));
    let snapshot = authority
        .snapshot(&application, declared_current(&application), &replay)
        .expect("actual read admission");
    let bytes = crate::preserves_rail::canonical_bytes(&snapshot_value(&snapshot)).expect("snapshot bytes");
    assert_eq!(decode_snapshot(&bytes).expect("closed safe projection"), snapshot);
    assert!(authority.snapshot(&application, declared_current(&application), &replay).is_err());
    let current = declared_current(&application);
    let mutation = fixtures::add_member(application.state(), "later-member").expect("membership request");
    let command = authority.stage(&mut application, current, mutation, &replay).expect("exact mutation authority");
    application.apply_batch(&[command]).expect("membership commit");
    assert_eq!(
        revalidate_snapshot(application.state(), application.state_ref(), &snapshot, &snapshot.decisions),
        Err(Denial::Stale)
    );
}

/// A canonical callback envelope the native bridge can observe without a host process.
fn bridge_envelope(
    context: &crate::system_extension::CallbackContext,
    callback: crate::system_extension::CallbackKind,
    incarnation: u64,
    sequence: u64,
    state: Option<crate::system_extension::NativeCallbackValue>,
) -> crate::system_extension::DecodedCallbackEnvelope {
    let reference = |label: &str| crate::preserves_rail::content_ref_from_bytes(label.as_bytes());
    let invocation = crate::system_extension::CallbackInvocation {
        callback,
        generation: 1,
        incarnation,
        sequence,
        event_ref: reference(&format!("{}-{}", callback.as_str(), sequence)),
        payload_ref: None,
        logical_tick: sequence,
        deadline_tick: sequence + 100,
    };
    let inputs = crate::system_extension::CallbackInputs { payload: None, state };
    let envelope = crate::system_extension::canonical_callback_envelope(context, &invocation, &inputs)
        .expect("canonical callback envelope");
    crate::system_extension::decode_callback_envelope(&envelope.bytes, 1_048_576, 1_048_576, 16)
        .expect("decoded callback envelope")
}

fn bridge_context() -> crate::system_extension::CallbackContext {
    let reference = |label: &str| crate::preserves_rail::content_ref_from_bytes(label.as_bytes());
    crate::system_extension::CallbackContext {
        manifest_ref: reference("bridge-manifest"),
        executable_ref: reference("bridge-executable"),
        instance_id: reference("bridge-instance"),
        extension_id: EXTENSION_ID.to_string(),
        service_id: "collaboration-native".to_string(),
        state_ref: None,
        policy_refs: vec![reference("bridge-policy")],
        resource_ref: reference("bridge-resource"),
        port_binding_refs: vec![reference("bridge-binding")],
    }
}

// r[verify molten.system_extension.incarnation_fencing]
#[test]
fn the_first_incarnation_is_zero_and_survives_the_start_callback() {
    let context = bridge_context();
    let initialize = bridge_envelope(&context, crate::system_extension::CallbackKind::Initialize, 0, 1, None);
    let initialized = handle_callback(&initialize).expect("initialize at the first incarnation");
    let state = initialized.state.clone().expect("initialize state");
    let mut started = context.clone();
    started.state_ref = Some(state.value_ref.clone());
    let start = bridge_envelope(&started, crate::system_extension::CallbackKind::Start, 0, 2, Some(state));
    handle_callback(&start).expect("start at the first incarnation is not a stale predecessor");
    assert!(initialized.checkpoint.is_none());
}

// r[verify molten.system_extension.incarnation_fencing]
#[test]
fn an_absent_generation_is_still_stale() {
    let mut context = bridge_context();
    let value = crate::preserves_rail::record("collaboration-native-bridge-v1", vec![
        crate::preserves_rail::string(SCHEMA),
        crate::preserves_rail::u64_value(0),
        crate::preserves_rail::u64_value(0),
        crate::preserves_rail::u64_value(1),
        crate::preserves_rail::record("none", Vec::new()),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value).expect("state bytes");
    let state = crate::system_extension::NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        bytes,
    };
    context.state_ref = Some(state.value_ref.clone());
    let start = bridge_envelope(&context, crate::system_extension::CallbackKind::Start, 0, 2, Some(state));
    assert!(handle_callback(&start).is_err());
}
