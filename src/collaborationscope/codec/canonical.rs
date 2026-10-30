use super::*;

pub const SCHEMA: &str = "molten.collaboration-scope.v1";
pub const MAX_RECORD_BYTES: u64 = 4 * 1024 * 1024;

fn text(value: &str) -> preserves::IOValue {
    crate::preserves_rail::string(value)
}
fn number(value: u64) -> preserves::IOValue {
    crate::preserves_rail::u64_value(value)
}
fn record(label: &'static str, values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::record(label, values)
}
fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values)
}
fn optional(value: Option<&str>) -> preserves::IOValue {
    record("optional", value.into_iter().map(text).collect())
}
fn expiry(value: Option<u64>) -> preserves::IOValue {
    record("expiry", value.into_iter().map(number).collect())
}

pub fn currentness_value(current: &Currentness) -> preserves::IOValue {
    record("scope-currentness-v1", vec![
        number(current.service_generation),
        number(current.consistency_epoch),
        number(current.committed_index),
        number(current.logical_now),
        text(current.revocation.as_str()),
        text(current.observation.as_str()),
    ])
}

pub fn membership_value(member: &Membership) -> preserves::IOValue {
    record("scope-member-v1", vec![
        text(member.reference.as_str()),
        text(member.subject.as_str()),
        sequence(member.abilities.iter().map(|ability| text(ability.as_str())).collect()),
        number(member.start_epoch),
        expiry(member.expires_at),
        text(member.decision.as_str()),
        text(member.operation.as_str()),
        text(member.role.as_str()),
    ])
}

pub fn space_value(scope: &CollaborationScope) -> preserves::IOValue {
    record("collaboration-scope-v1", vec![
        text(scope.reference.as_str()),
        text(scope.kind.as_str()),
        text(scope.owner.as_str()),
        number(scope.revision),
        number(scope.membership_epoch),
        text(scope.policy.as_str()),
        optional(scope.organization_floor.as_ref().map(|policy| policy.as_str())),
        sequence(scope.members.values().map(membership_value).collect()),
    ])
}

pub fn binding_value(binding: &ScopedResourceBinding) -> preserves::IOValue {
    record("scoped-resource-binding-v1", vec![
        text(binding.reference.as_str()),
        text(binding.resource.as_str()),
        text(binding.owner_scope.as_str()),
        text(binding.owner.as_str()),
        number(binding.revision),
        text(binding.policy.as_str()),
        sequence(binding.required_abilities.iter().map(|ability| text(ability.as_str())).collect()),
        text(binding.operation.as_str()),
        text(binding.decision.as_str()),
    ])
}

pub fn state_value(state: &State) -> preserves::IOValue {
    record("collaboration-state-v1", vec![
        text(SCHEMA),
        currentness_value(&state.currentness),
        sequence(state.scopes.values().map(space_value).collect()),
        sequence(state.bindings.values().map(binding_value).collect()),
        sequence(
            state
                .operations
                .iter()
                .map(|(operation, prior)| {
                    record("scope-operation-v1", vec![text(operation.as_str()), text(prior.request.as_str())])
                })
                .collect(),
        ),
    ])
}

pub fn validated_state_ref(state: &State) -> crate::error::Result<StateRef> {
    validate_state(state).map_err(denied)?;
    StateRef::new(crate::preserves_rail::canonical_hash(&state_value(state))?).map_err(|_| denied(Denial::Domain))
}

pub fn mutation_value(mutation: &Mutation) -> preserves::IOValue {
    match mutation {
        Mutation::Create(scope) => record("scope-create-v1", vec![space_value(scope)]),
        Mutation::PutMember { scope, member } => {
            record("scope-put-member-v1", vec![text(scope.as_str()), membership_value(member)])
        }
        Mutation::RemoveMember { scope, subject } => {
            record("scope-remove-member-v1", vec![text(scope.as_str()), text(subject.as_str())])
        }
        Mutation::ExpireMember { scope, subject } => {
            record("scope-expire-member-v1", vec![text(scope.as_str()), text(subject.as_str())])
        }
        Mutation::Bind(binding) => record("scope-bind-v1", vec![binding_value(binding)]),
        Mutation::Move {
            resource,
            target,
            replacement,
        } => record("scope-move-v1", vec![
            text(resource.as_str()),
            text(target.as_str()),
            text(replacement.as_str()),
        ]),
    }
}

pub fn mutation_request_value(request: &MutationRequest) -> preserves::IOValue {
    record("scope-mutation-request-v1", vec![
        text(request.operation.as_str()),
        text(request.before.as_str()),
        text(request.actor.as_str()),
        currentness_value(&request.currentness),
        mutation_value(&request.mutation),
    ])
}

pub fn mutation_request_ref(request: &MutationRequest) -> crate::error::Result<RequestRef> {
    RequestRef::new(crate::preserves_rail::canonical_hash(&mutation_request_value(request))?)
        .map_err(|_| denied(Denial::Domain))
}

pub fn audience_value(request: &AudienceRequest) -> preserves::IOValue {
    record("scope-audience-request-v1", vec![
        text(request.scope.as_str()),
        number(request.scope_revision),
        number(request.membership_epoch),
        text(request.binding.as_str()),
        text(request.resource.as_str()),
        sequence(request.audience.iter().map(|subject| text(subject.as_str())).collect()),
        text(request.ability.as_str()),
        optional(request.target.as_ref().map(|target| target.as_str())),
        expiry(request.target_revision),
        expiry(request.target_membership_epoch),
        text(request.scope_policy.as_str()),
        text(request.resource_policy.as_str()),
        optional(request.organization_floor.as_ref().map(|policy| policy.as_str())),
        optional(request.consent.as_ref().map(|consent| consent.as_str())),
        currentness_value(&request.currentness),
    ])
}

pub fn audience_request_ref(request: &AudienceRequest) -> crate::error::Result<RequestRef> {
    RequestRef::new(crate::preserves_rail::canonical_hash(&audience_value(request))?)
        .map_err(|_| denied(Denial::Domain))
}

pub fn decision_value(fact: &AuthorityDecision) -> preserves::IOValue {
    record("scope-authority-decision-v1", vec![
        text(fact.request.as_str()),
        text(fact.subject.as_str()),
        text(fact.policy.as_str()),
        text(fact.ability.as_str()),
        text(fact.basalt.as_str()),
        text(fact.ucan.as_str()),
        optional(fact.consent.as_ref().map(|consent| consent.as_str())),
        currentness_value(&fact.currentness),
        text(if fact.is_allowed { "allowed" } else { "denied" }),
    ])
}

// r[impl molten.collaboration_scope.safe_projection]
// r[impl molten.collaboration_scope.authority_snapshot]
pub fn snapshot_value(snapshot: &ScopeAdmissionSnapshot) -> preserves::IOValue {
    record("scope-admission-snapshot-v1", vec![
        audience_value(&snapshot.request),
        text(snapshot.request.reference.as_str()),
        text(snapshot.state.as_str()),
        sequence(snapshot.decisions.iter().map(decision_value).collect()),
        text("one-request-evidence-not-bearer-authority"),
    ])
}

pub fn snapshot_ref(snapshot: &ScopeAdmissionSnapshot) -> crate::error::Result<SnapshotRef> {
    SnapshotRef::new(crate::preserves_rail::canonical_hash(&snapshot_value(snapshot))?)
        .map_err(|_| denied(Denial::Domain))
}

pub fn status_value(state: &State, state_ref: &StateRef) -> preserves::IOValue {
    record("collaboration-status-v1", vec![
        text(state_ref.as_str()),
        currentness_value(&state.currentness),
        sequence(
            state
                .scopes
                .values()
                .map(|scope| {
                    record("scope-status-v1", vec![
                        text(scope.reference.as_str()),
                        number(scope.revision),
                        number(scope.membership_epoch),
                        number(scope.members.len() as u64),
                        text("active"),
                    ])
                })
                .collect(),
        ),
        number(state.bindings.len() as u64),
        text("not-global-revocation-or-confidentiality-or-release-proof"),
    ])
}

pub(crate) fn denied(denial: Denial) -> crate::error::Failure {
    let class = match denial {
        Denial::Bound => "bound",
        Denial::Domain => "domain",
        Denial::Missing => "missing",
        Denial::Stale => "stale",
        Denial::Expired => "expired",
        Denial::Membership => "membership",
        Denial::Authority => "authority",
        Denial::Conflict => "conflict",
        Denial::Owner => "owner",
        Denial::Replay => "replay",
        Denial::Uncertain => "uncertain",
        Denial::Partition => "partition",
    };
    crate::error::Failure::invalid_harness(format!("collaboration-scope/{class}"))
}
