//! Hegel properties over the pure collaboration-scope transitions: a committed membership change
//! advances the epoch by exactly one, previews are deterministic and never mutate their input,
//! replays keep their original result, allowed authority is never misreported as an authority
//! denial, collections stay bounded, and every resource keeps exactly one current owner scope.

use super::*;

const SUBJECTS: usize = 6;
const SCOPES: usize = 3;
const RESOURCES: usize = 3;
const STEPS: usize = 24;

fn reference(label: &str) -> String {
    super::fixtures::reference(label)
}

fn subject(label: &str) -> SubjectRef {
    SubjectRef::new(reference(&format!("subject-{label}"))).expect("subject")
}

fn space(index: usize) -> ScopeRef {
    ScopeRef::new(reference(&format!("scope-{index}"))).expect("scope")
}

fn resource(index: usize) -> molten_core::nominal::ResourceRef {
    molten_core::nominal::ResourceRef::new(reference(&format!("resource-{index}"))).expect("resource")
}

fn policy(label: &str) -> molten_core::nominal::PolicyRef {
    molten_core::nominal::PolicyRef::new(reference(label)).expect("policy")
}

fn operation(step: usize) -> molten_core::nominal::OperationRef {
    molten_core::nominal::OperationRef::new(format!("step-{step}")).expect("operation")
}

fn decision(label: &str) -> DecisionRef {
    DecisionRef::new(reference(label)).expect("decision")
}

fn state_ref() -> StateRef {
    StateRef::new(reference("property-state")).expect("state")
}

fn member(subject: SubjectRef, role: MembershipRole, start_epoch: u64, step: usize, expiry: Option<u64>) -> Membership {
    Membership {
        reference: MembershipRef::new(reference(&format!("member-{step}"))).expect("membership"),
        subject,
        role,
        abilities: [Ability::Read, Ability::Use].into_iter().collect(),
        start_epoch,
        expires_at: expiry,
        decision: decision("member-decision"),
        operation: operation(step),
    }
}

fn space_record(index: usize) -> CollaborationScope {
    let owner = subject(&format!("owner-{index}"));
    CollaborationScope {
        reference: space(index),
        kind: ScopeKind::Group,
        owner: owner.clone(),
        revision: 1,
        membership_epoch: 1,
        policy: policy(&format!("policy-{index}")),
        organization_floor: Some(policy("organization-floor")),
        members: [(owner.clone(), member(owner, MembershipRole::Owner, 1, 0, None))].into_iter().collect(),
    }
}

fn state(scopes: usize) -> State {
    State {
        scopes: (0..scopes).map(|index| (space(index), space_record(index))).collect(),
        bindings: Default::default(),
        operations: Default::default(),
        currentness: Currentness {
            service_generation: 1,
            consistency_epoch: 1,
            committed_index: 1,
            logical_now: 10,
            revocation: molten_core::nominal::RevocationRef::new(reference("revocation")).expect("revocation"),
            observation: molten_core::nominal::EvidenceRef::new(reference("observation")).expect("observation"),
        },
    }
}

fn request(state: &State, step: usize, actor: SubjectRef, mutation: Mutation) -> MutationRequest {
    MutationRequest {
        operation: operation(step),
        reference: RequestRef::new(reference(&format!("request-{step}"))).expect("request"),
        before: state_ref(),
        actor,
        currentness: state.currentness.clone(),
        mutation,
    }
}

/// One exact decision per required policy; `is_allowed` is the drawn authority outcome.
fn decisions(state: &State, request: &MutationRequest, is_allowed: bool) -> Vec<AuthorityDecision> {
    let policies = mutation_policies(state, &request.mutation).unwrap_or_default();
    policies
        .into_iter()
        .map(|policy| AuthorityDecision {
            request: request.reference.clone(),
            subject: request.actor.clone(),
            policy,
            ability: request.mutation.ability(),
            basalt: decision("basalt"),
            ucan: decision("ucan"),
            consent: None,
            currentness: request.currentness.clone(),
            is_allowed,
        })
        .collect()
}

/// Previews twice: the plan is deterministic and the input state is never mutated.
fn preview_purely(
    state: &State,
    request: &MutationRequest,
    facts: &[AuthorityDecision],
) -> Result<MutationPlan, Denial> {
    let before = state.clone();
    let first = preview_mutation(state, &state_ref(), request, facts);
    assert_eq!(first, preview_mutation(state, &state_ref(), request, facts), "deterministic replay");
    assert_eq!(state, &before, "a preview never mutates its input");
    first
}

fn membership_mutation(
    tc: &hegel::TestCase,
    state: &State,
    step: usize,
    applied: &[MutationRequest],
) -> MutationRequest {
    let action = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(3));
    let label = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(SUBJECTS - 1));
    let (scope, owner) = (space(0), state.scopes[&space(0)].owner.clone());
    let target = subject(&format!("member-{label}"));
    match (action, applied.len()) {
        (0, _) => {
            let epoch = state.scopes[&scope].membership_epoch + 1;
            let lifetime = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(3));
            let expiry = Some(state.currentness.logical_now + lifetime);
            let member = member(target, MembershipRole::Participant, epoch, step, expiry);
            request(state, step, owner, Mutation::PutMember { scope, member })
        }
        (1, _) => request(state, step, owner, Mutation::RemoveMember { scope, subject: target }),
        (2, _) | (3, 0) => request(state, step, owner, Mutation::ExpireMember { scope, subject: target }),
        (_, count) => {
            let index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(count - 1));
            let mut replay = applied[index].clone();
            if tc.draw(hegel::generators::booleans()) {
                replay.reference = RequestRef::new(reference(&format!("conflict-{step}"))).expect("request");
            }
            replay
        }
    }
}

// r[verify molten.collaboration_scope.validation]
// r[verify molten.collaboration_scope.validation.complete]
#[hegel::test(test_cases = 64)]
fn hegel_membership_epochs_are_monotone_replayable_bounded_and_denial_preserving(tc: hegel::TestCase) {
    let mut state = state(1);
    let mut applied: Vec<MutationRequest> = Vec::with_capacity(STEPS);
    for step in 1..=STEPS {
        state.currentness.logical_now += tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(2));
        let mut request = membership_mutation(&tc, &state, step, &applied);
        request.currentness.clone_from(&state.currentness);
        let is_allowed = tc.draw(hegel::generators::booleans());
        let facts = decisions(&state, &request, is_allowed);
        let epoch = state.scopes[&space(0)].membership_epoch;
        match (state.operations.get(&request.operation), preview_purely(&state, &request, &facts)) {
            (Some(prior), plan) if prior.request == request.reference => {
                assert_eq!(plan, Ok(MutationPlan::Duplicate(prior.clone())), "an identical replay keeps its result");
            }
            (Some(_), plan) => assert_eq!(plan, Err(Denial::Conflict), "a conflicting replay is denied"),
            (None, Ok(MutationPlan::Apply(next))) => {
                assert!(is_allowed, "only allowed authority commits");
                assert_eq!(next.scopes[&space(0)].membership_epoch, epoch + 1, "one commit, one epoch");
                assert!(next.scopes[&space(0)].members.len() <= MAX_MEMBERS);
                assert_eq!(next.operations.len(), state.operations.len() + 1);
                assert_eq!(next.bindings, state.bindings);
                validate_state(&next).expect("committed state stays valid");
                applied.push(request);
                state = next;
            }
            (None, Ok(MutationPlan::Duplicate(_))) => panic!("a fresh operation is never a duplicate"),
            (None, Err(denial)) => {
                if is_allowed {
                    assert_ne!(denial, Denial::Authority, "allowed authority is never an authority denial");
                }
            }
        }
    }
}

fn binding(
    state: &State,
    step: usize,
    resource: molten_core::nominal::ResourceRef,
    owner: usize,
) -> ScopedResourceBinding {
    let scope = &state.scopes[&space(owner)];
    ScopedResourceBinding {
        reference: BindingRef::new(reference(&format!("binding-{step}"))).expect("binding"),
        resource,
        owner_scope: scope.reference.clone(),
        owner: scope.owner.clone(),
        revision: 1,
        policy: scope.policy.clone(),
        required_abilities: [Ability::Read, Ability::Share, Ability::Move].into_iter().collect(),
        operation: operation(step),
        decision: decision("bind-decision"),
    }
}

fn assert_one_owner(state: &State) {
    assert!(state.bindings.len() <= RESOURCES);
    for (resource, binding) in &state.bindings {
        assert_eq!(resource, &binding.resource);
        let owner = state.scopes.get(&binding.owner_scope).expect("every binding names a current owner scope");
        assert_eq!(binding.owner, owner.owner, "the binding owner is its scope's owner");
    }
}

// r[verify molten.collaboration_scope.validation]
// r[verify molten.collaboration_scope.validation.complete]
#[hegel::test(test_cases = 64)]
fn hegel_bind_and_move_keep_exactly_one_current_owner_scope(tc: hegel::TestCase) {
    let mut state = state(SCOPES);
    for step in 1..=STEPS {
        let item = resource(tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(RESOURCES - 1)));
        let target = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(SCOPES - 1));
        let is_move = tc.draw(hegel::generators::booleans());
        let mutation = if is_move {
            let replacement = BindingRef::new(reference(&format!("binding-{step}"))).expect("binding");
            Mutation::Move {
                resource: item.clone(),
                target: space(target),
                replacement,
            }
        } else {
            Mutation::Bind(binding(&state, step, item.clone(), target))
        };
        let request = request(&state, step, subject("operator"), mutation);
        let is_allowed = tc.draw(hegel::generators::booleans());
        let before = state.bindings.get(&item).cloned();
        match preview_purely(&state, &request, &decisions(&state, &request, is_allowed)) {
            Ok(MutationPlan::Apply(next)) => {
                assert!(is_allowed, "only allowed authority commits");
                let after = &next.bindings[&item];
                assert_eq!(after.owner_scope, space(target));
                match before {
                    Some(prior) => assert_eq!(after.revision, prior.revision + 1, "a move replaces the one binding"),
                    None => {
                        assert!(!is_move, "only an explicit bind creates a binding");
                        assert_eq!(after.revision, 1);
                    }
                }
                state = next;
            }
            Ok(MutationPlan::Duplicate(_)) => panic!("each step uses a fresh operation"),
            Err(denial) if is_allowed && before.is_some() && !is_move => {
                assert_eq!(denial, Denial::Owner, "binding a bound resource is not an implicit copy");
            }
            Err(_) => {}
        }
        assert_one_owner(&state);
    }
}
