use super::*;

fn request(state: &State, mutation: Mutation, name: &str) -> MutationRequest {
    MutationRequest {
        operation: operation(name),
        reference: RequestRef::new(reference(name)).expect("request"),
        before: state_ref(),
        actor: subject("owner"),
        currentness: state.currentness.clone(),
        mutation,
    }
}
fn facts(state: &State, request: &MutationRequest) -> Vec<AuthorityDecision> {
    mutation_policies(state, &request.mutation)
        .expect("policies")
        .into_iter()
        .map(|policy| AuthorityDecision {
            request: request.reference.clone(),
            subject: request.actor.clone(),
            policy,
            ability: request.mutation.ability(),
            basalt: DecisionRef::new(reference("basalt-authority")).expect("decision"),
            ucan: DecisionRef::new(reference("ucan-authority")).expect("decision"),
            consent: None,
            currentness: request.currentness.clone(),
            is_allowed: true,
        })
        .collect()
}
fn apply(state: &State, request: &MutationRequest) -> State {
    let MutationPlan::Apply(next) =
        preview_mutation(state, &state_ref(), request, &facts(state, request)).expect("authorized mutation")
    else {
        panic!("new operation")
    };
    next
}
fn new_member(epoch: u64, name: &str) -> Membership {
    Membership {
        reference: MembershipRef::new(reference(name)).expect("member"),
        subject: subject("participant"),
        role: MembershipRole::Participant,
        abilities: [Ability::Read].into_iter().collect(),
        start_epoch: epoch,
        expires_at: Some(11),
        decision: DecisionRef::new(reference("membership-authority")).expect("decision"),
        operation: operation(name),
    }
}
// r[verify molten.collaboration_scope.validation]
#[test]
fn all_scope_kinds_create_and_personal_scope_cannot_gain_foreign_members() {
    for kind in [
        ScopeKind::Personal,
        ScopeKind::Conversation,
        ScopeKind::Group,
        ScopeKind::Organization,
    ] {
        let (mut state, _, _) = fixture(kind);
        let scope = state.scopes.values().next().expect("scope").clone();
        state.scopes.clear();
        state.bindings.clear();
        let created = apply(&state, &request(&state, Mutation::Create(scope.clone()), "create-scope"));
        assert_eq!(created.scopes[&scope.reference], scope);
        let mutation = request(
            &created,
            Mutation::PutMember {
                scope: scope.reference.clone(),
                member: new_member(2, "add-participant"),
            },
            "add-participant",
        );
        if kind == ScopeKind::Personal {
            assert_eq!(
                preview_mutation(&created, &state_ref(), &mutation, &facts(&created, &mutation)),
                Err(Denial::Membership)
            );
            assert_eq!(created.scopes[&scope.reference].members.len(), 1);
        } else {
            assert_eq!(apply(&created, &mutation).scopes[&scope.reference].membership_epoch, 2);
        }
    }
}
// r[verify molten.collaboration_scope.validation]
#[test]
fn role_changes_removal_expiry_and_duplicate_conflicts_are_epoch_fenced() {
    let (state, audience, _) = fixture(ScopeKind::Group);
    let scope = audience.scope;
    let add = request(
        &state,
        Mutation::PutMember {
            scope: scope.clone(),
            member: new_member(2, "add"),
        },
        "add",
    );
    let added = apply(&state, &add);
    assert!(matches!(
        preview_mutation(&added, &state_ref(), &add, &facts(&state, &add)),
        Ok(MutationPlan::Duplicate(_))
    ));
    let mut conflict = add.clone();
    conflict.reference = RequestRef::new(reference("different-operation-body")).expect("request");
    assert_eq!(preview_mutation(&added, &state_ref(), &conflict, &facts(&added, &conflict)), Err(Denial::Conflict));
    let mut changed = new_member(3, "role-change");
    changed.role = MembershipRole::Viewer;
    let changed = apply(
        &added,
        &request(
            &added,
            Mutation::PutMember {
                scope: scope.clone(),
                member: changed,
            },
            "role-change",
        ),
    );
    assert_eq!(changed.scopes[&scope].membership_epoch, 3);
    assert_eq!(changed.scopes[&scope].members[&subject("participant")].role, MembershipRole::Viewer);
    let removed = apply(
        &changed,
        &request(
            &changed,
            Mutation::RemoveMember {
                scope: scope.clone(),
                subject: subject("participant"),
            },
            "remove",
        ),
    );
    assert_eq!(removed.scopes[&scope].membership_epoch, 4);
    assert!(!removed.scopes[&scope].members.contains_key(&subject("participant")));
    let mut expired = added.clone();
    expired.currentness.logical_now = 11;
    let expiry = request(
        &expired,
        Mutation::ExpireMember {
            scope: scope.clone(),
            subject: subject("participant"),
        },
        "expire",
    );
    assert!(!apply(&expired, &expiry).scopes[&scope].members.contains_key(&subject("participant")));
    let premature = request(
        &added,
        Mutation::ExpireMember {
            scope,
            subject: subject("participant"),
        },
        "early-expire",
    );
    assert_eq!(preview_mutation(&added, &state_ref(), &premature, &facts(&added, &premature)), Err(Denial::Expired));
}
// r[verify molten.collaboration_scope.validation]
#[test]
fn exact_bind_and_explicit_move_keep_one_owner_and_floor_denials_preserve_binding() {
    let (mut state, _, _) = fixture(ScopeKind::Group);
    let mut target = state.scopes.values().next().expect("scope").clone();
    target.reference = ScopeRef::new(reference("target-scope")).expect("scope");
    target.policy = policy("target-policy");
    target.organization_floor = Some(policy("target-floor"));
    state.scopes.insert(target.reference.clone(), target.clone());
    let original = state.bindings.values().next().expect("binding").clone();
    let mut fresh = original.clone();
    fresh.resource = crate::nominal::ResourceRef::new(reference("new-resource")).expect("resource");
    fresh.reference = BindingRef::new(reference("new-binding")).expect("binding");
    fresh.operation = operation("explicit-bind");
    let bound = apply(&state, &request(&state, Mutation::Bind(fresh), "explicit-bind"));
    assert_eq!(bound.bindings.len(), 2);
    let move_request = request(
        &state,
        Mutation::Move {
            resource: original.resource.clone(),
            target: target.reference.clone(),
            replacement: BindingRef::new(reference("moved-binding")).expect("binding"),
        },
        "explicit-move",
    );
    let mut denied = facts(&state, &move_request);
    denied.iter_mut().find(|fact| fact.policy == policy("target-floor")).expect("floor").is_allowed = false;
    assert_eq!(preview_mutation(&state, &state_ref(), &move_request, &denied), Err(Denial::Authority));
    assert_eq!(state.bindings[&original.resource], original);
    let moved = apply(&state, &move_request);
    assert_eq!(moved.bindings.len(), 1);
    let current = &moved.bindings[&original.resource];
    assert_eq!(current.owner_scope, target.reference);
    assert_eq!(current.revision, 2);
    assert_eq!(current.operation, operation("explicit-move"));
    assert_ne!(current.reference, original.reference);
    let mut implicit = original.clone();
    implicit.owner_scope = target.reference;
    let copy = request(&state, Mutation::Bind(implicit), "implicit-copy");
    assert_eq!(preview_mutation(&state, &state_ref(), &copy, &facts(&state, &copy)), Err(Denial::Owner));
}
// r[verify molten.collaboration_scope.validation]
#[test]
fn sharing_requires_exact_ability_and_both_scope_policy_intersections() {
    let (mut state, mut request, _) = fixture(ScopeKind::Group);
    let mut target = state.scopes[&request.scope].clone();
    target.reference = ScopeRef::new(reference("share-target")).expect("scope");
    target.policy = policy("target-share-policy");
    state.scopes.insert(target.reference.clone(), target.clone());
    request.ability = Ability::Share;
    request.target = Some(target.reference);
    request.target_revision = Some(1);
    request.target_membership_epoch = Some(1);
    let mut decisions: Vec<_> = required_policies(&state, &request)
        .expect("effective policies")
        .into_iter()
        .map(|policy| AuthorityDecision {
            request: request.reference.clone(),
            subject: subject("owner"),
            policy: policy.clone(),
            ability: Ability::Share,
            basalt: DecisionRef::new(reference("share-policy-decision")).expect("decision"),
            ucan: DecisionRef::new(reference("share-delegation")).expect("decision"),
            consent: None,
            currentness: request.currentness.clone(),
            is_allowed: true,
        })
        .collect();
    admit_audience(&state, &state_ref(), &request, &decisions).expect("exact non-transitive share");
    for fact in &mut decisions {
        fact.ability = Ability::Read;
    }
    assert_eq!(admit_audience(&state, &state_ref(), &request, &decisions), Err(Denial::Authority));
    for fact in &mut decisions {
        fact.ability = Ability::Share;
        if fact.policy == policy("target-share-policy") {
            fact.is_allowed = false;
        }
    }
    assert_eq!(admit_audience(&state, &state_ref(), &request, &decisions), Err(Denial::Authority));
    request.target_revision = Some(2);
    assert_eq!(admit_audience(&state, &state_ref(), &request, &decisions), Err(Denial::Stale));
}
// r[verify molten.collaboration_scope.validation]
#[test]
fn membership_capacity_and_stale_generation_deny_without_partial_growth() {
    let (mut state, audience, _) = fixture(ScopeKind::Group);
    let scope = audience.scope;
    for index in 1..MAX_MEMBERS {
        let mut member = new_member(1, "initial");
        member.subject = subject(&format!("member-{index}"));
        state.scopes.get_mut(&scope).expect("scope").members.insert(member.subject.clone(), member);
    }
    let original = state.clone();
    let request = request(
        &state,
        Mutation::PutMember {
            scope,
            member: new_member(2, "overflow"),
        },
        "overflow",
    );
    assert_eq!(preview_mutation(&state, &state_ref(), &request, &facts(&state, &request)), Err(Denial::Bound));
    assert_eq!(state, original);
    let mut stale = request.clone();
    stale.currentness.service_generation += 1;
    assert_eq!(preview_mutation(&state, &state_ref(), &stale, &facts(&state, &stale)), Err(Denial::Stale));
    assert_eq!(state, original);
}
