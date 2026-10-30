use super::*;

pub fn parse_mutation(value: &preserves::IOValue) -> crate::error::Result<Mutation> {
    if value.collect_simple_record("scope-create-v1", Some(1)).is_some() {
        return Ok(Mutation::Create(parse_space(&decode::fields(value, "scope-create-v1", 1)?[0])?));
    }
    if value.collect_simple_record("scope-bind-v1", Some(1)).is_some() {
        return Ok(Mutation::Bind(parse_binding(&decode::fields(value, "scope-bind-v1", 1)?[0])?));
    }
    if value.collect_simple_record("scope-put-member-v1", Some(2)).is_some() {
        let f = decode::fields(value, "scope-put-member-v1", 2)?;
        return Ok(Mutation::PutMember {
            scope: parse_ref(&f[0])?,
            member: parse_membership(&f[1])?,
        });
    }
    for (label, is_expiry) in [("scope-remove-member-v1", false), ("scope-expire-member-v1", true)] {
        if value.collect_simple_record(label, Some(2)).is_some() {
            let f = decode::fields(value, label, 2)?;
            let scope = parse_ref(&f[0])?;
            let subject = SubjectRef::new(decode::string(&f[1])?).map_err(|_| denied(Denial::Domain))?;
            return Ok(if is_expiry {
                Mutation::ExpireMember { scope, subject }
            } else {
                Mutation::RemoveMember { scope, subject }
            });
        }
    }
    let f = decode::fields(value, "scope-move-v1", 3)?;
    Ok(Mutation::Move {
        resource: molten_core::nominal::ResourceRef::new(decode::string(&f[0])?).map_err(|_| denied(Denial::Domain))?,
        target: parse_ref(&f[1])?,
        replacement: BindingRef::new(decode::string(&f[2])?).map_err(|_| denied(Denial::Domain))?,
    })
}
fn parse_ref(value: &preserves::IOValue) -> crate::error::Result<ScopeRef> {
    ScopeRef::new(decode::string(value)?).map_err(|_| denied(Denial::Domain))
}

pub fn parse_mutation_request(value: &preserves::IOValue) -> crate::error::Result<MutationRequest> {
    let f = decode::fields(value, "scope-mutation-request-v1", 5)?;
    let reference =
        RequestRef::new(crate::preserves_rail::canonical_hash(value)?).map_err(|_| denied(Denial::Domain))?;
    Ok(MutationRequest {
        reference,
        operation: molten_core::nominal::OperationRef::new(decode::string(&f[0])?)
            .map_err(|_| denied(Denial::Domain))?,
        before: StateRef::new(decode::string(&f[1])?).map_err(|_| denied(Denial::Domain))?,
        actor: SubjectRef::new(decode::string(&f[2])?).map_err(|_| denied(Denial::Domain))?,
        currentness: parse_currentness(&f[3])?,
        mutation: parse_mutation(&f[4])?,
    })
}

pub fn parse_decision(value: &preserves::IOValue) -> crate::error::Result<AuthorityDecision> {
    let f = decode::fields(value, "scope-authority-decision-v1", 9)?;
    let is_allowed = match decode::string(&f[8])?.as_str() {
        "allowed" => true,
        "denied" => false,
        _ => return Err(denied(Denial::Domain)),
    };
    Ok(AuthorityDecision {
        request: RequestRef::new(decode::string(&f[0])?).map_err(|_| denied(Denial::Domain))?,
        subject: SubjectRef::new(decode::string(&f[1])?).map_err(|_| denied(Denial::Domain))?,
        policy: molten_core::nominal::PolicyRef::new(decode::string(&f[2])?).map_err(|_| denied(Denial::Domain))?,
        ability: parse_ability(&f[3])?,
        basalt: DecisionRef::new(decode::string(&f[4])?).map_err(|_| denied(Denial::Domain))?,
        ucan: DecisionRef::new(decode::string(&f[5])?).map_err(|_| denied(Denial::Domain))?,
        consent: decode::option(&f[6])?.map(DecisionRef::new).transpose().map_err(|_| denied(Denial::Domain))?,
        currentness: parse_currentness(&f[7])?,
        is_allowed,
    })
}

pub(crate) fn proposal_value(request: &MutationRequest, decisions: &[AuthorityDecision]) -> preserves::IOValue {
    crate::preserves_rail::record("collaboration-command-v1", vec![
        mutation_request_value(request),
        crate::preserves_rail::sequence(decisions.iter().map(decision_value).collect()),
    ])
}

/// Produces an untrusted mutation intent with no authority decisions. Only fresh
/// admitted authority acquisition may turn it into a proposed command.
pub fn intent_value(request: &MutationRequest) -> preserves::IOValue {
    proposal_value(request, &[])
}
pub(crate) fn parse_proposal(
    value: &preserves::IOValue,
) -> crate::error::Result<(MutationRequest, Vec<AuthorityDecision>)> {
    let f = decode::fields(value, "collaboration-command-v1", 2)?;
    let decisions = decode::items(&f[1], 6)?.iter().map(parse_decision).collect::<crate::error::Result<Vec<_>>>()?;
    Ok((parse_mutation_request(&f[0])?, decisions))
}
