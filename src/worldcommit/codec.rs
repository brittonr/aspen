use molten_core::world_commit::CompletenessClaim;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::SnapshotCohortRef;
use molten_core::world_commit::SnapshotProfile;
use molten_core::world_commit::SnapshotProfileKind;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitCore;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldCommitVersion;
use molten_core::world_commit::WorldRootRef;
use preserves::IOValue;
use preserves::Value;
use preserves::ValueImpl;

use super::CanonicalWorldCommit;
use super::WORLD_COMMIT_RECORD;
use super::canonical_world_commit;
use crate::error::MoltenError;
use crate::error::Result;

const WORLD_COMMIT_RECORD_ARITY: usize = 6;
const PROFILE_RECORD_ARITY: usize = 3;
const TYPED_ROOT_RECORD_ARITY: usize = 2;

// r[impl molten.world_commit.core]
// r[impl molten.world_commit.verification]
pub fn parse_canonical_world_commit(bytes: &[u8], bounds: &WorldCommitBounds) -> Result<CanonicalWorldCommit> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields =
        crate::preserves_rail::simple_record_fields(&decoded.value, WORLD_COMMIT_RECORD, WORLD_COMMIT_RECORD_ARITY)?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "world commit schema")?;
    if schema != molten_core::world_commit::WORLD_COMMIT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!("world commit schema {schema} is not supported")));
    }
    let core = WorldCommitCore {
        version: parse_version(&fields[1])?,
        profile: parse_profile(&fields[2])?,
        parents: parse_parents(&fields[3], bounds.max_parents)?,
        roots: parse_roots(&fields[4], bounds.max_roots)?,
        completeness: parse_completeness(&fields[5], bounds.max_roots)?,
    };
    let canonical = canonical_world_commit(&core, bounds)?;
    if canonical.bytes != bytes {
        return Err(MoltenError::invalid_harness(
            "world commit input is canonical Preserves but not normalized world-commit order",
        ));
    }
    Ok(canonical)
}

pub fn parse_canonical_world_commit_with_ref(
    bytes: &[u8],
    expected_ref: &WorldCommitRef,
    bounds: &WorldCommitBounds,
) -> Result<CanonicalWorldCommit> {
    let canonical = parse_canonical_world_commit(bytes, bounds)?;
    if canonical.commit_ref != *expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "world commit identity mismatch: expected {expected_ref}, got {}",
            canonical.commit_ref
        )));
    }
    Ok(canonical)
}

fn parse_version(value: &Value<IOValue>) -> Result<WorldCommitVersion> {
    let fields = wrapped_fields(value, "version", 1)?;
    let version = crate::preserves_rail::required_string_field(&fields[0], "world commit version")?;
    WorldCommitVersion::parse(&version)
        .map_err(|_| MoltenError::invalid_harness(format!("unsupported world commit version {version}")))
}

fn parse_profile(value: &Value<IOValue>) -> Result<SnapshotProfile> {
    let fields = wrapped_fields(value, "profile", PROFILE_RECORD_ARITY)?;
    let kind_text = crate::preserves_rail::required_string_field(&fields[0], "snapshot profile kind")?;
    let kind = SnapshotProfileKind::parse(&kind_text)
        .map_err(|_| MoltenError::invalid_harness(format!("unsupported snapshot profile kind {kind_text}")))?;
    let profile_ref = crate::preserves_rail::required_content_ref_string(&fields[1], "snapshot profile ref")?;
    let cohort_ref = parse_optional_ref(&fields[2], "snapshot cohort ref")?
        .map(SnapshotCohortRef::new)
        .transpose()
        .map_err(|issue| MoltenError::invalid_harness(format!("invalid snapshot cohort ref: {issue:?}")))?;
    Ok(SnapshotProfile {
        kind,
        profile_ref: SnapshotProfileRef::new(profile_ref)
            .map_err(|issue| MoltenError::invalid_harness(format!("invalid snapshot profile ref: {issue:?}")))?,
        cohort_ref,
    })
}

fn parse_parents(value: &Value<IOValue>, maximum: usize) -> Result<Vec<WorldCommitRef>> {
    let values = wrapped_sequence(value, "parents", maximum)?;
    values
        .as_slice()
        .iter()
        .map(|value| {
            let reference = crate::preserves_rail::required_content_ref_string(value, "world commit parent")?;
            WorldCommitRef::new(reference)
                .map_err(|issue| MoltenError::invalid_harness(format!("invalid world commit parent: {issue:?}")))
        })
        .collect()
}

fn parse_roots(value: &Value<IOValue>, maximum: usize) -> Result<Vec<WorldRootRef>> {
    let values = wrapped_sequence(value, "roots", maximum)?;
    values.as_slice().iter().map(parse_root).collect()
}

fn parse_root(value: &Value<IOValue>) -> Result<WorldRootRef> {
    let fields = wrapped_fields(value, "typed-root", TYPED_ROOT_RECORD_ARITY)?;
    let kind_text = crate::preserves_rail::required_string_field(&fields[0], "world root kind")?;
    let kind = RootKind::parse(&kind_text)
        .map_err(|_| MoltenError::invalid_harness(format!("unsupported world root kind {kind_text}")))?;
    let reference = crate::preserves_rail::required_content_ref_string(&fields[1], "world root ref")?;
    WorldRootRef::parse(kind, reference)
        .map_err(|issue| MoltenError::invalid_harness(format!("invalid {} root ref: {issue:?}", kind.as_str())))
}

fn parse_completeness(value: &Value<IOValue>, maximum: usize) -> Result<CompletenessClaim> {
    let values = wrapped_sequence(value, "completeness", maximum)?;
    let mut required_roots = Vec::with_capacity(values.len());
    for value in values.as_slice() {
        let kind_text = crate::preserves_rail::required_string_field(value, "completeness root kind")?;
        required_roots
            .push(RootKind::parse(&kind_text).map_err(|_| {
                MoltenError::invalid_harness(format!("unsupported completeness root kind {kind_text}"))
            })?);
    }
    Ok(CompletenessClaim { required_roots })
}

fn parse_optional_ref(value: &Value<IOValue>, field: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let fields = value
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <none> or <some ref> for {field}")))?;
    crate::preserves_rail::required_content_ref_string(&fields[0], field).map(Some)
}

fn wrapped_fields(value: &Value<IOValue>, label: &str, arity: usize) -> Result<Vec<Value<IOValue>>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, label, arity)?;
    Ok(fields.fields_iter().cloned().collect())
}

fn wrapped_sequence(value: &Value<IOValue>, label: &str, maximum: usize) -> Result<Vec<Value<IOValue>>> {
    let fields = wrapped_fields(value, label, 1)?;
    let values = crate::preserves_rail::required_sequence_field(&fields[0], label)?;
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!(
            "world commit {label} count {} exceeds maximum {maximum}",
            values.len()
        )));
    }
    Ok(values.into_owned())
}

const WORLD_COMMIT_BOUNDARY_SCHEMA_COUNT: usize = 4;
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

const WORLD_COMMIT_FIELDS: &[crate::preserves_rail::BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    boundary_field!("version", AnyRecord),
    boundary_field!("profile", AnyRecord),
    boundary_field!("parents", AnySequenceRecord),
    boundary_field!("roots", AnySequenceRecord),
    boundary_field!("completeness", AnySequenceRecord),
];
const CAPTURE_RECEIPT_FIELDS: &[crate::preserves_rail::BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    boundary_field!("decision", StringRecord),
    boundary_field!("commit-ref", OptionalRefRecord),
    boundary_field!("profile-ref", RefRecord),
    boundary_field!("persisted-roots", AnySequenceRecord),
    boundary_field!("revision-fences", AnySequenceRecord),
    boundary_field!("publication", StringRecord),
    boundary_field!("issues", StringSequenceRecord),
    boundary_field!("non-claims", UniqueStringSequenceRecord),
];
const CLOSURE_REPORT_FIELDS: &[crate::preserves_rail::BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    boundary_field!("commit-ref", RefRecord),
    boundary_field!("decision", StringRecord),
    boundary_field!("first-missing-root", AnyRecord),
    boundary_field!("issues", StringSequenceRecord),
    boundary_field!("non-claims", UniqueStringSequenceRecord),
];
const RESTORE_PLAN_FIELDS: &[crate::preserves_rail::BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    boundary_field!("commit-ref", RefRecord),
    boundary_field!("steps", AnySequenceRecord),
    boundary_field!("replay", AnySequenceRecord),
    boundary_field!("current-admission-required", AnyRecord),
    boundary_field!("non-claims", UniqueStringSequenceRecord),
];

pub const WORLD_COMMIT_BOUNDARY_SCHEMA: crate::preserves_rail::BoundarySchemaSpec =
    crate::preserves_rail::BoundarySchemaSpec {
        family: "molten-world-commit",
        version: "v1",
        record_label: super::WORLD_COMMIT_RECORD,
        schema_id: molten_core::world_commit::WORLD_COMMIT_SCHEMA,
        fields: WORLD_COMMIT_FIELDS,
    };
pub const WORLD_COMMIT_CAPTURE_RECEIPT_BOUNDARY_SCHEMA: crate::preserves_rail::BoundarySchemaSpec =
    crate::preserves_rail::BoundarySchemaSpec {
        family: "molten-world-commit-capture-receipt",
        version: "v1",
        record_label: super::WORLD_COMMIT_CAPTURE_RECEIPT_RECORD,
        schema_id: molten_core::world_commit::WORLD_COMMIT_CAPTURE_RECEIPT_SCHEMA,
        fields: CAPTURE_RECEIPT_FIELDS,
    };
pub const WORLD_COMMIT_CLOSURE_REPORT_BOUNDARY_SCHEMA: crate::preserves_rail::BoundarySchemaSpec =
    crate::preserves_rail::BoundarySchemaSpec {
        family: "molten-world-commit-closure-report",
        version: "v1",
        record_label: super::WORLD_COMMIT_CLOSURE_REPORT_RECORD,
        schema_id: molten_core::world_commit::WORLD_COMMIT_CLOSURE_REPORT_SCHEMA,
        fields: CLOSURE_REPORT_FIELDS,
    };
pub const WORLD_COMMIT_RESTORE_PLAN_BOUNDARY_SCHEMA: crate::preserves_rail::BoundarySchemaSpec =
    crate::preserves_rail::BoundarySchemaSpec {
        family: "molten-world-commit-restore-plan",
        version: "v1",
        record_label: super::WORLD_COMMIT_RESTORE_PLAN_RECORD,
        schema_id: molten_core::world_commit::WORLD_COMMIT_RESTORE_PLAN_SCHEMA,
        fields: RESTORE_PLAN_FIELDS,
    };

pub const WORLD_COMMIT_BOUNDARY_SCHEMAS: [crate::preserves_rail::BoundarySchemaSpec;
    WORLD_COMMIT_BOUNDARY_SCHEMA_COUNT] = [
    WORLD_COMMIT_BOUNDARY_SCHEMA,
    WORLD_COMMIT_CAPTURE_RECEIPT_BOUNDARY_SCHEMA,
    WORLD_COMMIT_CLOSURE_REPORT_BOUNDARY_SCHEMA,
    WORLD_COMMIT_RESTORE_PLAN_BOUNDARY_SCHEMA,
];
