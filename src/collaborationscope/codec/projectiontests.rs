//! Operator status and downstream consumer views stay metadata-only, and snapshot decoding rejects
//! appended payloads, raw identities in reference positions and a missing non-bearer marker.

use super::*;
use crate::fabric_consistency::raft::CommittedBatchHandler;

struct Admitted {
    _workspace: crate::test_support::ProcessWorkspace,
    application: Application,
    snapshot: ScopeAdmissionSnapshot,
}

fn admitted(label: &str) -> Admitted {
    let workspace = crate::test_support::process_workspace(label).expect("workspace");
    let mut application =
        fixtures::open_application(&workspace.join("scope"), &workspace.join("values")).expect("application");
    let authority = fixtures::Signers::new();
    let replay = NonceLedger::new(fixtures::open_store(&workspace.join("replay")).expect("replay"));
    let mutation = fixtures::add_member(application.state(), "status-member").expect("membership request");
    let staged_at = current(&application);
    let command = authority.stage(&mut application, staged_at, mutation, &replay).expect("authority");
    application.apply_batch(&[command]).expect("membership commit");
    let snapshot = authority.snapshot(&application, current(&application), &replay).expect("read admission");
    Admitted {
        _workspace: workspace,
        application,
        snapshot,
    }
}

fn current(application: &Application) -> CurrentState {
    CurrentState {
        currentness: application.state().currentness.clone(),
        state: application.state_ref().clone(),
    }
}

fn text(value: &preserves::IOValue) -> String {
    crate::preserves_rail::to_text(value).expect("rendered projection")
}

// r[verify molten.collaboration_scope.safe_projection]
// r[verify molten.collaboration_scope.safe_projection.safe]
#[test]
fn operator_status_exposes_references_epochs_counts_and_nonclaims_without_member_or_resource_content() {
    let admitted = admitted("collaboration-operator-status");
    let state = admitted.application.state();
    let scope = &state.scopes[&fixtures::space_ref()];
    let status = text(&admitted.application.status());
    assert!(status.contains(scope.reference.as_str()));
    assert!(status.contains(admitted.application.state_ref().as_str()));
    assert!(status.contains("loaded-state-not-current-authority"));
    assert!(status.contains("not-global-revocation-or-confidentiality-or-release-proof"));
    assert_eq!((scope.membership_epoch, scope.members.len()), (2, 2));
    for subject in scope.members.keys() {
        assert!(!status.contains(subject.as_str()), "member subjects are not status labels");
    }
    for (resource, binding) in &state.bindings {
        assert!(!status.contains(resource.as_str()), "resources are not status content");
        assert!(!status.contains(binding.policy.as_str()), "policies are not status content");
    }
    assert!(!status.contains(fixtures::POLICY), "policy bodies never reach status");
}

// r[verify molten.collaboration_scope.safe_projection]
// r[verify molten.collaboration_scope.safe_projection.safe]
#[test]
fn consumer_views_are_metadata_only_and_require_revalidation() {
    let admitted = admitted("collaboration-consumer-views");
    let snapshot = &admitted.snapshot;
    let identity = snapshot_ref(snapshot).expect("snapshot identity");
    let consumers = [Consumer::Lattice, Consumer::Animus, Consumer::Tile];
    let mut views = Vec::with_capacity(consumers.len());
    for consumer in consumers {
        let label = consumer.as_str();
        let view = text(&consumer_value(snapshot, consumer).expect("consumer view"));
        assert!(view.contains(label));
        assert!(view.contains(identity.as_str()));
        assert!(view.contains(snapshot.request.reference.as_str()));
        assert!(view.contains(snapshot.request.scope.as_str()));
        assert!(view.contains("revalidate-before-effect-not-bearer-authority"));
        for subject in &snapshot.request.audience {
            assert!(!view.contains(subject.as_str()), "{label} receives an audience digest, not subjects");
        }
        for decision in &snapshot.decisions {
            assert!(!view.contains(decision.basalt.as_str()), "{label} receives no Basalt decision");
            assert!(!view.contains(decision.ucan.as_str()), "{label} receives no UCAN decision");
        }
        assert!(!view.contains(snapshot.request.scope_policy.as_str()), "{label} receives no policy reference");
        views.push(view.replace(label, ""));
    }
    assert!(views.windows(2).all(|pair| pair[0] == pair[1]), "views differ only by consumer label");
}

fn mutated_snapshot(snapshot: &ScopeAdmissionSnapshot, edit: impl Fn(&mut Vec<preserves::IOValue>)) -> Vec<u8> {
    let mut fields: Vec<_> = snapshot_value(snapshot)
        .collect_simple_record("scope-admission-snapshot-v1", None)
        .expect("snapshot record")
        .fields_iter()
        .map(crate::preserves_rail::value_to_iovalue)
        .collect();
    edit(&mut fields);
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("scope-admission-snapshot-v1", fields))
        .expect("snapshot bytes")
}

fn with_raw_audience(fields: &mut [preserves::IOValue]) {
    let mut request: Vec<_> = fields[0]
        .collect_simple_record("scope-audience-request-v1", None)
        .expect("audience request")
        .fields_iter()
        .map(crate::preserves_rail::value_to_iovalue)
        .collect();
    request[5] =
        crate::preserves_rail::sequence(vec![crate::preserves_rail::string("did:key:z6MkRawIdentityDocument")]);
    fields[0] = crate::preserves_rail::record("scope-audience-request-v1", request);
}

// r[verify molten.collaboration_scope.safe_projection]
// r[verify molten.collaboration_scope.safe_projection.leak]
// r[verify molten.collaboration_scope.nominal_model.valid]
// r[verify molten.collaboration_scope.nominal_model.domain-mismatch]
// r[verify molten.collaboration_scope.validation.overclaim]
#[test]
fn snapshot_decoding_rejects_payloads_raw_identities_and_bearer_reuse() {
    let admitted = admitted("collaboration-snapshot-leaks");
    let snapshot = &admitted.snapshot;
    let exact = crate::preserves_rail::canonical_bytes(&snapshot_value(snapshot)).expect("snapshot bytes");
    assert_eq!(decode_snapshot(&exact).expect("closed projection"), *snapshot);
    let candidates = [
        mutated_snapshot(snapshot, |fields| fields.push(crate::preserves_rail::string("raw-ucan-token-or-message"))),
        mutated_snapshot(snapshot, |fields| fields[4] = crate::preserves_rail::string("reusable-bearer-capability")),
        mutated_snapshot(snapshot, |fields| fields[4] = crate::preserves_rail::string("globally-fresh-revocation")),
        mutated_snapshot(snapshot, |fields| fields[4] = crate::preserves_rail::string("confidential-release-eligible")),
        mutated_snapshot(snapshot, |fields| fields[2] = crate::preserves_rail::string("did:key:z6MkRawState")),
        mutated_snapshot(snapshot, |fields| with_raw_audience(fields)),
    ];
    for candidate in candidates {
        assert!(decode_snapshot(&candidate).is_err(), "unsafe snapshot candidate decoded");
    }
}
