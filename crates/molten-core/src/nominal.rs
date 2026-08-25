//! Nominal reference domains for admitted authority and artifact values.
//!
//! Constructors validate category syntax only. They do not grant authority.
//!
//! r[impl molten.authority.nominal_references.types]
//! r[impl molten.authority.nominal_references.validation]
//! r[impl molten.authority.nominal_references.compile_time]
//! r[impl molten.authority.nominal_references.authority_tests]

use core::marker::PhantomData;

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
    marker: PhantomData<D>,
}

impl<D: ReferenceDomain> Clone for EntityRef<D> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            marker: PhantomData,
        }
    }
}

impl<D: ReferenceDomain> EntityRef<D> {
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceError> {
        let value = value.into();
        validate_entity::<D>(&value)?;
        Ok(Self {
            value,
            marker: PhantomData,
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
    marker: PhantomData<D>,
}

impl<D: ReferenceDomain> Clone for CanonicalRef<D> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            marker: PhantomData,
        }
    }
}

impl<D: ReferenceDomain> CanonicalRef<D> {
    pub fn new(value: impl Into<String>) -> Result<Self, ReferenceError> {
        let value = value.into();
        validate_canonical::<D>(&value)?;
        Ok(Self {
            value,
            marker: PhantomData,
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
    let exact_roles_match = supplied == expected;
    if exact_roles_match && policy_allows && !is_expired && !is_revoked {
        AuthorityDecision::Allow
    } else {
        AuthorityDecision::Deny
    }
}

pub fn historical_replay_is_evidence_only(references: &HistoricalReferenceSet) -> bool {
    !references.current_authority
}

/// Requires a session-domain reference.
///
/// ```compile_fail
/// use molten_core::nominal::{AuthorityContextRef, SessionRef, require_session};
/// let context = AuthorityContextRef::new("context-a").unwrap();
/// let _: &SessionRef = require_session(&context);
/// ```
///
/// ```compile_fail
/// use molten_core::nominal::{EvidenceRef, PolicyRef};
/// let evidence = EvidenceRef::new("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
/// let _: PolicyRef = evidence;
/// ```
///
/// ```compile_fail
/// use molten_core::nominal::{DelegationRef, RevocationRef};
/// let delegation = DelegationRef::new("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
/// let _: RevocationRef = delegation;
/// ```
///
/// ```compile_fail
/// use molten_core::nominal::{AuthorityContextRef, KeyRef};
/// let key = KeyRef::new("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
/// let _: AuthorityContextRef = key;
/// ```
///
/// ```compile_fail
/// use molten_core::nominal::{ArtifactRef, ReceiptRef};
/// let artifact = ArtifactRef::new("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
/// let _: ReceiptRef = artifact;
/// ```
///
/// ```compile_fail
/// use molten_core::nominal::{OperationRef, ResourceRef};
/// let operation = OperationRef::new("operation-a").unwrap();
/// let _: ResourceRef = operation;
/// ```
///
/// ```compile_fail
/// use molten_core::nominal::{NodeRef, PrincipalRef};
/// let node = NodeRef::new("node-a").unwrap();
/// let _: PrincipalRef = node;
/// ```
pub const fn require_session(reference: &SessionRef) -> &SessionRef {
    reference
}

fn validate_entity<D: ReferenceDomain>(value: &str) -> Result<(), ReferenceError> {
    if value.is_empty() {
        return Err(ReferenceError::Empty { domain: D::NAME });
    }
    if value.len() > MAX_ENTITY_REF_BYTES {
        return Err(ReferenceError::TooLong {
            domain: D::NAME,
            actual: value.len(),
            maximum: MAX_ENTITY_REF_BYTES,
        });
    }
    let valid = value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
    });
    if !valid || value.starts_with(['-', '_', '.', ':', '/']) || value.ends_with(['-', '_', '.', ':', '/']) {
        return Err(ReferenceError::InvalidSpelling { domain: D::NAME });
    }
    Ok(())
}

fn validate_canonical<D: ReferenceDomain>(value: &str) -> Result<(), ReferenceError> {
    let Some(hex) = value.strip_prefix(BLAKE3_PREFIX) else {
        return Err(ReferenceError::UnsupportedAlgorithm { domain: D::NAME });
    };
    if hex.len() != BLAKE3_HEX_BYTES {
        return Err(ReferenceError::WrongDigestLength {
            domain: D::NAME,
            actual: hex.len(),
            expected: BLAKE3_HEX_BYTES,
        });
    }
    if !hex.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')) {
        return Err(ReferenceError::InvalidSpelling { domain: D::NAME });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn authority_set() -> AuthorityReferenceSet {
        AuthorityReferenceSet {
            holder: PrincipalRef::new("principal-a").expect("principal"),
            session: SessionRef::new("session-a").expect("session"),
            context: AuthorityContextRef::new("context-a").expect("context"),
            delegation: DelegationRef::new(HASH).expect("delegation"),
            revocation: RevocationRef::new(HASH).expect("revocation"),
            key: KeyRef::new(HASH).expect("key"),
            policy: PolicyRef::new(HASH).expect("policy"),
            resource: ResourceRef::new(HASH).expect("resource"),
            evidence: EvidenceRef::new(HASH).expect("evidence"),
        }
    }

    #[test]
    fn same_domain_values_are_admitted_and_accessible() {
        let session = SessionRef::new("session-a").expect("session");
        assert_eq!(session.as_str(), "session-a");
        assert_eq!(session.domain(), "session");
        assert_eq!(require_session(&session), &session);
    }

    #[test]
    fn entity_construction_rejects_empty_oversized_and_noncanonical_values() {
        assert!(matches!(SessionRef::new(""), Err(ReferenceError::Empty { .. })));
        assert!(matches!(SessionRef::new("a".repeat(MAX_ENTITY_REF_BYTES + 1)), Err(ReferenceError::TooLong { .. })));
        assert!(matches!(SessionRef::new("Session A"), Err(ReferenceError::InvalidSpelling { .. })));
    }

    #[test]
    fn canonical_construction_rejects_algorithm_length_and_spelling_drift() {
        assert!(matches!(PolicyRef::new("sha256:aaaa"), Err(ReferenceError::UnsupportedAlgorithm { .. })));
        assert!(matches!(PolicyRef::new("blake3:aa"), Err(ReferenceError::WrongDigestLength { .. })));
        let uppercase = format!("blake3:{}", "A".repeat(BLAKE3_HEX_BYTES));
        assert!(matches!(PolicyRef::new(uppercase), Err(ReferenceError::InvalidSpelling { .. })));
    }

    #[test]
    fn wire_admission_preserves_exact_external_text() {
        let AdmittedReference::Artifact(admitted) = admit_reference(ReferenceRole::Artifact, HASH).expect("artifact")
        else {
            panic!("wrong admitted role")
        };
        assert_eq!(admitted.as_str().as_bytes(), HASH.as_bytes());
    }

    #[test]
    fn wire_admission_rejects_unknown_roles_and_role_grammar_mismatch() {
        assert!(matches!(ReferenceRole::parse("transport-ticket"), Err(ReferenceError::UnknownRole { .. })));
        assert!(matches!(
            admit_reference(ReferenceRole::Evidence, "policy-a"),
            Err(ReferenceError::UnsupportedAlgorithm { .. })
        ));
    }

    #[test]
    fn valid_reference_possession_does_not_bypass_authority_policy() {
        let supplied = authority_set();
        assert_eq!(decide_authority(&supplied, &supplied, false, false, false), AuthorityDecision::Deny);
        assert_eq!(decide_authority(&supplied, &supplied, true, true, false), AuthorityDecision::Deny);
        assert_eq!(decide_authority(&supplied, &supplied, true, false, true), AuthorityDecision::Deny);
        assert_eq!(decide_authority(&supplied, &supplied, true, false, false), AuthorityDecision::Allow);
    }

    #[test]
    fn exact_authority_roles_fail_closed_on_holder_session_and_policy_drift() {
        let expected = authority_set();
        let mut supplied = expected.clone();
        supplied.holder = PrincipalRef::new("principal-b").expect("principal");
        assert_eq!(decide_authority(&supplied, &expected, true, false, false), AuthorityDecision::Deny);
        supplied = expected.clone();
        supplied.session = SessionRef::new("session-b").expect("session");
        assert_eq!(decide_authority(&supplied, &expected, true, false, false), AuthorityDecision::Deny);
        supplied = expected.clone();
        supplied.policy =
            PolicyRef::new("blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").expect("policy");
        assert_eq!(decide_authority(&supplied, &expected, true, false, false), AuthorityDecision::Deny);
    }

    #[test]
    fn artifact_evidence_operation_and_receipt_roles_remain_distinct() {
        let links = ArtifactReferenceSet {
            artifact: ArtifactRef::new(HASH).expect("artifact"),
            evidence: EvidenceRef::new(HASH).expect("evidence"),
            operation: OperationRef::new("operation-a").expect("operation"),
            receipt: ReceiptRef::new(HASH).expect("receipt"),
        };
        assert_eq!(links.artifact.as_str(), links.evidence.as_str());
        assert_eq!(links.artifact.domain(), "artifact");
        assert_eq!(links.evidence.domain(), "evidence");
        assert_eq!(links.receipt.domain(), "receipt");
    }

    #[test]
    fn historical_replay_never_mints_current_authority() {
        let historical = HistoricalReferenceSet {
            artifact: ArtifactRef::new(HASH).expect("artifact"),
            evidence: EvidenceRef::new(HASH).expect("evidence"),
            receipt: ReceiptRef::new(HASH).expect("receipt"),
            current_authority: false,
        };
        assert!(historical_replay_is_evidence_only(&historical));
    }
}
