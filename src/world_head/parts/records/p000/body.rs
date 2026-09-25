use artifact_auth_core::ALGORITHM_BLAKE3;
use artifact_auth_core::ArtifactRef;
use artifact_auth_core::ArtifactStatement;
use artifact_auth_core::AuthenticationScope;
use artifact_auth_core::STATEMENT_SCHEMA_V1;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_DOMAIN;
use molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PROFILE;
use molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PURPOSE;
use molten_core::world_head::WORLD_HEAD_CLAIM_SCHEMA;
use molten_core::world_head::WORLD_HEAD_CONFLICT_SCHEMA;
use molten_core::world_head::WORLD_HEAD_TRANSITION_SCHEMA;
use molten_core::world_head::WorldBranchClass;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadClaim;
use molten_core::world_head::WorldHeadClaimRef;
use molten_core::world_head::WorldHeadConflictSet;
use molten_core::world_head::WorldHeadCurrentnessClass;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_head::WorldHeadPurpose;
use molten_core::world_head::WorldHeadState;
use molten_core::world_head::WorldHeadStatementRef;
use molten_core::world_head::WorldHeadTransitionPlan;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const WORLD_HEAD_CLAIM_RECORD: &str = "world-head-claim";
pub const WORLD_HEAD_STATE_RECORD: &str = "world-head-state";
pub const WORLD_HEAD_CONFLICT_RECORD: &str = "world-head-conflict";
pub const WORLD_HEAD_TRANSITION_RECEIPT_RECORD: &str = "world-head-transition-receipt";
const WORLD_HEAD_CLAIM_FIELD_COUNT: u64 = 10;
const WORLD_HEAD_STATE_FIELD_COUNT: u64 = 6;
const WORLD_COMMIT_ARTIFACT_PROFILE: &str = "molten-world-commit-v1";
const WORLD_HEAD_CLAIM_ARTIFACT_PROFILE: &str = "molten-world-head-claim-v1";
const WORLD_HEAD_POLICY_ARTIFACT_PROFILE: &str = "molten-world-head-policy-v1";
const TRANSITION_DECISION_ADMITTED: &str = "admitted";
const TRANSITION_DECISION_DENIED: &str = "denied";
const TRANSITION_DECISION_CONFLICT: &str = "conflict";
const NON_CLAIM_COUNT: usize = 8;
const WORLD_HEAD_BOUNDARY_SCHEMA_COUNT: usize = 3;
const SCHEMA_FIELD: crate::preserves_rail::BoundaryFieldSpec = crate::preserves_rail::BoundaryFieldSpec {
    label: "schema-id",
    kind: crate::preserves_rail::BoundaryFieldKind::SchemaId,
};

macro_rules! boundary_field {
    ($label:literal, $kind:ident) => {
        crate::preserves_rail::BoundaryFieldSpec {
            label: $label,
            kind: crate::preserves_rail::BoundaryFieldKind::$kind,
        }
    };
}

const WORLD_HEAD_CLAIM_FIELDS: &[crate::preserves_rail::BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    boundary_field!("branch-id", StringRecord),
    boundary_field!("branch-class", StringRecord),
    boundary_field!("expected-head", OptionalRefRecord),
    boundary_field!("successor-head", RefRecord),
    boundary_field!("expected-generation", U64Record),
    boundary_field!("successor-generation", U64Record),
    boundary_field!("purpose", StringRecord),
    boundary_field!("policy-ref", RefRecord),
    boundary_field!("source-heads", RefSequenceRecord),
];
const WORLD_HEAD_CONFLICT_FIELDS: &[crate::preserves_rail::BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    boundary_field!("branch-id", StringRecord),
    boundary_field!("expected-head", RefRecord),
    boundary_field!("expected-generation", U64Record),
    boundary_field!("members", AnySequenceRecord),
    boundary_field!("conflict-ref", RefRecord),
    boundary_field!("non-claims", UniqueStringSequenceRecord),
];
const WORLD_HEAD_TRANSITION_RECEIPT_FIELDS: &[crate::preserves_rail::BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    boundary_field!("decision", StringRecord),
    boundary_field!("claim-ref", RefRecord),
    boundary_field!("statement-ref", RefRecord),
    boundary_field!("authentication-decision-ref", RefRecord),
    boundary_field!("authority-ref", RefRecord),
    boundary_field!("before-head", OptionalRefRecord),
    boundary_field!("before-generation", AnyRecord),
    boundary_field!("after-head", OptionalRefRecord),
    boundary_field!("after-generation", AnyRecord),
    boundary_field!("currentness", StringRecord),
    boundary_field!("issues", StringSequenceRecord),
    boundary_field!("non-claims", UniqueStringSequenceRecord),
];

pub const WORLD_HEAD_CLAIM_BOUNDARY_SCHEMA: crate::preserves_rail::BoundarySchemaSpec =
    crate::preserves_rail::BoundarySchemaSpec {
        family: "molten-world-head-claim",
        version: "v1",
        record_label: WORLD_HEAD_CLAIM_RECORD,
        schema_id: WORLD_HEAD_CLAIM_SCHEMA,
        fields: WORLD_HEAD_CLAIM_FIELDS,
    };
pub const WORLD_HEAD_CONFLICT_BOUNDARY_SCHEMA: crate::preserves_rail::BoundarySchemaSpec =
    crate::preserves_rail::BoundarySchemaSpec {
        family: "molten-world-head-conflict",
        version: "v1",
        record_label: WORLD_HEAD_CONFLICT_RECORD,
        schema_id: WORLD_HEAD_CONFLICT_SCHEMA,
        fields: WORLD_HEAD_CONFLICT_FIELDS,
    };
pub const WORLD_HEAD_TRANSITION_RECEIPT_BOUNDARY_SCHEMA: crate::preserves_rail::BoundarySchemaSpec =
    crate::preserves_rail::BoundarySchemaSpec {
        family: "molten-world-head-transition-receipt",
        version: "v1",
        record_label: WORLD_HEAD_TRANSITION_RECEIPT_RECORD,
        schema_id: WORLD_HEAD_TRANSITION_SCHEMA,
        fields: WORLD_HEAD_TRANSITION_RECEIPT_FIELDS,
    };
pub const WORLD_HEAD_BOUNDARY_SCHEMAS: [crate::preserves_rail::BoundarySchemaSpec; WORLD_HEAD_BOUNDARY_SCHEMA_COUNT] = [
    WORLD_HEAD_CLAIM_BOUNDARY_SCHEMA,
    WORLD_HEAD_CONFLICT_BOUNDARY_SCHEMA,
    WORLD_HEAD_TRANSITION_RECEIPT_BOUNDARY_SCHEMA,
];

pub const WORLD_HEAD_NON_CLAIMS: [&str; NON_CLAIM_COUNT] = [
    "authentication-does-not-grant-branch-authority",
    "generation-fencing-does-not-prove-whole-store-rollback-detection",
    "local-atomicity-does-not-prove-remote-publication",
    "local-head-state-does-not-prove-distributed-consensus",
    "conflict-retention-does-not-select-application-meaning",
    "world-head-receipts-do-not-prove-commit-correctness",
    "world-head-receipts-do-not-prove-effect-release",
    "world-head-receipts-do-not-prove-release-eligibility",
];

#[derive(Debug, Clone)]
pub struct CanonicalWorldHeadClaim {
    pub claim: WorldHeadClaim,
    pub claim_ref: WorldHeadClaimRef,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WorldHeadArtifactAuthInput<'a> {
    pub producer_id: &'a str,
    pub key_id: &'a str,
    pub key_identity: ArtifactRef,
}

#[derive(Debug, Clone)]
pub struct CanonicalWorldHeadConflict {
    pub conflict_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WorldHeadTransitionReceiptInput<'a> {
    pub decision: &'a str,
    pub plan: Option<&'a WorldHeadTransitionPlan>,
    pub claim_ref: &'a WorldHeadClaimRef,
    pub statement_ref: &'a WorldHeadStatementRef,
    pub authentication_decision_ref: &'a str,
    pub authority_ref: &'a str,
    pub issue_codes: &'a [String],
}

#[derive(Debug, Clone)]
pub struct CanonicalWorldHeadTransitionReceipt {
    pub receipt_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_world_head_claim(claim: &WorldHeadClaim) -> Result<CanonicalWorldHeadClaim> {
    let value = world_head_claim_value(claim);
    crate::preserves_rail::validate_boundary_schema(&value, &WORLD_HEAD_CLAIM_BOUNDARY_SCHEMA)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let claim_ref = WorldHeadClaimRef::new(crate::preserves_rail::content_ref_from_bytes(&bytes))
        .map_err(|error| MoltenError::invalid_harness(format!("world-head claim identity failed: {error}")))?;
    Ok(CanonicalWorldHeadClaim {
        claim: claim.clone(),
        claim_ref,
        value,
        bytes,
    })
}

pub fn parse_canonical_world_head_claim(bytes: &[u8]) -> Result<CanonicalWorldHeadClaim> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(
        &decoded.value,
        WORLD_HEAD_CLAIM_RECORD,
        WORLD_HEAD_CLAIM_FIELD_COUNT,
    )?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "world-head claim schema")?;
    if schema != WORLD_HEAD_CLAIM_SCHEMA {
        return Err(MoltenError::invalid_harness("unsupported world-head claim schema"));
    }
    let branch_id = WorldBranchId::new(crate::preserves_rail::required_string_field(
        &named_field_value(&fields[1], "branch-id")?,
        "world-head branch id",
    )?)
    .map_err(reference_error)?;
    let branch_class = WorldBranchClass::parse(&crate::preserves_rail::required_string_field(
        &named_field_value(&fields[2], "branch-class")?,
        "world-head branch class",
    )?)
    .map_err(reference_error)?;
    let expected_head = crate::preserves_rail::optional_content_ref_string(
        &named_field_value(&fields[3], "expected-head")?,
        "expected world head",
    )?
    .map(WorldCommitRef::new)
    .transpose()
    .map_err(world_commit_reference_error)?;
    let successor_head = WorldCommitRef::new(crate::preserves_rail::required_content_ref_string(
        &named_field_value(&fields[4], "successor-head")?,
        "successor world head",
    )?)
    .map_err(world_commit_reference_error)?;
    let expected_generation =
        required_u64(&named_field_value(&fields[5], "expected-generation")?, "expected world-head generation")?;
    let successor_generation =
        required_u64(&named_field_value(&fields[6], "successor-generation")?, "successor world-head generation")?;
    let purpose = WorldHeadPurpose::parse(&crate::preserves_rail::required_string_field(
        &named_field_value(&fields[7], "purpose")?,
        "world-head purpose",
    )?)
    .map_err(reference_error)?;
    let policy_ref = WorldHeadPolicyRef::new(crate::preserves_rail::required_content_ref_string(
        &named_field_value(&fields[8], "policy-ref")?,
        "world-head policy ref",
    )?)
    .map_err(reference_error)?;
    let source_heads = parse_source_heads(&named_field_value(&fields[9], "source-heads")?)?;
    let claim = WorldHeadClaim {
        branch_id,
        branch_class,
        expected_head,
        successor_head,
        expected_generation,
        successor_generation,
        purpose,
        policy_ref,
        source_heads,
    };
    let canonical = canonical_world_head_claim(&claim)?;
    if canonical.bytes != decoded.canonical_bytes {
        return Err(MoltenError::invalid_harness("world-head claim bytes are not canonical"));
    }
    Ok(canonical)
}

fn parse_source_heads(source_heads_field: &preserves::Value<IOValue>) -> Result<Vec<WorldCommitRef>> {
    let source_values = crate::preserves_rail::required_sequence_field(source_heads_field, "world-head source heads")?;
    source_values
        .iter()
        .map(|value| {
            crate::preserves_rail::required_content_ref_string(value, "world-head source head")
                .and_then(|reference| WorldCommitRef::new(reference).map_err(world_commit_reference_error))
        })
        .collect::<Result<Vec<_>>>()
}

pub fn world_head_authentication_scope(claim: &CanonicalWorldHeadClaim) -> Result<AuthenticationScope> {
    let mut parents = Vec::new();
    if let Some(expected) = &claim.claim.expected_head {
        parents.push(artifact_ref(WORLD_COMMIT_ARTIFACT_PROFILE, expected.as_str())?);
    }
    parents.push(artifact_ref(WORLD_COMMIT_ARTIFACT_PROFILE, claim.claim.successor_head.as_str())?);
    parents.sort();
    parents.dedup();
    Ok(AuthenticationScope {
        domain: WORLD_HEAD_ARTIFACT_AUTH_DOMAIN.to_string(),
        purpose: WORLD_HEAD_ARTIFACT_AUTH_PURPOSE.to_string(),
        profile_id: WORLD_HEAD_ARTIFACT_AUTH_PROFILE.to_string(),
        subject: artifact_ref(WORLD_HEAD_CLAIM_ARTIFACT_PROFILE, claim.claim_ref.as_str())?,
        parents,
        verifier_context: artifact_ref(WORLD_HEAD_POLICY_ARTIFACT_PROFILE, claim.claim.policy_ref.as_str())?,
    })
}
