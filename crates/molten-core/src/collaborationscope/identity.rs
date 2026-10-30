//! Constructors establish syntax and category, never authority.
// r[impl molten.collaboration_scope.nominal_model]
macro_rules! domains {
    ($(($marker:ident, $alias:ident, $name:literal)),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub enum $marker {}
            impl crate::nominal::ReferenceDomain for $marker {
                const NAME: &'static str = $name;
            }
            pub type $alias = crate::nominal::CanonicalRef<$marker>;
        )+
    };
}

domains!(
    (ScopeDomain, ScopeRef, "collaboration-scope"),
    (SubjectDomain, SubjectRef, "collaboration-subject"),
    (MembershipDomain, MembershipRef, "collaboration-membership"),
    (BindingDomain, BindingRef, "collaboration-binding"),
    (AudienceDomain, AudienceRef, "collaboration-audience"),
    (RequestDomain, RequestRef, "collaboration-request"),
    (DecisionDomain, DecisionRef, "collaboration-decision"),
    (SnapshotDomain, SnapshotRef, "collaboration-snapshot"),
    (StateDomain, StateRef, "collaboration-state"),
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ability {
    Read,
    Use,
    Share,
    Bind,
    Move,
    Create,
    Membership,
}

impl Ability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "resource/read",
            Self::Use => "resource/use",
            Self::Share => "resource/share",
            Self::Bind => "resource/bind",
            Self::Move => "resource/move",
            Self::Create => "scope/create",
            Self::Membership => "scope/membership",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denial {
    Bound,
    Domain,
    Missing,
    Stale,
    Expired,
    Membership,
    Authority,
    Conflict,
    Owner,
    Replay,
    Uncertain,
    Partition,
}

impl Denial {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bound => "bound",
            Self::Domain => "domain",
            Self::Missing => "missing",
            Self::Stale => "stale",
            Self::Expired => "expired",
            Self::Membership => "membership",
            Self::Authority => "authority",
            Self::Conflict => "conflict",
            Self::Owner => "owner",
            Self::Replay => "replay",
            Self::Uncertain => "uncertain",
            Self::Partition => "partition",
        }
    }
}
