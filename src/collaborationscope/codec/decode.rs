use super::*;

pub(crate) fn fields(
    value: &preserves::IOValue,
    label: &str,
    arity: usize,
) -> crate::error::Result<Vec<preserves::IOValue>> {
    Ok(crate::preserves_rail::simple_record_fields(value, label, arity as u64)?
        .fields_iter()
        .map(crate::preserves_rail::value_to_iovalue)
        .collect())
}
pub(crate) fn string(value: &preserves::IOValue) -> crate::error::Result<String> {
    value.as_string().map(|value| value.to_string()).ok_or_else(|| denied(Denial::Domain))
}
pub(crate) fn integer(value: &preserves::IOValue) -> crate::error::Result<u64> {
    value.as_u64().ok_or_else(|| denied(Denial::Domain))?.map_err(|_| denied(Denial::Bound))
}
pub(crate) fn items(value: &preserves::IOValue, maximum: usize) -> crate::error::Result<Vec<preserves::IOValue>> {
    let values = value.collect_sequence().ok_or_else(|| denied(Denial::Domain))?;
    if values.len() > maximum {
        return Err(denied(Denial::Bound));
    }
    Ok(values.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}
pub(crate) fn option(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    let values = value.collect_simple_record("optional", None).ok_or_else(|| denied(Denial::Domain))?;
    if values.fields_iter().count() > 1 {
        return Err(denied(Denial::Bound));
    }
    let values: Vec<_> = values.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect();
    values.first().map(string).transpose()
}
pub(crate) fn expiration(value: &preserves::IOValue) -> crate::error::Result<Option<u64>> {
    let values = value.collect_simple_record("expiry", None).ok_or_else(|| denied(Denial::Domain))?;
    if values.fields_iter().count() > 1 {
        return Err(denied(Denial::Bound));
    }
    let values: Vec<_> = values.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect();
    values.first().map(integer).transpose()
}
macro_rules! reference {
    ($type:ty, $value:expr) => {
        <$type>::new(string($value)?).map_err(|_| denied(Denial::Domain))?
    };
}

pub fn parse_ability(value: &preserves::IOValue) -> crate::error::Result<Ability> {
    match string(value)?.as_str() {
        "resource/read" => Ok(Ability::Read),
        "resource/use" => Ok(Ability::Use),
        "resource/share" => Ok(Ability::Share),
        "resource/bind" => Ok(Ability::Bind),
        "resource/move" => Ok(Ability::Move),
        "scope/create" => Ok(Ability::Create),
        "scope/membership" => Ok(Ability::Membership),
        _ => Err(denied(Denial::Domain)),
    }
}
fn abilities(value: &preserves::IOValue) -> crate::error::Result<std::collections::BTreeSet<Ability>> {
    let mut result = std::collections::BTreeSet::new();
    for value in items(value, 7)? {
        if !result.insert(parse_ability(&value)?) {
            return Err(denied(Denial::Conflict));
        }
    }
    Ok(result)
}
pub fn parse_currentness(value: &preserves::IOValue) -> crate::error::Result<Currentness> {
    let f = fields(value, "scope-currentness-v1", 6)?;
    Ok(Currentness {
        service_generation: integer(&f[0])?,
        consistency_epoch: integer(&f[1])?,
        committed_index: integer(&f[2])?,
        logical_now: integer(&f[3])?,
        revocation: reference!(molten_core::nominal::RevocationRef, &f[4]),
        observation: reference!(molten_core::nominal::EvidenceRef, &f[5]),
    })
}
pub fn parse_membership(value: &preserves::IOValue) -> crate::error::Result<Membership> {
    let f = fields(value, "scope-member-v1", 8)?;
    let role = match string(&f[7])?.as_str() {
        "owner" => MembershipRole::Owner,
        "administrator" => MembershipRole::Administrator,
        "participant" => MembershipRole::Participant,
        "viewer" => MembershipRole::Viewer,
        _ => return Err(denied(Denial::Domain)),
    };
    Ok(Membership {
        reference: reference!(MembershipRef, &f[0]),
        subject: reference!(SubjectRef, &f[1]),
        role,
        abilities: abilities(&f[2])?,
        start_epoch: integer(&f[3])?,
        expires_at: expiration(&f[4])?,
        decision: reference!(DecisionRef, &f[5]),
        operation: reference!(molten_core::nominal::OperationRef, &f[6]),
    })
}
pub fn parse_space(value: &preserves::IOValue) -> crate::error::Result<CollaborationScope> {
    let f = fields(value, "collaboration-scope-v1", 8)?;
    let kind = match string(&f[1])?.as_str() {
        "personal" => ScopeKind::Personal,
        "conversation" => ScopeKind::Conversation,
        "group" => ScopeKind::Group,
        "organization" => ScopeKind::Organization,
        _ => return Err(denied(Denial::Domain)),
    };
    let mut members = std::collections::BTreeMap::new();
    for value in items(&f[7], MAX_MEMBERS)? {
        let member = parse_membership(&value)?;
        if members.insert(member.subject.clone(), member).is_some() {
            return Err(denied(Denial::Conflict));
        }
    }
    Ok(CollaborationScope {
        reference: reference!(ScopeRef, &f[0]),
        kind,
        owner: reference!(SubjectRef, &f[2]),
        revision: integer(&f[3])?,
        membership_epoch: integer(&f[4])?,
        policy: reference!(molten_core::nominal::PolicyRef, &f[5]),
        organization_floor: option(&f[6])?
            .map(molten_core::nominal::PolicyRef::new)
            .transpose()
            .map_err(|_| denied(Denial::Domain))?,
        members,
    })
}
pub fn parse_binding(value: &preserves::IOValue) -> crate::error::Result<ScopedResourceBinding> {
    let f = fields(value, "scoped-resource-binding-v1", 9)?;
    Ok(ScopedResourceBinding {
        reference: reference!(BindingRef, &f[0]),
        resource: reference!(molten_core::nominal::ResourceRef, &f[1]),
        owner_scope: reference!(ScopeRef, &f[2]),
        owner: reference!(SubjectRef, &f[3]),
        revision: integer(&f[4])?,
        policy: reference!(molten_core::nominal::PolicyRef, &f[5]),
        required_abilities: abilities(&f[6])?,
        operation: reference!(molten_core::nominal::OperationRef, &f[7]),
        decision: reference!(DecisionRef, &f[8]),
    })
}

pub fn parse_state(value: &preserves::IOValue) -> crate::error::Result<State> {
    let f = fields(value, "collaboration-state-v1", 5)?;
    if string(&f[0])? != SCHEMA {
        return Err(denied(Denial::Domain));
    }
    let mut state = State {
        scopes: std::collections::BTreeMap::new(),
        bindings: std::collections::BTreeMap::new(),
        operations: std::collections::BTreeMap::new(),
        currentness: parse_currentness(&f[1])?,
    };
    for value in items(&f[2], MAX_SCOPES)? {
        let scope = parse_space(&value)?;
        if state.scopes.insert(scope.reference.clone(), scope).is_some() {
            return Err(denied(Denial::Conflict));
        }
    }
    for value in items(&f[3], MAX_BINDINGS)? {
        let binding = parse_binding(&value)?;
        if state.bindings.insert(binding.resource.clone(), binding).is_some() {
            return Err(denied(Denial::Conflict));
        }
    }
    for value in items(&f[4], MAX_OPERATIONS)? {
        let f = fields(&value, "scope-operation-v1", 2)?;
        let operation = reference!(molten_core::nominal::OperationRef, &f[0]);
        let prior = PriorOperation {
            request: reference!(RequestRef, &f[1]),
        };
        if state.operations.insert(operation, prior).is_some() {
            return Err(denied(Denial::Conflict));
        }
    }
    validate_state(&state).map_err(denied)?;
    Ok(state)
}
