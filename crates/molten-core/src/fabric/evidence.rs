use super::FabricNonClaim;
use super::MAX_FABRIC_COLLECTION_ITEMS;
use super::has_duplicates;
use super::valid_blake3_ref;
use super::valid_fabric_token;
use super::validate_required_non_claims;

pub const FABRIC_EVIDENCE_PROFILE_SCHEMA: &str = "molten.fabric.evidence-profile.v1";

const REQUIRED_SEMANTIC_BOUNDARY_COUNT: usize = 6;
const INTERNAL_BOUNDARY_COUNT: usize = 4;
const ALL_EVIDENCE_BOUNDARY_COUNT: usize = REQUIRED_SEMANTIC_BOUNDARY_COUNT + INTERNAL_BOUNDARY_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabricProfileClass {
    Production,
    Diagnostic,
}

impl FabricProfileClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Diagnostic => "diagnostic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceBoundary {
    Trust,
    Lifecycle,
    Commit,
    Checkpoint,
    Failure,
    OperatorObservation,
    InternalPageRead,
    InternalPacket,
    InternalSchedulerPoll,
    InternalCacheLookup,
}

impl EvidenceBoundary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trust => "trust",
            Self::Lifecycle => "lifecycle",
            Self::Commit => "commit",
            Self::Checkpoint => "checkpoint",
            Self::Failure => "failure",
            Self::OperatorObservation => "operator-observation",
            Self::InternalPageRead => "internal-page-read",
            Self::InternalPacket => "internal-packet",
            Self::InternalSchedulerPoll => "internal-scheduler-poll",
            Self::InternalCacheLookup => "internal-cache-lookup",
        }
    }

    pub const fn is_internal(self) -> bool {
        matches!(
            self,
            Self::InternalPageRead | Self::InternalPacket | Self::InternalSchedulerPoll | Self::InternalCacheLookup
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceEmission {
    CanonicalBoundaryReceipt,
    BoundedAggregate,
    Omitted,
}

impl EvidenceEmission {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalBoundaryReceipt => "canonical-boundary-receipt",
            Self::BoundedAggregate => "bounded-aggregate",
            Self::Omitted => "omitted",
        }
    }
}

pub const REQUIRED_SEMANTIC_EVIDENCE_BOUNDARIES: [EvidenceBoundary; REQUIRED_SEMANTIC_BOUNDARY_COUNT] = [
    EvidenceBoundary::Trust,
    EvidenceBoundary::Lifecycle,
    EvidenceBoundary::Commit,
    EvidenceBoundary::Checkpoint,
    EvidenceBoundary::Failure,
    EvidenceBoundary::OperatorObservation,
];

pub const INTERNAL_EVIDENCE_BOUNDARIES: [EvidenceBoundary; INTERNAL_BOUNDARY_COUNT] = [
    EvidenceBoundary::InternalPageRead,
    EvidenceBoundary::InternalPacket,
    EvidenceBoundary::InternalSchedulerPoll,
    EvidenceBoundary::InternalCacheLookup,
];

pub const ALL_EVIDENCE_BOUNDARIES: [EvidenceBoundary; ALL_EVIDENCE_BOUNDARY_COUNT] = [
    EvidenceBoundary::Trust,
    EvidenceBoundary::Lifecycle,
    EvidenceBoundary::Commit,
    EvidenceBoundary::Checkpoint,
    EvidenceBoundary::Failure,
    EvidenceBoundary::OperatorObservation,
    EvidenceBoundary::InternalPageRead,
    EvidenceBoundary::InternalPacket,
    EvidenceBoundary::InternalSchedulerPoll,
    EvidenceBoundary::InternalCacheLookup,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceRule {
    pub boundary: EvidenceBoundary,
    pub emission: EvidenceEmission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricEvidenceProfile {
    pub schema: String,
    pub profile_id: String,
    pub class: FabricProfileClass,
    pub rules: Vec<EvidenceRule>,
    pub aggregate_limit_ref: Option<String>,
    pub non_claims: Vec<FabricNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricEvidenceProfileSummary {
    pub profile_id: String,
    pub class: FabricProfileClass,
    pub rules: Vec<EvidenceRule>,
    pub aggregate_limit_ref: Option<String>,
    pub non_claims: Vec<FabricNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FabricEvidenceIssue {
    SchemaMismatch { actual: String, expected: String },
    MalformedProfileId(String),
    TooManyRules { actual: usize, maximum: usize },
    DuplicateBoundary(EvidenceBoundary),
    MissingBoundary(EvidenceBoundary),
    SemanticBoundaryNotCanonical(EvidenceBoundary),
    ProductionPerOperationInternalReceipt(EvidenceBoundary),
    MissingAggregateLimitRef,
    MalformedAggregateLimitRef(String),
    TooManyNonClaims { actual: usize, maximum: usize },
    DuplicateNonClaim,
    MissingNonClaim(FabricNonClaim),
}

pub fn default_production_evidence_profile(aggregate_limit_ref: impl Into<String>) -> FabricEvidenceProfile {
    let mut rules = REQUIRED_SEMANTIC_EVIDENCE_BOUNDARIES
        .iter()
        .map(|boundary| EvidenceRule {
            boundary: *boundary,
            emission: EvidenceEmission::CanonicalBoundaryReceipt,
        })
        .collect::<Vec<_>>();
    rules.extend(INTERNAL_EVIDENCE_BOUNDARIES.iter().map(|boundary| EvidenceRule {
        boundary: *boundary,
        emission: EvidenceEmission::BoundedAggregate,
    }));
    FabricEvidenceProfile {
        schema: FABRIC_EVIDENCE_PROFILE_SCHEMA.to_string(),
        profile_id: "molten.fabric.production-evidence.v1".to_string(),
        class: FabricProfileClass::Production,
        rules,
        aggregate_limit_ref: Some(aggregate_limit_ref.into()),
        non_claims: super::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
    }
}

// r[impl molten.fabric_boundary.evidence_granularity]
// r[impl molten.fabric_boundary.non_claims]
pub fn validate_fabric_evidence_profile(
    profile: &FabricEvidenceProfile,
) -> Result<FabricEvidenceProfileSummary, Vec<FabricEvidenceIssue>> {
    let mut issues = Vec::new();
    validate_evidence_identity(profile, &mut issues);
    validate_evidence_rules(profile, &mut issues);
    validate_aggregate_limit(profile, &mut issues);
    validate_evidence_non_claims(profile, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut rules = profile.rules.clone();
    rules.sort();
    let mut non_claims = profile.non_claims.clone();
    non_claims.sort();
    Ok(FabricEvidenceProfileSummary {
        profile_id: profile.profile_id.clone(),
        class: profile.class,
        rules,
        aggregate_limit_ref: profile.aggregate_limit_ref.clone(),
        non_claims,
    })
}

fn validate_evidence_identity(profile: &FabricEvidenceProfile, issues: &mut Vec<FabricEvidenceIssue>) {
    if profile.schema != FABRIC_EVIDENCE_PROFILE_SCHEMA {
        issues.push(FabricEvidenceIssue::SchemaMismatch {
            actual: profile.schema.clone(),
            expected: FABRIC_EVIDENCE_PROFILE_SCHEMA.to_string(),
        });
    }
    if !valid_fabric_token(&profile.profile_id) {
        issues.push(FabricEvidenceIssue::MalformedProfileId(profile.profile_id.clone()));
    }
}

fn validate_evidence_rules(profile: &FabricEvidenceProfile, issues: &mut Vec<FabricEvidenceIssue>) {
    if profile.rules.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(FabricEvidenceIssue::TooManyRules {
            actual: profile.rules.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    let boundaries = profile.rules.iter().map(|rule| rule.boundary).collect::<Vec<_>>();
    if has_duplicates(&boundaries) {
        for boundary in ALL_EVIDENCE_BOUNDARIES {
            if boundaries.iter().filter(|candidate| **candidate == boundary).count() > 1 {
                issues.push(FabricEvidenceIssue::DuplicateBoundary(boundary));
            }
        }
    }
    for boundary in ALL_EVIDENCE_BOUNDARIES {
        if !boundaries.contains(&boundary) {
            issues.push(FabricEvidenceIssue::MissingBoundary(boundary));
        }
    }
    for required in REQUIRED_SEMANTIC_EVIDENCE_BOUNDARIES {
        let emission = profile.rules.iter().find(|rule| rule.boundary == required).map(|rule| rule.emission);
        if emission.is_some() && emission != Some(EvidenceEmission::CanonicalBoundaryReceipt) {
            issues.push(FabricEvidenceIssue::SemanticBoundaryNotCanonical(required));
        }
    }
    if profile.class == FabricProfileClass::Production {
        for rule in &profile.rules {
            if rule.boundary.is_internal() && rule.emission == EvidenceEmission::CanonicalBoundaryReceipt {
                issues.push(FabricEvidenceIssue::ProductionPerOperationInternalReceipt(rule.boundary));
            }
        }
    }
}

fn validate_aggregate_limit(profile: &FabricEvidenceProfile, issues: &mut Vec<FabricEvidenceIssue>) {
    let uses_aggregate = profile.rules.iter().any(|rule| rule.emission == EvidenceEmission::BoundedAggregate);
    if !uses_aggregate {
        return;
    }
    let Some(reference) = profile.aggregate_limit_ref.as_deref() else {
        issues.push(FabricEvidenceIssue::MissingAggregateLimitRef);
        return;
    };
    if !valid_blake3_ref(reference) {
        issues.push(FabricEvidenceIssue::MalformedAggregateLimitRef(reference.to_string()));
    }
}

fn validate_evidence_non_claims(profile: &FabricEvidenceProfile, issues: &mut Vec<FabricEvidenceIssue>) {
    if profile.non_claims.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(FabricEvidenceIssue::TooManyNonClaims {
            actual: profile.non_claims.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(&profile.non_claims) {
        issues.push(FabricEvidenceIssue::DuplicateNonClaim);
    }
    validate_required_non_claims(&profile.non_claims, |missing| {
        issues.push(FabricEvidenceIssue::MissingNonClaim(missing));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    // r[verify molten.fabric_boundary.evidence_granularity]
    #[test]
    fn production_profile_records_semantic_boundaries_and_aggregates_internal_operations() {
        let summary = validate_fabric_evidence_profile(&default_production_evidence_profile(LIMIT_REF))
            .expect("bounded production evidence profile");

        assert_eq!(summary.class, FabricProfileClass::Production);
        assert_eq!(summary.aggregate_limit_ref.as_deref(), Some(LIMIT_REF));
        for boundary in REQUIRED_SEMANTIC_EVIDENCE_BOUNDARIES {
            assert!(summary.rules.contains(&EvidenceRule {
                boundary,
                emission: EvidenceEmission::CanonicalBoundaryReceipt,
            }));
        }
        for boundary in INTERNAL_EVIDENCE_BOUNDARIES {
            assert!(summary.rules.contains(&EvidenceRule {
                boundary,
                emission: EvidenceEmission::BoundedAggregate,
            }));
        }
    }

    // r[verify molten.fabric_boundary.evidence_granularity]
    #[test]
    fn production_profile_rejects_omitted_commit_and_per_packet_receipts() {
        let mut profile = default_production_evidence_profile(LIMIT_REF);
        for rule in &mut profile.rules {
            if rule.boundary == EvidenceBoundary::Commit {
                rule.emission = EvidenceEmission::Omitted;
            }
            if rule.boundary == EvidenceBoundary::InternalPacket {
                rule.emission = EvidenceEmission::CanonicalBoundaryReceipt;
            }
        }

        let issues = validate_fabric_evidence_profile(&profile).expect_err("unsafe evidence granularity must deny");

        assert!(issues.contains(&FabricEvidenceIssue::SemanticBoundaryNotCanonical(EvidenceBoundary::Commit)));
        assert!(
            issues.contains(&FabricEvidenceIssue::ProductionPerOperationInternalReceipt(
                EvidenceBoundary::InternalPacket
            ))
        );
    }

    // r[verify molten.fabric_boundary.evidence_granularity]
    #[test]
    fn aggregate_evidence_requires_a_canonical_bounded_profile_ref() {
        let mut profile = default_production_evidence_profile("sha256:not-canonical");

        let malformed = validate_fabric_evidence_profile(&profile).expect_err("malformed limit ref must deny");
        assert!(
            malformed.contains(&FabricEvidenceIssue::MalformedAggregateLimitRef("sha256:not-canonical".to_string()))
        );

        profile.aggregate_limit_ref = None;
        let missing = validate_fabric_evidence_profile(&profile).expect_err("missing limit ref must deny");
        assert!(missing.contains(&FabricEvidenceIssue::MissingAggregateLimitRef));
    }
}
