//! Nominal reference domains for admitted authority and artifact values.
//!
//! Constructors validate category syntax only. They do not grant authority.
//!
//! r[impl molten.authority.nominal_references.types]
//! r[impl molten.authority.nominal_references.validation]
//! r[impl molten.authority.nominal_references.compile_time]
//! r[impl molten.authority.nominal_references.authority_tests]

const MAX_ENTITY_REF_BYTES: usize = 256;
const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_BYTES: usize = 64;

pub trait ReferenceDomain {
    const NAME: &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceError {
    Empty {
        domain: &'static str,
    },
    TooLong {
        domain: &'static str,
        actual: usize,
        maximum: usize,
    },
    InvalidSpelling {
        domain: &'static str,
    },
    UnsupportedAlgorithm {
        domain: &'static str,
    },
    WrongDigestLength {
        domain: &'static str,
        actual: usize,
        expected: usize,
    },
    UnknownRole {
        role: String,
    },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityRef<D: ReferenceDomain> {
    value: String,
    marker: core::marker::PhantomData<D>,
}

impl<D: ReferenceDomain> Clone for EntityRef<D> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            marker: core::marker::PhantomData,
        }
    }
}

impl<D: ReferenceDomain> EntityRef<D> {
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceError> {
        let value = value.into();
        validate_entity::<D>(&value)?;
        Ok(Self {
            value,
            marker: core::marker::PhantomData,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub const fn domain(&self) -> &'static str {
        D::NAME
    }

    pub fn into_string(self) -> String {
        self.value
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalRef<D: ReferenceDomain> {
    value: String,
    marker: core::marker::PhantomData<D>,
}

impl<D: ReferenceDomain> Clone for CanonicalRef<D> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            marker: core::marker::PhantomData,
        }
    }
}

impl<D: ReferenceDomain> CanonicalRef<D> {
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceError> {
        let value = value.into();
        validate_canonical::<D>(&value)?;
        Ok(Self {
            value,
            marker: core::marker::PhantomData,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub const fn domain(&self) -> &'static str {
        D::NAME
    }

    pub fn into_string(self) -> String {
        self.value
    }
}

macro_rules! domains {
    ($(($marker:ident, $alias:ident, $name:literal, $family:ident)),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub enum $marker {}
            impl ReferenceDomain for $marker {
                const NAME: &'static str = $name;
            }
            pub type $alias = $family<$marker>;
        )+
    };
}

domains!(
    (PrincipalDomain, PrincipalRef, "principal", EntityRef),
    (NodeDomain, NodeRef, "node", EntityRef),
    (ActorDomain, ActorRef, "actor", EntityRef),
    (ServiceDomain, ServiceRef, "service", EntityRef),
    (SessionDomain, SessionRef, "session", EntityRef),
    (AuthorityContextDomain, AuthorityContextRef, "authority-context", EntityRef),
    (DelegationDomain, DelegationRef, "delegation", CanonicalRef),
    (RevocationDomain, RevocationRef, "revocation", CanonicalRef),
    (KeyDomain, KeyRef, "key", CanonicalRef),
    (PolicyDomain, PolicyRef, "policy", CanonicalRef),
    (ResourceDomain, ResourceRef, "resource", CanonicalRef),
    (EvidenceDomain, EvidenceRef, "evidence", CanonicalRef),
    (ArtifactDomain, ArtifactRef, "artifact", CanonicalRef),
    (OperationDomain, OperationRef, "operation", EntityRef),
    (ReceiptDomain, ReceiptRef, "receipt", CanonicalRef),
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceRole {
    Principal,
    Node,
    Actor,
    Service,
    Session,
    AuthorityContext,
    Delegation,
    Revocation,
    Key,
    Policy,
    Resource,
    Evidence,
    Artifact,
    Operation,
    Receipt,
}

impl ReferenceRole {
    pub fn parse(value: &str) -> Result<Self, ReferenceError> {
        match value {
            "principal" => Ok(Self::Principal),
            "node" => Ok(Self::Node),
            "actor" => Ok(Self::Actor),
            "service" => Ok(Self::Service),
            "session" => Ok(Self::Session),
            "authority-context" => Ok(Self::AuthorityContext),
            "delegation" => Ok(Self::Delegation),
            "revocation" => Ok(Self::Revocation),
            "key" => Ok(Self::Key),
            "policy" => Ok(Self::Policy),
            "resource" => Ok(Self::Resource),
            "evidence" => Ok(Self::Evidence),
            "artifact" => Ok(Self::Artifact),
            "operation" => Ok(Self::Operation),
            "receipt" => Ok(Self::Receipt),
            _ => Err(ReferenceError::UnknownRole {
                role: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmittedReference {
    Principal(PrincipalRef),
    Node(NodeRef),
    Actor(ActorRef),
    Service(ServiceRef),
    Session(SessionRef),
    AuthorityContext(AuthorityContextRef),
    Delegation(DelegationRef),
    Revocation(RevocationRef),
    Key(KeyRef),
    Policy(PolicyRef),
    Resource(ResourceRef),
    Evidence(EvidenceRef),
    Artifact(ArtifactRef),
    Operation(OperationRef),
    Receipt(ReceiptRef),
}

pub fn admit_reference(role: ReferenceRole, value: &str) -> Result<AdmittedReference, ReferenceError> {
    let admitted = match role {
        ReferenceRole::Principal => AdmittedReference::Principal(PrincipalRef::new(value)?),
        ReferenceRole::Node => AdmittedReference::Node(NodeRef::new(value)?),
        ReferenceRole::Actor => AdmittedReference::Actor(ActorRef::new(value)?),
        ReferenceRole::Service => AdmittedReference::Service(ServiceRef::new(value)?),
        ReferenceRole::Session => AdmittedReference::Session(SessionRef::new(value)?),
        ReferenceRole::AuthorityContext => AdmittedReference::AuthorityContext(AuthorityContextRef::new(value)?),
        ReferenceRole::Delegation => AdmittedReference::Delegation(DelegationRef::new(value)?),
        ReferenceRole::Revocation => AdmittedReference::Revocation(RevocationRef::new(value)?),
        ReferenceRole::Key => AdmittedReference::Key(KeyRef::new(value)?),
        ReferenceRole::Policy => AdmittedReference::Policy(PolicyRef::new(value)?),
        ReferenceRole::Resource => AdmittedReference::Resource(ResourceRef::new(value)?),
        ReferenceRole::Evidence => AdmittedReference::Evidence(EvidenceRef::new(value)?),
        ReferenceRole::Artifact => AdmittedReference::Artifact(ArtifactRef::new(value)?),
        ReferenceRole::Operation => AdmittedReference::Operation(OperationRef::new(value)?),
        ReferenceRole::Receipt => AdmittedReference::Receipt(ReceiptRef::new(value)?),
    };
    Ok(admitted)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityReferenceSet {
    pub holder: PrincipalRef,
    pub session: SessionRef,
    pub context: AuthorityContextRef,
    pub delegation: DelegationRef,
    pub revocation: RevocationRef,
    pub key: KeyRef,
    pub policy: PolicyRef,
    pub resource: ResourceRef,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReferenceSet {
    pub node: NodeRef,
    pub actor: ActorRef,
    pub service: ServiceRef,
    pub session: SessionRef,
    pub operation: OperationRef,
    pub resource: ResourceRef,
}

// r[impl molten.authority.nominal_references.artifact_core.distinct]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReferenceSet {
    pub artifact: ArtifactRef,
    pub evidence: EvidenceRef,
    pub operation: OperationRef,
    pub receipt: ReceiptRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalReferenceSet {
    pub artifact: ArtifactRef,
    pub evidence: EvidenceRef,
    pub receipt: ReceiptRef,
    pub current_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityDecision {
    Allow,
    Deny,
}

pub fn decide_authority(
    supplied: &AuthorityReferenceSet,
    expected: &AuthorityReferenceSet,
    policy_allows: bool,
    is_expired: bool,
    is_revoked: bool,
) -> AuthorityDecision {
    let is_exact_roles_match = supplied == expected;
    if is_exact_roles_match && policy_allows && !is_expired && !is_revoked {
        AuthorityDecision::Allow
    } else {
        AuthorityDecision::Deny
    }
}

include!("parts/nominal/p000/body.rs");
