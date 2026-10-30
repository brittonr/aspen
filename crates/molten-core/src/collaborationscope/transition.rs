use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mutation {
    Create(CollaborationScope),
    PutMember {
        scope: ScopeRef,
        member: Membership,
    },
    RemoveMember {
        scope: ScopeRef,
        subject: SubjectRef,
    },
    ExpireMember {
        scope: ScopeRef,
        subject: SubjectRef,
    },
    Bind(ScopedResourceBinding),
    Move {
        resource: crate::nominal::ResourceRef,
        target: ScopeRef,
        replacement: BindingRef,
    },
}

impl Mutation {
    pub const fn ability(&self) -> Ability {
        match self {
            Self::Create(_) => Ability::Create,
            Self::PutMember { .. } | Self::RemoveMember { .. } | Self::ExpireMember { .. } => Ability::Membership,
            Self::Bind(_) => Ability::Bind,
            Self::Move { .. } => Ability::Move,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationRequest {
    pub operation: crate::nominal::OperationRef,
    pub reference: RequestRef,
    pub before: StateRef,
    pub actor: SubjectRef,
    pub currentness: Currentness,
    pub mutation: Mutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationPlan {
    Apply(State),
    Duplicate(PriorOperation),
}

// r[impl molten.collaboration_scope.membership_currentness]
// r[impl molten.collaboration_scope.resource_binding]
pub fn preview_mutation(
    state: &State,
    state_ref: &StateRef,
    request: &MutationRequest,
    decisions: &[AuthorityDecision],
) -> Result<MutationPlan, Denial> {
    validate_state(state)?;
    if let Some(prior) = state.operations.get(&request.operation) {
        return if prior.request == request.reference {
            Ok(MutationPlan::Duplicate(prior.clone()))
        } else {
            Err(Denial::Conflict)
        };
    }
    if &request.before != state_ref || !currentness_extends(&state.currentness, &request.currentness) {
        return Err(Denial::Stale);
    }
    if state.operations.len() == MAX_OPERATIONS {
        return Err(Denial::Bound);
    }
    validate_authority(state, request, decisions)?;
    let mut next = state.clone();
    next.currentness.clone_from(&request.currentness);
    match &request.mutation {
        Mutation::Create(scope) => create(&mut next, scope, request)?,
        Mutation::PutMember { scope, member } => put_member(&mut next, scope, member, request)?,
        Mutation::RemoveMember { scope, subject } => remove_member(&mut next, scope, subject, false)?,
        Mutation::ExpireMember { scope, subject } => remove_member(&mut next, scope, subject, true)?,
        Mutation::Bind(binding) => bind(&mut next, binding, request)?,
        Mutation::Move {
            resource,
            target,
            replacement,
        } => move_binding(&mut next, resource, target, replacement, (&request.operation, &decisions[0].basalt))?,
    }
    next.operations.insert(request.operation.clone(), PriorOperation {
        request: request.reference.clone(),
    });
    validate_state(&next)?;
    Ok(MutationPlan::Apply(next))
}

fn validate_authority(state: &State, request: &MutationRequest, decisions: &[AuthorityDecision]) -> Result<(), Denial> {
    let policies = mutation_policies(state, &request.mutation)?;
    if decisions.len() != policies.len() {
        return Err(Denial::Authority);
    }
    for policy in policies {
        let mut matching = decisions.iter().filter(|fact| fact.policy == policy);
        let fact = matching.next().ok_or(Denial::Authority)?;
        if matching.next().is_some() {
            return Err(Denial::Authority);
        }
        if !fact.is_allowed
            || fact.subject != request.actor
            || fact.request != request.reference
            || fact.ability != request.mutation.ability()
            || fact.currentness != request.currentness
        {
            return Err(Denial::Authority);
        }
    }
    Ok(())
}

pub fn mutation_policies(state: &State, mutation: &Mutation) -> Result<Vec<crate::nominal::PolicyRef>, Denial> {
    let mut policies = Vec::new();
    let scope = match mutation {
        Mutation::Create(scope) => scope,
        Mutation::PutMember { scope, .. }
        | Mutation::RemoveMember { scope, .. }
        | Mutation::ExpireMember { scope, .. } => state.scopes.get(scope).ok_or(Denial::Missing)?,
        Mutation::Bind(binding) => {
            policies.push(binding.policy.clone());
            state.scopes.get(&binding.owner_scope).ok_or(Denial::Missing)?
        }
        Mutation::Move { resource, target, .. } => {
            let binding = state.bindings.get(resource).ok_or(Denial::Missing)?;
            policies.push(binding.policy.clone());
            add_scope_policies(&mut policies, state.scopes.get(target).ok_or(Denial::Missing)?);
            state.scopes.get(&binding.owner_scope).ok_or(Denial::Missing)?
        }
    };
    add_scope_policies(&mut policies, scope);
    policies.sort();
    policies.dedup();
    Ok(policies)
}

fn add_scope_policies(policies: &mut Vec<crate::nominal::PolicyRef>, scope: &CollaborationScope) {
    policies.push(scope.policy.clone());
    if let Some(floor) = &scope.organization_floor {
        policies.push(floor.clone());
    }
}

fn create(state: &mut State, scope: &CollaborationScope, request: &MutationRequest) -> Result<(), Denial> {
    if state.scopes.len() == MAX_SCOPES {
        return Err(Denial::Bound);
    }
    if scope.members.is_empty() || scope.members.len() > MAX_MEMBERS {
        return Err(Denial::Bound);
    }
    if state.scopes.contains_key(&scope.reference) {
        return Err(Denial::Conflict);
    }
    if scope.owner != request.actor || scope.revision != 1 || scope.membership_epoch != 1 {
        return Err(Denial::Owner);
    }
    state.scopes.insert(scope.reference.clone(), scope.clone());
    Ok(())
}

fn advance(scope: &mut CollaborationScope) -> Result<(), Denial> {
    scope.revision = scope.revision.checked_add(1).ok_or(Denial::Bound)?;
    scope.membership_epoch = scope.membership_epoch.checked_add(1).ok_or(Denial::Bound)?;
    Ok(())
}

fn put_member(
    state: &mut State,
    scope: &ScopeRef,
    member: &Membership,
    request: &MutationRequest,
) -> Result<(), Denial> {
    let scope = state.scopes.get_mut(scope).ok_or(Denial::Missing)?;
    if scope.kind == ScopeKind::Personal && member.subject != scope.owner {
        return Err(Denial::Membership);
    }
    if !scope.members.contains_key(&member.subject) && scope.members.len() == MAX_MEMBERS {
        return Err(Denial::Bound);
    }
    if member.expires_at.is_some_and(|expiry| expiry <= request.currentness.logical_now)
        || member.operation != request.operation
    {
        return Err(Denial::Expired);
    }
    advance(scope)?;
    if member.start_epoch != scope.membership_epoch {
        return Err(Denial::Stale);
    }
    scope.members.insert(member.subject.clone(), member.clone());
    Ok(())
}

fn remove_member(state: &mut State, scope: &ScopeRef, subject: &SubjectRef, is_expiry: bool) -> Result<(), Denial> {
    let scope = state.scopes.get_mut(scope).ok_or(Denial::Missing)?;
    if &scope.owner == subject {
        return Err(Denial::Owner);
    }
    let member = scope.members.get(subject).ok_or(Denial::Missing)?;
    if is_expiry && !member.expires_at.is_some_and(|expiry| expiry <= state.currentness.logical_now) {
        return Err(Denial::Expired);
    }
    advance(scope)?;
    scope.members.remove(subject);
    Ok(())
}

fn bind(state: &mut State, binding: &ScopedResourceBinding, request: &MutationRequest) -> Result<(), Denial> {
    if state.bindings.len() == MAX_BINDINGS {
        return Err(Denial::Bound);
    }
    if state.bindings.contains_key(&binding.resource) {
        return Err(Denial::Owner);
    }
    if binding.operation != request.operation || binding.revision != 1 {
        return Err(Denial::Conflict);
    }
    state.bindings.insert(binding.resource.clone(), binding.clone());
    Ok(())
}

fn move_binding(
    state: &mut State,
    resource: &crate::nominal::ResourceRef,
    target: &ScopeRef,
    replacement: &BindingRef,
    evidence: (&crate::nominal::OperationRef, &DecisionRef),
) -> Result<(), Denial> {
    let target = state.scopes.get(target).ok_or(Denial::Missing)?;
    let binding = state.bindings.get_mut(resource).ok_or(Denial::Missing)?;
    if binding.owner_scope == target.reference || &binding.reference == replacement {
        return Err(Denial::Conflict);
    }
    binding.owner_scope = target.reference.clone();
    binding.owner = target.owner.clone();
    binding.revision = binding.revision.checked_add(1).ok_or(Denial::Bound)?;
    binding.reference = replacement.clone();
    binding.operation = evidence.0.clone();
    binding.decision = evidence.1.clone();
    Ok(())
}
