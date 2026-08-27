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
const WORLD_HEAD_CLAIM_FIELD_COUNT: usize = 10;
const WORLD_HEAD_STATE_FIELD_COUNT: usize = 6;
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
    let source_heads_field = named_field_value(&fields[9], "source-heads")?;
    let source_values = crate::preserves_rail::required_sequence_field(&source_heads_field, "world-head source heads")?;
    let source_heads = source_values
        .iter()
        .map(|value| {
            crate::preserves_rail::required_content_ref_string(value, "world-head source head")
                .and_then(|reference| WorldCommitRef::new(reference).map_err(world_commit_reference_error))
        })
        .collect::<Result<Vec<_>>>()?;
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

pub fn world_head_artifact_statement(
    claim: &CanonicalWorldHeadClaim,
    input: WorldHeadArtifactAuthInput<'_>,
) -> Result<(ArtifactStatement, WorldHeadStatementRef)> {
    let statement = ArtifactStatement {
        schema: STATEMENT_SCHEMA_V1.to_string(),
        scope: world_head_authentication_scope(claim)?,
        producer_id: input.producer_id.to_string(),
        key_id: input.key_id.to_string(),
        key_identity: input.key_identity,
    };
    let statement_identity = artifact_auth_core::statement_identity(&statement)
        .map_err(|_| MoltenError::invalid_harness("world-head Artifact Auth statement is invalid"))?;
    let statement_ref = WorldHeadStatementRef::new(format!("blake3:{statement_identity}")).map_err(reference_error)?;
    Ok((statement, statement_ref))
}

pub fn canonical_world_head_state(state: &WorldHeadState) -> Result<(String, Vec<u8>)> {
    let value = world_head_state_value(state);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let state_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    Ok((state_ref, bytes))
}

pub fn parse_canonical_world_head_state(bytes: &[u8]) -> Result<WorldHeadState> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(
        &decoded.value,
        WORLD_HEAD_STATE_RECORD,
        WORLD_HEAD_STATE_FIELD_COUNT,
    )?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "world-head state schema")?;
    if schema != WORLD_HEAD_TRANSITION_SCHEMA {
        return Err(MoltenError::invalid_harness("unsupported world-head state schema"));
    }
    let state = WorldHeadState {
        branch_id: WorldBranchId::new(crate::preserves_rail::required_string_field(
            &named_field_value(&fields[1], "branch-id")?,
            "world-head state branch",
        )?)
        .map_err(reference_error)?,
        branch_class: WorldBranchClass::parse(&crate::preserves_rail::required_string_field(
            &named_field_value(&fields[2], "branch-class")?,
            "world-head state class",
        )?)
        .map_err(reference_error)?,
        head: WorldCommitRef::new(crate::preserves_rail::required_content_ref_string(
            &named_field_value(&fields[3], "head")?,
            "world-head state head",
        )?)
        .map_err(world_commit_reference_error)?,
        generation: required_u64(&named_field_value(&fields[4], "generation")?, "world-head state generation")?,
        policy_ref: WorldHeadPolicyRef::new(crate::preserves_rail::required_content_ref_string(
            &named_field_value(&fields[5], "policy-ref")?,
            "world-head state policy",
        )?)
        .map_err(reference_error)?,
    };
    let (_, canonical) = canonical_world_head_state(&state)?;
    if canonical != decoded.canonical_bytes {
        return Err(MoltenError::invalid_harness("world-head state bytes are not canonical"));
    }
    Ok(state)
}

pub fn canonical_world_head_conflict(conflict: &WorldHeadConflictSet) -> Result<CanonicalWorldHeadConflict> {
    let members = conflict
        .members
        .iter()
        .map(|member| {
            crate::preserves_rail::record("conflict-member", vec![
                crate::preserves_rail::string(member.claim_ref.as_str()),
                crate::preserves_rail::string(member.successor_head.as_str()),
            ])
        })
        .collect::<Vec<_>>();
    let value = crate::preserves_rail::record(WORLD_HEAD_CONFLICT_RECORD, vec![
        crate::preserves_rail::string(WORLD_HEAD_CONFLICT_SCHEMA),
        named_field("branch-id", crate::preserves_rail::string(conflict.branch_id.as_str())),
        named_field("expected-head", crate::preserves_rail::string(conflict.expected_head.as_str())),
        named_field("expected-generation", crate::preserves_rail::u64_value(conflict.expected_generation)),
        named_field("members", crate::preserves_rail::sequence(members)),
        named_field("conflict-ref", crate::preserves_rail::string(&conflict.conflict_ref)),
        non_claims_value(),
    ]);
    crate::preserves_rail::validate_boundary_schema(&value, &WORLD_HEAD_CONFLICT_BOUNDARY_SCHEMA)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldHeadConflict {
        conflict_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

pub fn canonical_world_head_transition_receipt(
    input: &WorldHeadTransitionReceiptInput<'_>,
) -> Result<CanonicalWorldHeadTransitionReceipt> {
    if !matches!(
        input.decision,
        TRANSITION_DECISION_ADMITTED | TRANSITION_DECISION_DENIED | TRANSITION_DECISION_CONFLICT
    ) {
        return Err(MoltenError::invalid_harness("unknown world-head receipt decision"));
    }
    let (before_head, before_generation, after_head, after_generation, currentness) =
        input
            .plan
            .map_or((None, None, None, None, WorldHeadCurrentnessClass::WholeStoreRollbackUnproven), |plan| {
                (
                    plan.before.as_ref().map(|state| state.head.as_str()),
                    plan.before.as_ref().map(|state| state.generation),
                    Some(plan.after.head.as_str()),
                    Some(plan.after.generation),
                    plan.currentness,
                )
            });
    let value = crate::preserves_rail::record(WORLD_HEAD_TRANSITION_RECEIPT_RECORD, vec![
        crate::preserves_rail::string(WORLD_HEAD_TRANSITION_SCHEMA),
        named_field("decision", crate::preserves_rail::string(input.decision)),
        named_field("claim-ref", crate::preserves_rail::string(input.claim_ref.as_str())),
        named_field("statement-ref", crate::preserves_rail::string(input.statement_ref.as_str())),
        named_field("authentication-decision-ref", crate::preserves_rail::string(input.authentication_decision_ref)),
        named_field("authority-ref", crate::preserves_rail::string(input.authority_ref)),
        named_field("before-head", crate::preserves_rail::optional_ref_value(before_head)),
        named_field("before-generation", optional_u64_value(before_generation)),
        named_field("after-head", crate::preserves_rail::optional_ref_value(after_head)),
        named_field("after-generation", optional_u64_value(after_generation)),
        named_field("currentness", crate::preserves_rail::string(currentness.as_str())),
        named_field(
            "issues",
            crate::preserves_rail::sequence(input.issue_codes.iter().map(crate::preserves_rail::string).collect()),
        ),
        non_claims_value(),
    ]);
    crate::preserves_rail::validate_boundary_schema(&value, &WORLD_HEAD_TRANSITION_RECEIPT_BOUNDARY_SCHEMA)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldHeadTransitionReceipt {
        receipt_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

fn world_head_claim_value(claim: &WorldHeadClaim) -> IOValue {
    let mut source_heads = claim.source_heads.iter().map(WorldCommitRef::as_str).collect::<Vec<_>>();
    source_heads.sort_unstable();
    crate::preserves_rail::record(WORLD_HEAD_CLAIM_RECORD, vec![
        crate::preserves_rail::string(WORLD_HEAD_CLAIM_SCHEMA),
        named_field("branch-id", crate::preserves_rail::string(claim.branch_id.as_str())),
        named_field("branch-class", crate::preserves_rail::string(claim.branch_class.as_str())),
        named_field(
            "expected-head",
            crate::preserves_rail::optional_ref_value(claim.expected_head.as_ref().map(WorldCommitRef::as_str)),
        ),
        named_field("successor-head", crate::preserves_rail::string(claim.successor_head.as_str())),
        named_field("expected-generation", crate::preserves_rail::u64_value(claim.expected_generation)),
        named_field("successor-generation", crate::preserves_rail::u64_value(claim.successor_generation)),
        named_field("purpose", crate::preserves_rail::string(claim.purpose.as_str())),
        named_field("policy-ref", crate::preserves_rail::string(claim.policy_ref.as_str())),
        named_field(
            "source-heads",
            crate::preserves_rail::sequence(source_heads.into_iter().map(crate::preserves_rail::string).collect()),
        ),
    ])
}

fn world_head_state_value(state: &WorldHeadState) -> IOValue {
    crate::preserves_rail::record(WORLD_HEAD_STATE_RECORD, vec![
        crate::preserves_rail::string(WORLD_HEAD_TRANSITION_SCHEMA),
        named_field("branch-id", crate::preserves_rail::string(state.branch_id.as_str())),
        named_field("branch-class", crate::preserves_rail::string(state.branch_class.as_str())),
        named_field("head", crate::preserves_rail::string(state.head.as_str())),
        named_field("generation", crate::preserves_rail::u64_value(state.generation)),
        named_field("policy-ref", crate::preserves_rail::string(state.policy_ref.as_str())),
    ])
}

fn named_field(label: &'static str, value: IOValue) -> IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn named_field_value(value: &preserves::Value<IOValue>, label: &str) -> Result<preserves::Value<IOValue>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} VALUE>")))?;
    Ok(fields[0].clone())
}

fn artifact_ref(profile: &str, reference: &str) -> Result<ArtifactRef> {
    Ok(ArtifactRef {
        profile: profile.to_string(),
        algorithm: ALGORITHM_BLAKE3.to_string(),
        digest_hex: crate::preserves_rail::content_ref_hex(reference)?.to_string(),
    })
}

fn required_u64(value: &preserves::Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn optional_u64_value(value: Option<u64>) -> IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
    )
}

fn non_claims_value() -> IOValue {
    crate::preserves_rail::record("non-claims", vec![crate::preserves_rail::sequence(
        WORLD_HEAD_NON_CLAIMS.iter().map(crate::preserves_rail::string).collect(),
    )])
}

fn reference_error(error: molten_core::world_head::WorldHeadReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world-head reference: {error}"))
}

fn world_commit_reference_error(error: molten_core::world_commit::WorldCommitReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world commit reference: {error:?}"))
}
