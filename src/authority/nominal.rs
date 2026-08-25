//! Wire-to-core admission for nominal authority and artifact references.
//!
//! r[impl molten.authority.nominal_references.wire_boundary]
//! r[impl molten.authority.nominal_references.authority_core]
//! r[impl molten.authority.nominal_references.execution_core]
//! r[impl molten.authority.nominal_references.artifact_core]
//! r[impl molten.authority.nominal_references.evidence_core]
//! r[impl molten.authority.nominal_references.compatibility]
//! r[impl molten.authority.nominal_references.octet]

#![allow(
    tigerstyle::path_segment_repetition,
    reason = "nominal wire adapters retain explicit domain names in roundtrip and declaration tests"
)]

pub use molten_core::nominal::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityWireDto {
    pub holder: String,
    pub session: String,
    pub context: String,
    pub delegation: String,
    pub revocation: String,
    pub key: String,
    pub policy: String,
    pub resource: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionWireDto {
    pub node: String,
    pub actor: String,
    pub service: String,
    pub session: String,
    pub operation: String,
    pub resource: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactWireDto {
    pub artifact: String,
    pub evidence: String,
    pub operation: String,
    pub receipt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalWireDto {
    pub artifact: String,
    pub evidence: String,
    pub receipt: String,
    pub current_authority: bool,
}

pub fn admit_authority_wire(wire: &AuthorityWireDto) -> Result<AuthorityReferenceSet, ReferenceError> {
    Ok(AuthorityReferenceSet {
        holder: PrincipalRef::new(wire.holder.clone())?,
        session: SessionRef::new(wire.session.clone())?,
        context: AuthorityContextRef::new(wire.context.clone())?,
        delegation: DelegationRef::new(wire.delegation.clone())?,
        revocation: RevocationRef::new(wire.revocation.clone())?,
        key: KeyRef::new(wire.key.clone())?,
        policy: PolicyRef::new(wire.policy.clone())?,
        resource: ResourceRef::new(wire.resource.clone())?,
        evidence: EvidenceRef::new(wire.evidence.clone())?,
    })
}

pub fn admit_execution_wire(wire: &ExecutionWireDto) -> Result<ExecutionReferenceSet, ReferenceError> {
    Ok(ExecutionReferenceSet {
        node: NodeRef::new(wire.node.clone())?,
        actor: ActorRef::new(wire.actor.clone())?,
        service: ServiceRef::new(wire.service.clone())?,
        session: SessionRef::new(wire.session.clone())?,
        operation: OperationRef::new(wire.operation.clone())?,
        resource: ResourceRef::new(wire.resource.clone())?,
    })
}

pub fn admit_artifact_wire(wire: &ArtifactWireDto) -> Result<ArtifactReferenceSet, ReferenceError> {
    Ok(ArtifactReferenceSet {
        artifact: ArtifactRef::new(wire.artifact.clone())?,
        evidence: EvidenceRef::new(wire.evidence.clone())?,
        operation: OperationRef::new(wire.operation.clone())?,
        receipt: ReceiptRef::new(wire.receipt.clone())?,
    })
}

pub fn admit_historical_wire(wire: &HistoricalWireDto) -> Result<HistoricalReferenceSet, ReferenceError> {
    Ok(HistoricalReferenceSet {
        artifact: ArtifactRef::new(wire.artifact.clone())?,
        evidence: EvidenceRef::new(wire.evidence.clone())?,
        receipt: ReceiptRef::new(wire.receipt.clone())?,
        current_authority: wire.current_authority,
    })
}

pub fn project_authority_wire(core: &AuthorityReferenceSet) -> AuthorityWireDto {
    AuthorityWireDto {
        holder: core.holder.as_str().to_string(),
        session: core.session.as_str().to_string(),
        context: core.context.as_str().to_string(),
        delegation: core.delegation.as_str().to_string(),
        revocation: core.revocation.as_str().to_string(),
        key: core.key.as_str().to_string(),
        policy: core.policy.as_str().to_string(),
        resource: core.resource.as_str().to_string(),
        evidence: core.evidence.as_str().to_string(),
    }
}

pub fn project_execution_wire(core: &ExecutionReferenceSet) -> ExecutionWireDto {
    ExecutionWireDto {
        node: core.node.as_str().to_string(),
        actor: core.actor.as_str().to_string(),
        service: core.service.as_str().to_string(),
        session: core.session.as_str().to_string(),
        operation: core.operation.as_str().to_string(),
        resource: core.resource.as_str().to_string(),
    }
}

pub fn project_artifact_wire(core: &ArtifactReferenceSet) -> ArtifactWireDto {
    ArtifactWireDto {
        artifact: core.artifact.as_str().to_string(),
        evidence: core.evidence.as_str().to_string(),
        operation: core.operation.as_str().to_string(),
        receipt: core.receipt.as_str().to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedContextRefs {
    pub context: AuthorityContextRef,
    pub subject: PrincipalRef,
    pub delegations: Vec<DelegationRef>,
    pub revocations: Vec<RevocationRef>,
    pub keys: Vec<KeyRef>,
    pub policies: Vec<PolicyRef>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedCurrentnessRequest {
    pub principal: PrincipalRef,
    pub operation: OperationRef,
    pub current_keys: Vec<KeyRef>,
}

pub fn admit_context_refs(context: &super::Context) -> crate::error::Result<AdmittedContextRefs> {
    Ok(AdmittedContextRefs {
        context: AuthorityContextRef::new(context.context_ref.clone()).map_err(reference_error)?,
        subject: PrincipalRef::new(context.subject_ref.clone()).map_err(reference_error)?,
        delegations: admit_many(&context.delegation_refs, |value| DelegationRef::new(value))?,
        revocations: admit_many(&context.revocation_refs, |value| RevocationRef::new(value))?,
        keys: admit_many(&context.key_refs, |value| KeyRef::new(value))?,
        policies: admit_many(&context.policy_refs, |value| PolicyRef::new(value))?,
        evidence: admit_many(&context.evidence_refs, |value| EvidenceRef::new(value))?,
    })
}

pub fn admit_currentness_request(
    requested_principal_ref: &str,
    requested_operation: &str,
    current_key_refs: &[String],
) -> crate::error::Result<AdmittedCurrentnessRequest> {
    Ok(AdmittedCurrentnessRequest {
        principal: PrincipalRef::new(requested_principal_ref).map_err(reference_error)?,
        operation: OperationRef::new(requested_operation).map_err(reference_error)?,
        current_keys: admit_many(current_key_refs, |value| KeyRef::new(value))?,
    })
}

fn admit_many<D: ReferenceDomain, F>(values: &[String], constructor: F) -> crate::error::Result<Vec<CanonicalRef<D>>>
where F: Fn(&str) -> Result<CanonicalRef<D>, ReferenceError> {
    values.iter().map(|value| constructor(value).map_err(reference_error)).collect()
}

fn reference_error(error: ReferenceError) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("nominal reference admission failed: {error:?}"))
}

pub fn nominal_domain_declarations() -> &'static [(&'static str, &'static str)] {
    &[
        ("authority-holder", "PrincipalRef"),
        ("node-control-node", "NodeRef"),
        ("effect-session", "SessionRef"),
        ("authority-context", "AuthorityContextRef"),
        ("delegation", "DelegationRef"),
        ("revocation", "RevocationRef"),
        ("key", "KeyRef"),
        ("policy", "PolicyRef"),
        ("resource", "ResourceRef"),
        ("evidence", "EvidenceRef"),
        ("artifact", "ArtifactRef"),
        ("operation", "OperationRef"),
        ("receipt", "ReceiptRef"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const MIGRATED_DOMAIN_COUNT: usize = 13;

    fn authority_wire() -> AuthorityWireDto {
        AuthorityWireDto {
            holder: "principal-a".to_string(),
            session: "session-a".to_string(),
            context: "context-a".to_string(),
            delegation: HASH.to_string(),
            revocation: HASH.to_string(),
            key: HASH.to_string(),
            policy: HASH.to_string(),
            resource: HASH.to_string(),
            evidence: HASH.to_string(),
        }
    }

    #[test]
    fn authority_wire_roundtrip_preserves_exact_fields() {
        let wire = authority_wire();
        let core = admit_authority_wire(&wire).expect("admitted authority wire");
        assert_eq!(project_authority_wire(&core), wire);
    }

    #[test]
    fn execution_wire_roundtrip_preserves_node_session_operation_and_resource() {
        let wire = ExecutionWireDto {
            node: "node-a".to_string(),
            actor: "actor-a".to_string(),
            service: "service-a".to_string(),
            session: "session-a".to_string(),
            operation: "operation-a".to_string(),
            resource: HASH.to_string(),
        };
        let core = admit_execution_wire(&wire).expect("admitted execution wire");
        assert_eq!(project_execution_wire(&core), wire);
    }

    #[test]
    fn artifact_wire_roundtrip_preserves_equal_digest_bytes_under_distinct_roles() {
        let wire = ArtifactWireDto {
            artifact: HASH.to_string(),
            evidence: HASH.to_string(),
            operation: "operation-a".to_string(),
            receipt: HASH.to_string(),
        };
        let core = admit_artifact_wire(&wire).expect("admitted artifact wire");
        assert_eq!(core.artifact.domain(), "artifact");
        assert_eq!(core.evidence.domain(), "evidence");
        assert_eq!(project_artifact_wire(&core), wire);
    }

    #[test]
    fn cross_domain_wire_field_fails_before_core_use() {
        let mut wire = authority_wire();
        wire.evidence = "policy-a".to_string();
        assert!(matches!(
            admit_authority_wire(&wire),
            Err(ReferenceError::UnsupportedAlgorithm { domain: "evidence" })
        ));
    }

    #[test]
    fn historical_receipts_remain_evidence_only() {
        let wire = HistoricalWireDto {
            artifact: HASH.to_string(),
            evidence: HASH.to_string(),
            receipt: HASH.to_string(),
            current_authority: false,
        };
        let core = admit_historical_wire(&wire).expect("historical refs");
        assert!(historical_replay_is_evidence_only(&core));
    }

    #[test]
    fn declarations_cover_each_migrated_nominal_domain() {
        let declarations = nominal_domain_declarations();
        assert_eq!(declarations.len(), MIGRATED_DOMAIN_COUNT);
        assert!(declarations.iter().any(|row| row == &("policy", "PolicyRef")));
        assert!(declarations.iter().any(|row| row == &("artifact", "ArtifactRef")));
    }
}
