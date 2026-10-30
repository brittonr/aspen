use super::*;

// r[impl molten.collaboration_scope.effective_audience]
// r[impl molten.collaboration_scope.nontransitive_sharing]
pub fn admit_audience(
    state: &State,
    state_ref: &StateRef,
    request: &AudienceRequest,
    decisions: &[AuthorityDecision],
) -> Result<ScopeAdmissionSnapshot, Denial> {
    validate_state(state)?;
    if !currentness_extends(&state.currentness, &request.currentness) {
        return Err(Denial::Stale);
    }
    let scope = state.scopes.get(&request.scope).ok_or(Denial::Missing)?;
    let binding = state.bindings.get(&request.resource).ok_or(Denial::Missing)?;
    validate_request(scope, binding, request)?;
    if request.audience.is_empty() || request.audience.len() > MAX_MEMBERS || decisions.len() > MAX_MEMBERS * 5 {
        return Err(Denial::Bound);
    }
    let policies = required_policies(state, request)?;
    if decisions.len() != request.audience.len() * policies.len() {
        return Err(Denial::Authority);
    }
    for subject in &request.audience {
        if let Some(target) = &request.target {
            let target = state.scopes.get(target).ok_or(Denial::Missing)?;
            let member = target.members.get(subject).ok_or(Denial::Membership)?;
            if !member.abilities.contains(&request.ability)
                || member.expires_at.is_some_and(|expiry| expiry <= request.currentness.logical_now)
            {
                return Err(Denial::Membership);
            }
        }
        let member = scope.members.get(subject).ok_or(Denial::Membership)?;
        if member.start_epoch > scope.membership_epoch || !member.abilities.contains(&request.ability) {
            return Err(Denial::Membership);
        }
        if member.expires_at.is_some_and(|expiry| expiry <= request.currentness.logical_now) {
            return Err(Denial::Expired);
        }
        for policy in &policies {
            let mut matching = decisions.iter().filter(|fact| &fact.subject == subject && &fact.policy == *policy);
            let fact = matching.next().ok_or(Denial::Authority)?;
            if matching.next().is_some() || !decision_matches(fact, request) {
                return Err(Denial::Authority);
            }
        }
    }
    Ok(ScopeAdmissionSnapshot {
        request: request.clone(),
        decisions: decisions.to_vec(),
        state: state_ref.clone(),
    })
}

pub fn required_policies<'a>(
    state: &'a State,
    request: &'a AudienceRequest,
) -> Result<Vec<&'a crate::nominal::PolicyRef>, Denial> {
    let mut policies = vec![&request.scope_policy, &request.resource_policy];
    if let Some(floor) = &request.organization_floor {
        policies.push(floor);
    }
    if let Some(target) = &request.target {
        let scope = state.scopes.get(target).ok_or(Denial::Missing)?;
        if Some(scope.revision) != request.target_revision
            || Some(scope.membership_epoch) != request.target_membership_epoch
        {
            return Err(Denial::Stale);
        }
        policies.push(&scope.policy);
        if let Some(floor) = &scope.organization_floor {
            policies.push(floor);
        }
    }
    policies.sort();
    policies.dedup();
    Ok(policies)
}

fn decision_matches(fact: &AuthorityDecision, request: &AudienceRequest) -> bool {
    fact.is_allowed
        && fact.request == request.reference
        && fact.ability == request.ability
        && fact.currentness == request.currentness
        && fact.consent == request.consent
}

fn validate_request(
    scope: &CollaborationScope,
    binding: &ScopedResourceBinding,
    request: &AudienceRequest,
) -> Result<(), Denial> {
    if request.scope_revision != scope.revision
        || request.membership_epoch != scope.membership_epoch
        || binding.reference != request.binding
        || binding.owner_scope != scope.reference
        || request.scope_policy != scope.policy
        || request.resource_policy != binding.policy
        || request.organization_floor != scope.organization_floor
    {
        return Err(Denial::Stale);
    }
    if !binding.required_abilities.contains(&request.ability) {
        return Err(Denial::Authority);
    }
    if matches!(request.ability, Ability::Share | Ability::Move) {
        if request.target.is_none() || request.target_revision.is_none() || request.target_membership_epoch.is_none() {
            return Err(Denial::Missing);
        }
    } else if request.target.is_some() || request.target_revision.is_some() || request.target_membership_epoch.is_some()
    {
        return Err(Denial::Authority);
    }
    Ok(())
}

// r[impl molten.collaboration_scope.authority_snapshot]
pub fn revalidate_snapshot(
    state: &State,
    state_ref: &StateRef,
    snapshot: &ScopeAdmissionSnapshot,
    current_decisions: &[AuthorityDecision],
) -> Result<(), Denial> {
    if &snapshot.state != state_ref || snapshot.decisions != current_decisions {
        return Err(Denial::Stale);
    }
    admit_audience(state, state_ref, &snapshot.request, current_decisions).map(|_| ())
}

/// The shell supplies a quorum-current observation. The pure core additionally
/// rejects rollback relative to the last committed engine and logical position.
pub fn currentness_extends(previous: &Currentness, current: &Currentness) -> bool {
    previous.service_generation == current.service_generation
        && current.consistency_epoch >= previous.consistency_epoch
        && current.committed_index == previous.committed_index
        && current.logical_now >= previous.logical_now
}

pub fn validate_state(state: &State) -> Result<(), Denial> {
    if state.scopes.len() > MAX_SCOPES || state.bindings.len() > MAX_BINDINGS || state.operations.len() > MAX_OPERATIONS
    {
        return Err(Denial::Bound);
    }
    if state.currentness.service_generation == 0 || state.currentness.consistency_epoch == 0 {
        return Err(Denial::Stale);
    }
    for (reference, scope) in &state.scopes {
        if reference != &scope.reference
            || scope.members.len() > MAX_MEMBERS
            || scope.members.is_empty()
            || scope.revision == 0
            || scope.membership_epoch == 0
            || !scope.members.contains_key(&scope.owner)
        {
            return Err(Denial::Membership);
        }
        for (subject, member) in &scope.members {
            if subject != &member.subject
                || member.abilities.is_empty()
                || member.start_epoch == 0
                || member.start_epoch > scope.membership_epoch
                || (member.role == MembershipRole::Owner) != (subject == &scope.owner)
            {
                return Err(Denial::Membership);
            }
            if scope.kind == ScopeKind::Personal && subject != &scope.owner {
                return Err(Denial::Membership);
            }
        }
    }
    for (resource, binding) in &state.bindings {
        let scope = state.scopes.get(&binding.owner_scope).ok_or(Denial::Owner)?;
        if resource != &binding.resource
            || scope.owner != binding.owner
            || binding.revision == 0
            || binding.required_abilities.is_empty()
        {
            return Err(Denial::Owner);
        }
    }
    Ok(())
}
