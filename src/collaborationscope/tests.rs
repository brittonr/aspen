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
