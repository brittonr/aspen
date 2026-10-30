use std::collections;

use super::*;

pub const MAX_SCOPES: usize = 128;
pub const MAX_MEMBERS: usize = 128;
pub const MAX_BINDINGS: usize = 1024;
pub const MAX_OPERATIONS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Personal,
    Conversation,
    Group,
    Organization,
}

impl ScopeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Conversation => "conversation",
            Self::Group => "group",
            Self::Organization => "organization",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipRole {
    Owner,
    Administrator,
    Participant,
    Viewer,
}

impl MembershipRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Administrator => "administrator",
            Self::Participant => "participant",
            Self::Viewer => "viewer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membership {
    pub reference: MembershipRef,
    pub subject: SubjectRef,
    pub role: MembershipRole,
    pub abilities: collections::BTreeSet<Ability>,
    pub start_epoch: u64,
    pub expires_at: Option<u64>,
    pub decision: DecisionRef,
    pub operation: crate::nominal::OperationRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationScope {
    pub reference: ScopeRef,
    pub kind: ScopeKind,
    pub owner: SubjectRef,
    pub revision: u64,
    pub membership_epoch: u64,
    pub policy: crate::nominal::PolicyRef,
    pub organization_floor: Option<crate::nominal::PolicyRef>,
    pub members: collections::BTreeMap<SubjectRef, Membership>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedResourceBinding {
    pub reference: BindingRef,
    pub resource: crate::nominal::ResourceRef,
    pub owner_scope: ScopeRef,
    pub owner: SubjectRef,
    pub revision: u64,
    pub policy: crate::nominal::PolicyRef,
    pub required_abilities: collections::BTreeSet<Ability>,
    pub operation: crate::nominal::OperationRef,
    pub decision: DecisionRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Currentness {
    pub service_generation: u64,
    pub consistency_epoch: u64,
    pub committed_index: u64,
    pub logical_now: u64,
    pub revocation: crate::nominal::RevocationRef,
    pub observation: crate::nominal::EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudienceRequest {
    pub reference: RequestRef,
    pub scope: ScopeRef,
    pub scope_revision: u64,
    pub membership_epoch: u64,
    pub binding: BindingRef,
    pub resource: crate::nominal::ResourceRef,
    pub audience: collections::BTreeSet<SubjectRef>,
    pub ability: Ability,
    pub target: Option<ScopeRef>,
    pub target_revision: Option<u64>,
    pub target_membership_epoch: Option<u64>,
    pub scope_policy: crate::nominal::PolicyRef,
    pub resource_policy: crate::nominal::PolicyRef,
    pub organization_floor: Option<crate::nominal::PolicyRef>,
    pub consent: Option<DecisionRef>,
    pub currentness: Currentness,
}

/// A loaded exact decision; the shell must obtain it from the admitted authority provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityDecision {
    pub request: RequestRef,
    pub subject: SubjectRef,
    pub policy: crate::nominal::PolicyRef,
    pub ability: Ability,
    pub basalt: DecisionRef,
    pub ucan: DecisionRef,
    pub consent: Option<DecisionRef>,
    pub currentness: Currentness,
    pub is_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeAdmissionSnapshot {
    pub request: AudienceRequest,
    pub decisions: Vec<AuthorityDecision>,
    pub state: StateRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorOperation {
    pub request: RequestRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub scopes: collections::BTreeMap<ScopeRef, CollaborationScope>,
    pub bindings: collections::BTreeMap<crate::nominal::ResourceRef, ScopedResourceBinding>,
    pub operations: collections::BTreeMap<crate::nominal::OperationRef, PriorOperation>,
    pub currentness: Currentness,
}
