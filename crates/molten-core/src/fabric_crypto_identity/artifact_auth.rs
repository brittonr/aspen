use std::collections::BTreeSet;

use artifact_auth_core::ALGORITHM_BLAKE3;
use artifact_auth_core::ALGORITHM_ED25519;
use artifact_auth_core::ArtifactRef;
use artifact_auth_core::ArtifactStatement;
use artifact_auth_core::AuthenticationDecision;
use artifact_auth_core::AuthenticationPolicy;
use artifact_auth_core::AuthenticationScope;
use artifact_auth_core::CryptographicObservation;
use artifact_auth_core::ED25519_PUBLIC_KEY_PROFILE_V1;
use artifact_auth_core::KeyCurrentness as StandaloneCurrentness;
use artifact_auth_core::POLICY_SCHEMA_V1;
use artifact_auth_core::STATEMENT_SCHEMA_V1;
use artifact_auth_core::SignatureEvidence;
use artifact_auth_core::TrustedKeyObservation;
use artifact_auth_core::evaluate_authentication;
use artifact_auth_core::required_non_claims;

use super::CryptoAdapterProfile;
use super::CryptoAlgorithm;
use super::CryptoIdentityIssue;
use super::KeyCurrentness;
use super::VerificationDecision;
use super::VerificationDecisionKind;
use super::VerificationRequest;
use super::evaluate_verification;

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
    pub profile: &'a CryptoAdapterProfile,
    pub request: &'a VerificationRequest,
    pub producer_id: &'a str,
    pub key_id: &'a str,
    pub currentness_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoltenArtifactAuthObservation<'a> {
    pub profile: &'a CryptoAdapterProfile,
    pub request: &'a VerificationRequest,
    pub producer_id: &'a str,
    pub key_id: &'a str,
    pub currentness_ref: &'a str,
    pub standalone_cryptographic: CryptographicObservation,
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
    pub legacy: VerificationDecision,
    pub standalone: Option<AuthenticationDecision>,
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
    let legacy = evaluate_verification(observation.profile, observation.request);
    let mapped = map_observation(observation);
    let standalone = mapped
        .as_ref()
        .ok()
        .map(|(policy, scope, evidence)| evaluate_authentication(policy, scope, evidence));
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
) -> Result<ArtifactStatement, Vec<String>> {
    let (_, _, statement) = map_statement_and_policy(input)?;
    Ok(statement)
}

fn map_observation(
    observation: &MoltenArtifactAuthObservation<'_>,
) -> Result<(AuthenticationPolicy, AuthenticationScope, Vec<SignatureEvidence>), Vec<String>> {
    let input = MoltenArtifactAuthStatementInput {
        profile: observation.profile,
        request: observation.request,
        producer_id: observation.producer_id,
        key_id: observation.key_id,
        currentness_ref: observation.currentness_ref,
    };
    let (policy, scope, statement) = map_statement_and_policy(&input)?;
    let evidence = vec![SignatureEvidence {
        statement,
        generation: observation.request.observed.generation,
        cryptographic: observation.standalone_cryptographic.clone(),
    }];
    Ok((policy, scope, evidence))
}

fn map_statement_and_policy(
    input: &MoltenArtifactAuthStatementInput<'_>,
) -> Result<(AuthenticationPolicy, AuthenticationScope, ArtifactStatement), Vec<String>> {
    let request = input.request;
    let key_identity =
        artifact_ref(ED25519_PUBLIC_KEY_PROFILE_V1, &request.observed.signer_public_ref, "observed.signer_public_ref")?;
    let subject = artifact_ref(
        &request.expected_domain.payload_schema,
        &request.expected_domain.payload_ref,
        "expected_domain.payload_ref",
    )?;
    let verifier_context = artifact_ref(
        MOLTEN_VERIFIER_CONTEXT_PROFILE,
        &request.expected_domain.verifier_context_ref,
        "expected_domain.verifier_context_ref",
    )?;
    let currentness_ref = artifact_ref(MOLTEN_CURRENTNESS_PROFILE, input.currentness_ref, "currentness_ref")?;
    if input.profile.algorithm != CryptoAlgorithm::Ed25519Iroh {
        return Err(vec!["unsupported-production-algorithm".to_string()]);
    }
    let scope = AuthenticationScope {
        domain: request.expected_domain.domain_id.clone(),
        purpose: request.expected_domain.purpose.as_str().to_string(),
        profile_id: input.profile.profile_id.clone(),
        subject,
        parents: Vec::new(),
        verifier_context,
    };
    let statement = ArtifactStatement {
        schema: STATEMENT_SCHEMA_V1.to_string(),
        scope: scope.clone(),
        producer_id: input.producer_id.to_string(),
        key_id: input.key_id.to_string(),
        key_identity: key_identity.clone(),
    };
    let policy = AuthenticationPolicy {
        schema: POLICY_SCHEMA_V1.to_string(),
        profile_id: input.profile.profile_id.clone(),
        threshold: STANDALONE_THRESHOLD_ONE,
        trusted_keys: vec![TrustedKeyObservation {
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

fn artifact_ref(profile: &str, value: &str, field: &str) -> Result<ArtifactRef, Vec<String>> {
    let Some(digest_hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return Err(vec![format!("{field}:expected-blake3-ref")]);
    };
    Ok(ArtifactRef {
        profile: profile.to_string(),
        algorithm: ALGORITHM_BLAKE3.to_string(),
        digest_hex: digest_hex.to_string(),
    })
}

const fn map_currentness(currentness: KeyCurrentness) -> StandaloneCurrentness {
    match currentness {
        KeyCurrentness::Current => StandaloneCurrentness::Current,
        KeyCurrentness::Overlap => StandaloneCurrentness::VerificationOverlap,
        KeyCurrentness::Superseded => StandaloneCurrentness::Superseded,
        KeyCurrentness::Revoked => StandaloneCurrentness::Revoked,
    }
}

fn compare_decisions(
    observation: &MoltenArtifactAuthObservation<'_>,
    legacy: &VerificationDecision,
    standalone: Option<&AuthenticationDecision>,
    mut blockers: Vec<String>,
) -> MoltenArtifactAuthCompatibility {
    let causes = legacy_failure_causes(observation.profile, observation.request);
    let standalone_causes = standalone.map_or_else(BTreeSet::new, standalone_failure_causes);
    let legacy_passed = legacy.kind == VerificationDecisionKind::Accept;
    let standalone_passed = standalone.is_some_and(|decision| decision.passed);
    let decision_drift = standalone.is_some() && legacy_passed != standalone_passed;
    if standalone.is_none() {
        blockers.push("standalone-evaluation-unavailable".to_string());
    }
    if decision_drift {
        blockers.push("decision-drift".to_string());
    }
    if !legacy_passed && !standalone_passed && causes.is_empty() {
        blockers.push("unclassified-rejection".to_string());
    }
    if !legacy_passed && !standalone_passed && causes.is_disjoint(&standalone_causes) {
        blockers.push("unrelated-rejection-causes".to_string());
    }
    let identity_drift_explained = observation
        .request
        .observed
        .signer_public_ref
        .strip_prefix(BLAKE3_REF_PREFIX)
        .is_some_and(|digest| observation.standalone_cryptographic.key_identity.digest_hex == digest);
    if !identity_drift_explained {
        blockers.push("identity-drift".to_string());
    }
    let non_claim_drift = standalone.is_none_or(|decision| decision.non_claims != required_non_claims());
    if non_claim_drift {
        blockers.push("non-claim-drift".to_string());
    }
    blockers.sort();
    blockers.dedup();
    let issue_class = if legacy_passed && standalone_passed {
        ISSUE_CLASS_PARITY
    } else {
        ISSUE_CLASS_MAPPED_REJECTION
    };
    MoltenArtifactAuthCompatibility {
        case_explained: blockers.is_empty(),
        preimage_class: PREIMAGE_CLASS.to_string(),
        identity_drift_explained,
        decision_drift,
        issue_class: issue_class.to_string(),
        mapped_failure_causes: causes.into_iter().collect(),
        standalone_failure_causes: standalone_causes.into_iter().collect(),
        non_claim_drift,
        blockers,
        legacy_authoritative: true,
        standalone_authority_admitted: false,
        rollback_available: true,
    }
}

fn legacy_failure_causes(profile: &CryptoAdapterProfile, request: &VerificationRequest) -> BTreeSet<String> {
    let decision = evaluate_verification(profile, request);
    decision.issues.iter().map(issue_class).map(str::to_string).collect()
}

fn standalone_failure_causes(decision: &AuthenticationDecision) -> BTreeSet<String> {
    decision.issues.iter().map(|issue| standalone_issue_class(&issue.code)).collect()
}

fn standalone_issue_class(issue_code: &str) -> String {
    if issue_code.contains("crypto") || issue_code.contains("signature") || issue_code.contains("ed25519") {
        return "signature".to_string();
    }
    if issue_code.contains("current") || issue_code.contains("revoked") || issue_code.contains("superseded") {
        return "currentness".to_string();
    }
    if issue_code.contains("generation") {
        return "generation".to_string();
    }
    if issue_code.contains("identity") || issue_code.contains("key") {
        return "signer-identity".to_string();
    }
    "standalone-policy".to_string()
}

fn issue_class(issue: &CryptoIdentityIssue) -> &'static str {
    match issue {
        CryptoIdentityIssue::PurposeMismatch | CryptoIdentityIssue::UnsupportedPurpose(_) => "purpose",
        CryptoIdentityIssue::PayloadRefMismatch | CryptoIdentityIssue::PayloadSchemaMismatch => "payload",
        CryptoIdentityIssue::SignerPublicRefMismatch => "signer-identity",
        CryptoIdentityIssue::VerifierContextMismatch => "verifier-context",
        CryptoIdentityIssue::HandleGenerationStale { .. } => "generation",
        CryptoIdentityIssue::HandleNotCurrent(_) => "currentness",
        CryptoIdentityIssue::CryptographicVerificationFailed | CryptoIdentityIssue::SignatureMalformed => "signature",
        CryptoIdentityIssue::SignatureTooLarge { .. } => "signature-size",
        CryptoIdentityIssue::ProfileMismatch => "profile",
        CryptoIdentityIssue::DomainVersionMismatch => "domain",
        _ => "consumer-policy",
    }
}

pub fn standalone_observation(key_ref: &str, verified: bool) -> Result<CryptographicObservation, String> {
    let key_identity =
        artifact_ref(ED25519_PUBLIC_KEY_PROFILE_V1, key_ref, "key_ref").map_err(|issues| issues.join(","))?;
    Ok(CryptographicObservation {
        algorithm: ALGORITHM_ED25519.to_string(),
        key_identity,
        verified,
        failure_code: (!verified).then(|| STANDALONE_FAILURE_CODE.to_string()),
    })
}
