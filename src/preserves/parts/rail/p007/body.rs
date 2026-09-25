
fn validate_string_and_ref_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_TWO, spec, schema_ref)?;
    ensure_string(&record[0], field_spec.label, spec, schema_ref)?;
    ensure_content_ref(&record[1], field_spec.label, spec, schema_ref)
}

fn validate_ref_and_string_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_TWO, spec, schema_ref)?;
    ensure_content_ref(&record[0], field_spec.label, spec, schema_ref)?;
    ensure_string(&record[1], field_spec.label, spec, schema_ref).map(|_| ())
}

fn validate_two_refs_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_TWO, spec, schema_ref)?;
    ensure_content_ref(&record[0], field_spec.label, spec, schema_ref)?;
    ensure_content_ref(&record[1], field_spec.label, spec, schema_ref)
}

fn validate_file_refs_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let files = ensure_sequence(&record[0], field_spec.label, spec, schema_ref)?;
    for file in files.iter() {
        let file = value_to_iovalue(file);
        let fields = file.collect_simple_record("file", Some(FIELD_ARITY_TWO)).ok_or_else(|| {
            boundary_field_error(spec, field_spec.label, "<file string ref>", schema_ref)
        })?;
        ensure_string(&fields[0], "file name", spec, schema_ref)?;
        ensure_content_ref(&fields[1], "file ref", spec, schema_ref)?;
    }
    Ok(())
}

fn validate_conformance_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let value = value_to_iovalue(value);
    let fields = value.collect_simple_record(field_spec.label, Some(FIELD_ARITY_THREE)).ok_or_else(|| {
        boundary_field_error(spec, field_spec.label, "conformance record", schema_ref)
    })?;
    validate_ref_record(&fields[0], "positive", spec, schema_ref)?;
    validate_ref_record(&fields[1], "negative", spec, schema_ref)?;
    validate_ref_record(&fields[2], "property", spec, schema_ref)
}

fn validate_hostcall_descriptors_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let descriptors = ensure_sequence(&record[0], field_spec.label, spec, schema_ref)?;
    let mut seen = std::collections::BTreeSet::new();
    for descriptor in descriptors.iter() {
        let identity = validate_hostcall_descriptor_boundary_record(descriptor, spec, schema_ref)?;
        if !seen.insert(identity.clone()) {
            return Err(MoltenError::invalid_harness(format!(
                "{} schema validation deny: duplicate hostcall descriptor {identity} using schema {}",
                spec.family, schema_ref
            )));
        }
    }
    Ok(())
}

fn validate_hostcall_descriptor_boundary_record(
    value: &Value<IoValue>,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value.collect_simple_record("hostcall-descriptor", Some(HOSTCALL_DESCRIPTOR_ARITY)).ok_or_else(|| {
        boundary_field_error(spec, "hostcall-descriptor", "hostcall descriptor", schema_ref)
    })?;
    let operation_record = boundary_record(&fields[HOSTCALL_DESCRIPTOR_OPERATION_INDEX], "operation", FIELD_ARITY_ONE, spec, schema_ref)?;
    let operation = ensure_string(&operation_record[0], "operation", spec, schema_ref)?;
    OperationId::parse(operation.as_ref()).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: hostcall operation expected operation id using schema {}: {error}",
            spec.family, schema_ref
        ))
    })?;
    let descriptor_record = boundary_record(&fields[HOSTCALL_DESCRIPTOR_DESCRIPTOR_INDEX], "descriptor", FIELD_ARITY_ONE, spec, schema_ref)?;
    let descriptor_ref = ensure_content_ref_string(&descriptor_record[0], "descriptor", spec, schema_ref)?;
    validate_ref_record(&fields[HOSTCALL_DESCRIPTOR_INPUT_SCHEMA_INDEX], "input-schema", spec, schema_ref)?;
    validate_ref_record(&fields[HOSTCALL_DESCRIPTOR_OUTPUT_SCHEMA_INDEX], "output-schema", spec, schema_ref)?;
    validate_ref_sequence_record_with_contract(RefSequenceContractInput { value: &fields[HOSTCALL_DESCRIPTOR_AUTHORITY_INDEX], label: "authority", spec, schema_ref, require_non_empty: true, require_unique: true })?;
    validate_ref_sequence_record_with_contract(RefSequenceContractInput { value: &fields[HOSTCALL_DESCRIPTOR_RESOURCE_INDEX], label: "resource", spec, schema_ref, require_non_empty: true, require_unique: true })?;
    validate_ref_sequence_record_with_contract(RefSequenceContractInput { value: &fields[HOSTCALL_DESCRIPTOR_EFFECTS_INDEX], label: "effects", spec, schema_ref, require_non_empty: true, require_unique: true })?;
    let replay_record = boundary_record(&fields[HOSTCALL_DESCRIPTOR_REPLAY_INDEX], "replay", FIELD_ARITY_ONE, spec, schema_ref)?;
    let replay = ensure_string(&replay_record[0], "replay", spec, schema_ref)?;
    ReplayClass::parse(replay.as_ref()).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: hostcall replay class unsupported using schema {}: {error}",
            spec.family, schema_ref
        ))
    })?;
    validate_ref_sequence_record_with_contract(RefSequenceContractInput { value: &fields[HOSTCALL_DESCRIPTOR_ERRORS_INDEX], label: "errors", spec, schema_ref, require_non_empty: true, require_unique: true })?;
    Ok(format!("{}:{}", operation.as_ref(), descriptor_ref))
}

const HOSTCALL_DESCRIPTOR_ARITY: usize = 9;
const HOSTCALL_DESCRIPTOR_OPERATION_INDEX: usize = 0;
const HOSTCALL_DESCRIPTOR_DESCRIPTOR_INDEX: usize = 1;
const HOSTCALL_DESCRIPTOR_INPUT_SCHEMA_INDEX: usize = 2;
const HOSTCALL_DESCRIPTOR_OUTPUT_SCHEMA_INDEX: usize = 3;
const HOSTCALL_DESCRIPTOR_AUTHORITY_INDEX: usize = 4;
const HOSTCALL_DESCRIPTOR_RESOURCE_INDEX: usize = 5;
const HOSTCALL_DESCRIPTOR_EFFECTS_INDEX: usize = 6;
const HOSTCALL_DESCRIPTOR_REPLAY_INDEX: usize = 7;
const HOSTCALL_DESCRIPTOR_ERRORS_INDEX: usize = 8;

fn ensure_string<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<std::borrow::Cow<'a, str>> {
    value.as_string().ok_or_else(|| boundary_field_error(spec, label, "string", schema_ref))
}

fn ensure_content_ref(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    ensure_content_ref_string(value, label, spec, schema_ref).map(|_| ())
}

fn ensure_content_ref_string(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<String> {
    let reference = value
        .as_string()
        .ok_or_else(|| boundary_field_error(spec, label, "canonical content ref string", schema_ref))?;
    ContentRef::parse(reference.as_ref()).map(|_| reference.to_string()).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} expected canonical content ref string using schema {}: {error}",
            spec.family, schema_ref
        ))
    })
}

fn ensure_sequence<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<std::borrow::Cow<'a, [Value<IoValue>]>> {
    value
        .collect_sequence()
        .map(|sequence| match sequence {
            std::borrow::Cow::Borrowed(values) => std::borrow::Cow::Borrowed(values.as_slice()),
            std::borrow::Cow::Owned(values) => std::borrow::Cow::Owned(values),
        })
        .ok_or_else(|| boundary_field_error(spec, label, "sequence", schema_ref))
}

fn boundary_field_error(
    spec: &BoundarySchemaSpec,
    label: &str,
    expected: &str,
    schema_ref: &ContentRef,
) -> MoltenError {
    MoltenError::invalid_harness(format!(
        "{} schema validation deny: field {label} expected {expected} using schema {}",
        spec.family, schema_ref
    ))
}

pub const DEFAULT_STRUCTURAL_SCAN_MAX_NODES: usize = 8_192;
pub const DEFAULT_STRUCTURAL_SCAN_MAX_DEPTH: usize = 128;
pub const SENSITIVE_STRUCTURAL_MARKERS: &[&str] = &["secret", "confidential", "credential", "private", "encrypted-ref"];
pub const AMBIENT_JOB_TOKENS: &[&str] = &[
    "mobile-code",
    "raw-closure",
    "closure",
    "host-path",
    "source-path",
    "source-registry",
    "process-command",
    "command",
    "env",
    "environment",
    "source-text",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralTokenKind {
    RecordLabel,
    Symbol,
    String,
    ByteString,
    ContentRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralMatch {
    pub kind: StructuralTokenKind,
    pub token: String,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralInspectionScope {
    pub record_labels: bool,
    pub symbols: bool,
    pub strings: bool,
    pub byte_strings: bool,
    pub content_refs: bool,
}

impl StructuralInspectionScope {
    pub const fn structural_markers() -> Self {
        Self {
            record_labels: true,
            symbols: true,
            strings: false,
            byte_strings: false,
            content_refs: false,
        }
    }

    pub const fn content_refs() -> Self {
        Self {
            record_labels: false,
            symbols: false,
            strings: false,
            byte_strings: false,
            content_refs: true,
        }
    }

    pub const fn all() -> Self {
        Self {
            record_labels: true,
            symbols: true,
            strings: true,
            byte_strings: true,
            content_refs: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralInspectionLimits {
    pub max_nodes: usize,
    pub max_depth: usize,
}

impl Default for StructuralInspectionLimits {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_STRUCTURAL_SCAN_MAX_NODES,
            max_depth: DEFAULT_STRUCTURAL_SCAN_MAX_DEPTH,
        }
    }
}
