use std::collections;

use super::*;

#[path = "tests/transitions.rs"]
mod transitions;

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()))
}
fn subject(label: &str) -> SubjectRef {
    SubjectRef::new(reference(label)).expect("subject")
}
fn policy(label: &str) -> crate::nominal::PolicyRef {
    crate::nominal::PolicyRef::new(reference(label)).expect("policy")
}
fn operation(label: &str) -> crate::nominal::OperationRef {
    crate::nominal::OperationRef::new(label).expect("operation")
}
fn state_ref() -> StateRef {
    StateRef::new(reference("state")).expect("state")
}

fn fixture(kind: ScopeKind) -> (State, AudienceRequest, Vec<AuthorityDecision>) {
    let owner = subject("owner");
    let scope_ref = ScopeRef::new(reference("scope")).expect("scope");
    let member = Membership {
        reference: MembershipRef::new(reference("member")).expect("member"),
        subject: owner.clone(),
        role: MembershipRole::Owner,
        abilities: [Ability::Read, Ability::Use, Ability::Share].into_iter().collect(),
        start_epoch: 1,
        expires_at: None,
        decision: DecisionRef::new(reference("member-decision")).expect("decision"),
        operation: operation("create"),
    };
    let scope = CollaborationScope {
        reference: scope_ref.clone(),
        kind,
        owner: owner.clone(),
        revision: 1,
        membership_epoch: 1,
        policy: policy("scope-policy"),
        organization_floor: Some(policy("floor")),
        members: [(owner.clone(), member)].into_iter().collect(),
    };
    let resource = crate::nominal::ResourceRef::new(reference("resource")).expect("resource");
    let binding = ScopedResourceBinding {
        reference: BindingRef::new(reference("binding")).expect("binding"),
        resource: resource.clone(),
        owner_scope: scope_ref.clone(),
        owner: owner.clone(),
        revision: 1,
        policy: policy("resource-policy"),
        required_abilities: [Ability::Read, Ability::Use, Ability::Share].into_iter().collect(),
        operation: operation("bind"),
        decision: DecisionRef::new(reference("bind-decision")).expect("decision"),
    };
    let currentness = Currentness {
        service_generation: 1,
        consistency_epoch: 1,
        committed_index: 1,
        logical_now: 10,
        revocation: crate::nominal::RevocationRef::new(reference("revocation")).expect("revocation"),
        observation: crate::nominal::EvidenceRef::new(reference("observation")).expect("evidence"),
    };
    let request = AudienceRequest {
        reference: RequestRef::new(reference("read-request")).expect("request"),
        scope: scope_ref.clone(),
        scope_revision: 1,
        membership_epoch: 1,
        binding: binding.reference.clone(),
        resource: resource.clone(),
        audience: [owner.clone()].into_iter().collect(),
        ability: Ability::Read,
        target: None,
        target_revision: None,
        target_membership_epoch: None,
        scope_policy: scope.policy.clone(),
        resource_policy: binding.policy.clone(),
        organization_floor: scope.organization_floor.clone(),
        consent: None,
        currentness: currentness.clone(),
    };
    let state = State {
        scopes: [(scope_ref, scope)].into_iter().collect(),
        bindings: [(resource, binding)].into_iter().collect(),
        operations: collections::BTreeMap::new(),
        currentness,
    };
    let decisions = required_policies(&state, &request)
        .expect("policies")
        .into_iter()
        .map(|policy| AuthorityDecision {
            request: request.reference.clone(),
            subject: owner.clone(),
            policy: policy.clone(),
            ability: request.ability,
            basalt: DecisionRef::new(reference("basalt")).expect("decision"),
            ucan: DecisionRef::new(reference("ucan")).expect("decision"),
            consent: None,
            currentness: request.currentness.clone(),
            is_allowed: true,
        })
        .collect();
    (state, request, decisions)
}

// r[verify molten.collaboration_scope.validation]
// r[verify molten.collaboration_scope.effective_audience.allowed]
// r[verify molten.collaboration_scope.authority_snapshot.current]
// r[verify molten.collaboration_scope.membership_currentness.current]
#[test]
fn every_kind_admits_exact_current_audience_and_rejects_future_use() {
    for kind in [
        ScopeKind::Personal,
        ScopeKind::Conversation,
        ScopeKind::Group,
        ScopeKind::Organization,
    ] {
        let (mut state, request, decisions) = fixture(kind);
        let snapshot = admit_audience(&state, &state_ref(), &request, &decisions).expect("exact audience");
        revalidate_snapshot(&state, &state_ref(), &snapshot, &decisions).expect("current consumer");
        state.currentness.consistency_epoch += 1;
        assert_eq!(revalidate_snapshot(&state, &state_ref(), &snapshot, &decisions), Err(Denial::Stale));
    }
}

// r[verify molten.collaboration_scope.validation]
// r[verify molten.collaboration_scope.effective_audience.floor]
// r[verify molten.collaboration_scope.effective_audience.partial-denial]
#[test]
fn floor_denial_wrong_bindings_expiry_revocation_and_ability_preserve_state() {
    let (state, request, decisions) = fixture(ScopeKind::Group);
    let original = state.clone();
    for index in 0..decisions.len() {
        let mut denied_facts = decisions.clone();
        denied_facts[index].is_allowed = false;
        assert_eq!(admit_audience(&state, &state_ref(), &request, &denied_facts), Err(Denial::Authority));
    }
    for case in 0..9 {
        let mut candidate = request.clone();
        match case {
            0 => candidate.membership_epoch += 1,
            1 => candidate.scope_revision += 1,
            2 => candidate.binding = BindingRef::new(reference("wrong-binding")).expect("binding"),
            3 => candidate.ability = Ability::Share,
            4 => candidate.organization_floor = None,
            5 => {
                candidate.currentness.revocation =
                    crate::nominal::RevocationRef::new(reference("new-revocation")).expect("revocation")
            }
            6 => candidate.scope = ScopeRef::new(reference("wrong-scope")).expect("scope"),
            7 => candidate.resource = crate::nominal::ResourceRef::new(reference("wrong-resource")).expect("resource"),
            _ => {
                candidate.audience.insert(subject("not-a-member"));
            }
        }
        assert!(admit_audience(&state, &state_ref(), &candidate, &decisions).is_err());
    }
    let mut expired = state.clone();
    expired
        .scopes
        .values_mut()
        .next()
        .expect("scope")
        .members
        .values_mut()
        .next()
        .expect("member")
        .expires_at = Some(10);
    assert_eq!(admit_audience(&expired, &state_ref(), &request, &decisions), Err(Denial::Expired));
    assert_eq!(state, original);
}

// r[verify molten.collaboration_scope.validation]
#[test]
fn missing_and_duplicate_policy_facts_never_satisfy_the_intersection() {
    let (state, request, decisions) = fixture(ScopeKind::Group);
    for index in 0..decisions.len() {
        let mut missing = decisions.clone();
        missing.remove(index);
        assert_eq!(admit_audience(&state, &state_ref(), &request, &missing), Err(Denial::Authority));
    }
    let mut duplicates = decisions.clone();
    duplicates[1] = duplicates[0].clone();
    assert_eq!(admit_audience(&state, &state_ref(), &request, &duplicates), Err(Denial::Authority));
}

// r[verify molten.collaboration_scope.versioned_extension.fabric-denied]
#[test]
fn fabric_node_identity_without_collaboration_membership_is_denied() {
    let (state, mut request, decisions) = fixture(ScopeKind::Group);
    // A fabric node's identity, even in canonical reference form, is not a collaboration member.
    let node = subject("node:fixture-a");
    request.audience = [node.clone()].into_iter().collect();
    let facts: Vec<_> = decisions
        .into_iter()
        .map(|mut fact| {
            fact.subject = node.clone();
            fact
        })
        .collect();
    assert_eq!(admit_audience(&state, &state_ref(), &request, &facts), Err(Denial::Membership));
    assert!(SubjectRef::new("node:fixture-a").is_err(), "a raw node id is not a subject reference");
}

// r[verify molten.collaboration_scope.validation]
// r[verify molten.collaboration_scope.nontransitive_sharing.reader-denied]
// r[verify molten.collaboration_scope.resource_binding.implicit-copy]
#[test]
fn reader_cannot_share_or_implicitly_copy_a_binding() {
    let (state, request, mut facts) = fixture(ScopeKind::Group);
    let mutation = MutationRequest {
        operation: operation("copy"),
        reference: request.reference.clone(),
        before: state_ref(),
        actor: subject("owner"),
        currentness: request.currentness.clone(),
        mutation: Mutation::Bind(state.bindings.values().next().expect("binding").clone()),
    };
    assert_eq!(preview_mutation(&state, &state_ref(), &mutation, &facts), Err(Denial::Authority));
    for fact in &mut facts {
        fact.ability = Ability::Bind;
    }
    assert_eq!(preview_mutation(&state, &state_ref(), &mutation, &facts), Err(Denial::Owner));
}

// r[verify molten.collaboration_scope.validation]
#[test]
fn bounded_membership_sequences_are_monotone_replayable_and_denial_preserving() {
    let (mut state, request, _) = fixture(ScopeKind::Group);
    let scope_ref = request.scope;
    for epoch in 2..64 {
        let operation = operation(&format!("membership-{epoch}"));
        let member = Membership {
            reference: MembershipRef::new(reference(&format!("member-{epoch}"))).expect("member"),
            subject: subject(&format!("subject-{epoch}")),
            abilities: [Ability::Read].into_iter().collect(),
            role: MembershipRole::Participant,
            start_epoch: epoch,
            expires_at: None,
            decision: DecisionRef::new(reference("membership-authority")).expect("decision"),
            operation: operation.clone(),
        };
        let mutation = MutationRequest {
            operation,
            reference: RequestRef::new(reference(&format!("request-{epoch}"))).expect("request"),
            before: state_ref(),
            actor: subject("owner"),
            currentness: state.currentness.clone(),
            mutation: Mutation::PutMember {
                scope: scope_ref.clone(),
                member,
            },
        };
        let facts: Vec<_> = mutation_policies(&state, &mutation.mutation)
            .expect("policies")
            .into_iter()
            .map(|policy| AuthorityDecision {
                request: mutation.reference.clone(),
                subject: mutation.actor.clone(),
                policy,
                ability: Ability::Membership,
                basalt: DecisionRef::new(reference("basalt")).expect("decision"),
                ucan: DecisionRef::new(reference("ucan")).expect("decision"),
                consent: None,
                currentness: state.currentness.clone(),
                is_allowed: true,
            })
            .collect();
        let before = state.clone();
        let first = preview_mutation(&state, &state_ref(), &mutation, &facts).expect("mutation");
        assert_eq!(first, preview_mutation(&state, &state_ref(), &mutation, &facts).expect("deterministic replay"));
        let MutationPlan::Apply(next) = first else {
            panic!("new operation")
        };
        assert_eq!(next.scopes[&scope_ref].membership_epoch, epoch);
        assert_eq!(next.bindings, before.bindings);
        assert!(next.scopes[&scope_ref].members.len() <= MAX_MEMBERS);
        let mut denied = facts.clone();
        denied[0].is_allowed = false;
        assert_eq!(preview_mutation(&state, &state_ref(), &mutation, &denied), Err(Denial::Authority));
        assert_eq!(state, before);
        state = next;
    }
}
