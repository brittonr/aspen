const STANDALONE_THRESHOLD_ONE: u16 = 1;
const BLAKE3_REF_PREFIX: &str = "blake3:";
const MOLTEN_CURRENTNESS_PROFILE: &str = "molten-key-currentness.v1";
const MOLTEN_VERIFIER_CONTEXT_PROFILE: &str = "molten-verifier-context.v1";
const STANDALONE_FAILURE_CODE: &str = "molten-supplied-cryptographic-verification-failed";
const PREIMAGE_CLASS: &str = "distinct-canonical-preimages";
const ISSUE_CLASS_PARITY: &str = "no-issues";
const ISSUE_CLASS_MAPPED_REJECTION: &str = "consumer-specific-taxonomy";
const AUTHORITY_BOUNDARY: &str = "standalone authentication is diagnostic input only; Molten retains key generation, signing, storage, capability, federation, transport, runtime, evidence, deployment, and release authority";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoltenArtifactAuthStatementInput<'a> {
    pub profile: &'a super::CryptoAdapterProfile,
    pub request: &'a super::VerificationRequest,
    pub producer_id: &'a str,
    pub key_id: &'a str,
    pub currentness_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoltenArtifactAuthObservation<'a> {
    pub profile: &'a super::CryptoAdapterProfile,
    pub request: &'a super::VerificationRequest,
    pub producer_id: &'a str,
    pub key_id: &'a str,
    pub currentness_ref: &'a str,
    pub standalone_cryptographic: artifact_auth_core::CryptographicObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoltenArtifactAuthCompatibility {
    pub case_explained: bool,
    pub preimage_class: String,
    pub identity_drift_explained: bool,
    pub decision_drift: bool,
    pub issue_class: String,
    pub mapped_failure_causes: Vec<String>,
    pub standalone_failure_causes: Vec<String>,
    pub non_claim_drift: bool,
    pub blockers: Vec<String>,
    pub legacy_authoritative: bool,
    pub standalone_authority_admitted: bool,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoltenArtifactAuthReport {
    pub legacy: super::VerificationDecision,
    pub standalone: Option<artifact_auth_core::AuthenticationDecision>,
    pub compatibility: MoltenArtifactAuthCompatibility,
    pub opaque_handle_authority_retained: bool,
    pub backend_authority_retained: bool,
    pub rotation_authority_retained: bool,
    pub authority_boundary: String,
}

// r[impl molten.artifact_auth_adoption.authority]
// r[impl molten.artifact_auth_adoption.cutover]
#[must_use]
pub fn evaluate_artifact_auth_dual_run(observation: &MoltenArtifactAuthObservation<'_>) -> MoltenArtifactAuthReport {
    let legacy = super::evaluate_verification(observation.profile, observation.request);
    let mapped = map_observation(observation);
    let standalone = mapped
        .as_ref()
        .ok()
        .map(|(policy, scope, evidence)| artifact_auth_core::evaluate_authentication(policy, scope, evidence));
    let mapping_blockers = mapped.err().unwrap_or_default();
    let compatibility = compare_decisions(observation, &legacy, standalone.as_ref(), mapping_blockers);
    MoltenArtifactAuthReport {
        legacy,
        standalone,
        compatibility,
        opaque_handle_authority_retained: true,
        backend_authority_retained: true,
        rotation_authority_retained: true,
        authority_boundary: AUTHORITY_BOUNDARY.to_string(),
    }
}

// r[impl molten.artifact_auth_shell.exact_verification]
/// Map Molten observations to the exact signer-specific standalone statement.
///
/// This pure mapping performs no signing or verification and never consumes the
/// legacy cryptographic decision as standalone proof.
pub fn map_artifact_auth_statement(
    input: &MoltenArtifactAuthStatementInput<'_>,
) -> Result<artifact_auth_core::ArtifactStatement, Vec<String>> {
    let (_, _, statement) = map_statement_and_policy(input)?;
    Ok(statement)
}

fn map_observation(
    observation: &MoltenArtifactAuthObservation<'_>,
) -> Result<
    (
        artifact_auth_core::AuthenticationPolicy,
        artifact_auth_core::AuthenticationScope,
        Vec<artifact_auth_core::SignatureEvidence>,
    ),
    Vec<String>,
> {
    let input = MoltenArtifactAuthStatementInput {
        profile: observation.profile,
        request: observation.request,
        producer_id: observation.producer_id,
        key_id: observation.key_id,
        currentness_ref: observation.currentness_ref,
    };
    let (policy, scope, statement) = map_statement_and_policy(&input)?;
    let evidence = vec![artifact_auth_core::SignatureEvidence {
        statement,
        generation: observation.request.observed.generation,
        cryptographic: observation.standalone_cryptographic.clone(),
    }];
    Ok((policy, scope, evidence))
}

fn map_statement_and_policy(
    input: &MoltenArtifactAuthStatementInput<'_>,
) -> Result<
    (
        artifact_auth_core::AuthenticationPolicy,
        artifact_auth_core::AuthenticationScope,
        artifact_auth_core::ArtifactStatement,
    ),
    Vec<String>,
> {
    let request = input.request;
    let key_identity = parse_ref(RefInput {
        profile: artifact_auth_core::ED25519_PUBLIC_KEY_PROFILE_V1,
        value: &request.observed.signer_public_ref,
        field: "observed.signer_public_ref",
    })?;
    let subject = parse_ref(RefInput {
        profile: &request.expected_domain.payload_schema,
        value: &request.expected_domain.payload_ref,
        field: "expected_domain.payload_ref",
    })?;
    let verifier_context = parse_ref(RefInput {
        profile: MOLTEN_VERIFIER_CONTEXT_PROFILE,
        value: &request.expected_domain.verifier_context_ref,
        field: "expected_domain.verifier_context_ref",
    })?;
    let currentness_ref = parse_ref(RefInput {
        profile: MOLTEN_CURRENTNESS_PROFILE,
        value: input.currentness_ref,
        field: "currentness_ref",
    })?;
    if input.profile.algorithm != super::CryptoAlgorithm::Ed25519Iroh {
        return Err(vec!["unsupported-production-algorithm".to_string()]);
    }
    let scope = artifact_auth_core::AuthenticationScope {
        domain: request.expected_domain.domain_id.clone(),
        purpose: request.expected_domain.purpose.as_str().to_string(),
        profile_id: input.profile.profile_id.clone(),
        subject,
        parents: Vec::new(),
        verifier_context,
    };
    let statement = artifact_auth_core::ArtifactStatement {
        schema: artifact_auth_core::STATEMENT_SCHEMA_V1.to_string(),
        scope: scope.clone(),
        producer_id: input.producer_id.to_string(),
        key_id: input.key_id.to_string(),
        key_identity: key_identity.clone(),
    };
    let policy = artifact_auth_core::AuthenticationPolicy {
        schema: artifact_auth_core::POLICY_SCHEMA_V1.to_string(),
        profile_id: input.profile.profile_id.clone(),
        threshold: STANDALONE_THRESHOLD_ONE,
        trusted_keys: vec![artifact_auth_core::TrustedKeyObservation {
            producer_id: input.producer_id.to_string(),
            key_id: input.key_id.to_string(),
            key_identity,
            allowed_purposes: vec![request.expected_domain.purpose.as_str().to_string()],
            generation: request.signer_generation,
            currentness: map_currentness(request.signer_currentness),
            currentness_ref,
        }],
    };
    Ok((policy, scope, statement))
}

struct RefInput<'a> {
    profile: &'a str,
    value: &'a str,
    field: &'static str,
}

fn parse_ref(input: RefInput<'_>) -> Result<artifact_auth_core::ArtifactRef, Vec<String>> {
    let Some(digest_hex) = input.value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return Err(vec![format!("{}:expected-blake3-ref", input.field)]);
    };
    if !crate::fabric::valid_blake3_ref(input.value) {
        return Err(vec![format!("{}:malformed-blake3-ref", input.field)]);
    }
    Ok(artifact_auth_core::ArtifactRef {
        profile: input.profile.to_string(),
        algorithm: artifact_auth_core::ALGORITHM_BLAKE3.to_string(),
        digest_hex: digest_hex.to_string(),
    })
}

const fn map_currentness(currentness: super::KeyCurrentness) -> artifact_auth_core::KeyCurrentness {
    match currentness {
        super::KeyCurrentness::Current => artifact_auth_core::KeyCurrentness::Current,
        super::KeyCurrentness::Overlap => artifact_auth_core::KeyCurrentness::VerificationOverlap,
        super::KeyCurrentness::Superseded => artifact_auth_core::KeyCurrentness::Superseded,
        super::KeyCurrentness::Revoked => artifact_auth_core::KeyCurrentness::Revoked,
    }
}

fn compare_decisions(
    observation: &MoltenArtifactAuthObservation<'_>,
    legacy: &super::VerificationDecision,
    standalone: Option<&artifact_auth_core::AuthenticationDecision>,
    mut blockers: Vec<String>,
) -> MoltenArtifactAuthCompatibility {
    let causes = legacy.issues.iter().map(issue_class).collect::<std::collections::BTreeSet<_>>();
    let standalone_causes = standalone.map_or_else(std::collections::BTreeSet::new, standalone_failure_causes);
    let is_legacy_accepted = legacy.kind == super::VerificationDecisionKind::Accept;
    let is_standalone_accepted = standalone.is_some_and(|decision| decision.passed);
    let has_decision_drift = standalone.is_some() && is_legacy_accepted != is_standalone_accepted;
    if standalone.is_none() {
        blockers.push("standalone-evaluation-unavailable".to_string());
    }
    if has_decision_drift {
        blockers.push("decision-drift".to_string());
    }
    if !is_legacy_accepted && !is_standalone_accepted && causes.is_empty() {
        blockers.push("unclassified-rejection".to_string());
    }
    if !is_legacy_accepted && !is_standalone_accepted && causes.is_disjoint(&standalone_causes) {
        blockers.push("unrelated-rejection-causes".to_string());
    }
    let is_identity_drift_explained = observation
        .request
        .observed
        .signer_public_ref
        .strip_prefix(BLAKE3_REF_PREFIX)
        .is_some_and(|digest| {
            crate::fabric::valid_blake3_ref(&observation.request.observed.signer_public_ref)
                && observation.standalone_cryptographic.key_identity.digest_hex == digest
        });
    if !is_identity_drift_explained {
        blockers.push("identity-drift".to_string());
    }
    let has_non_claim_drift =
        standalone.is_none_or(|decision| decision.non_claims != artifact_auth_core::required_non_claims());
    if has_non_claim_drift {
        blockers.push("non-claim-drift".to_string());
    }
    blockers.sort();
    blockers.dedup();
    let issue_class = if is_legacy_accepted && is_standalone_accepted {
        ISSUE_CLASS_PARITY
    } else {
        ISSUE_CLASS_MAPPED_REJECTION
    };
    MoltenArtifactAuthCompatibility {
        case_explained: blockers.is_empty(),
        preimage_class: PREIMAGE_CLASS.to_string(),
        identity_drift_explained: is_identity_drift_explained,
        decision_drift: has_decision_drift,
        issue_class: issue_class.to_string(),
        mapped_failure_causes: causes.into_iter().map(str::to_string).collect(),
        standalone_failure_causes: standalone_causes.into_iter().map(str::to_string).collect(),
        non_claim_drift: has_non_claim_drift,
        blockers,
        legacy_authoritative: true,
        standalone_authority_admitted: false,
        rollback_available: true,
    }
}

fn standalone_failure_causes(
    decision: &artifact_auth_core::AuthenticationDecision,
) -> std::collections::BTreeSet<&'static str> {
    decision.issues.iter().map(|issue| standalone_issue_class(&issue.code)).collect()
}

fn standalone_issue_class(issue_code: &str) -> &'static str {
    if issue_code.contains("crypto") || issue_code.contains("signature") || issue_code.contains("ed25519") {
        return "signature";
    }
    if issue_code.contains("current") || issue_code.contains("revoked") || issue_code.contains("superseded") {
        return "currentness";
    }
    if issue_code.contains("generation") {
        return "generation";
    }
    if issue_code.contains("identity") || issue_code.contains("key") {
        return "signer-identity";
    }
    "standalone-policy"
}

const fn issue_class(issue: &super::CryptoIdentityIssue) -> &'static str {
    match issue {
        super::CryptoIdentityIssue::PurposeMismatch | super::CryptoIdentityIssue::UnsupportedPurpose(_) => "purpose",
        super::CryptoIdentityIssue::PayloadRefMismatch | super::CryptoIdentityIssue::PayloadSchemaMismatch => "payload",
        super::CryptoIdentityIssue::SignerPublicRefMismatch => "signer-identity",
        super::CryptoIdentityIssue::VerifierContextMismatch => "verifier-context",
        super::CryptoIdentityIssue::HandleGenerationStale { .. } => "generation",
        super::CryptoIdentityIssue::HandleNotCurrent(_) => "currentness",
        super::CryptoIdentityIssue::CryptographicVerificationFailed
        | super::CryptoIdentityIssue::SignatureMalformed => "signature",
        super::CryptoIdentityIssue::SignatureTooLarge { .. } => "signature-size",
        super::CryptoIdentityIssue::ProfileMismatch => "profile",
        super::CryptoIdentityIssue::DomainVersionMismatch => "domain",
        _ => "consumer-policy",
    }
}

pub fn standalone_observation(
    key_ref: &str,
    verified: bool,
) -> Result<artifact_auth_core::CryptographicObservation, String> {
    let key_identity = parse_ref(RefInput {
        profile: artifact_auth_core::ED25519_PUBLIC_KEY_PROFILE_V1,
        value: key_ref,
        field: "key_ref",
    })
    .map_err(|issues| issues.join(","))?;
    Ok(artifact_auth_core::CryptographicObservation {
        algorithm: artifact_auth_core::ALGORITHM_ED25519.to_string(),
        key_identity,
        verified,
        failure_code: (!verified).then(|| STANDALONE_FAILURE_CODE.to_string()),
    })
}
