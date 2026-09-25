
pub const PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "plugin-hostcall-receipt",
    version: "v1",
    record_label: "plugin-hostcall-receipt-v1",
    schema_id: PLUGIN_HOSTCALL_RECEIPT_SCHEMA,
    fields: PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_FIELDS,
};

pub const PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "plugin-extension-contract",
    version: "v1",
    record_label: "plugin-extension-contract-v1",
    schema_id: PLUGIN_EXTENSION_CONTRACT_SCHEMA,
    fields: PLUGIN_EXTENSION_CONTRACT_BOUNDARY_FIELDS,
};

pub const RETENTION_RECEIPT_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "retention-receipt",
    version: "v1",
    record_label: "retention-receipt-v1",
    schema_id: RETENTION_RECEIPT_SCHEMA,
    fields: RETENTION_RECEIPT_BOUNDARY_FIELDS,
};

pub const EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "evidence-chain-segment-bundle",
    version: "v1",
    record_label: "chain-segment-bundle-v1",
    schema_id: EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA,
    fields: EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_FIELDS,
};

pub const OPERATOR_RELEASE_EVIDENCE_BUNDLE_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "operator-release-evidence-bundle",
    version: "v1",
    record_label: "release-evidence-bundle-v1",
    schema_id: OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA,
    fields: OPERATOR_RELEASE_EVIDENCE_BUNDLE_BOUNDARY_FIELDS,
};

// r[impl molten.preserves_schema_boundaries.schema_artifacts]
pub fn boundary_schema_artifact_value(spec: &BoundarySchemaSpec) -> Result<IoValue> {
    let arity = spec.arity()?;
    Ok(record("preserves-boundary-schema-artifact-v1", vec![
        record("family", vec![string(spec.family)]),
        record("version", vec![string(spec.version)]),
        record("preserves-schema-version", vec![string(preserves_schema::PRESERVES_SCHEMA_SPEC_VERSION)]),
        record("record-label", vec![string(spec.record_label)]),
        record("schema-id", vec![string(spec.schema_id)]),
        record("arity", vec![u64_value(arity)]),
        record("fields", vec![sequence(
            spec.fields.iter().map(boundary_field_contract_value).collect(),
        )]),
    ]))
}

pub fn boundary_schema_ref(spec: &BoundarySchemaSpec) -> Result<ContentRef> {
    canonical_content_ref(&boundary_schema_artifact_value(spec)?)
}

pub fn validate_boundary_claimed_schema_ref(spec: &BoundarySchemaSpec, claimed_ref: &str) -> Result<ContentRef> {
    let expected = boundary_schema_ref(spec)?;
    let claimed = ContentRef::parse(claimed_ref).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: claimed schema ref is invalid using current schema {}: {error}",
            spec.family, expected
        ))
    })?;
    if claimed == expected {
        return Ok(expected);
    }
    Err(MoltenError::invalid_harness(format!(
        "{} schema validation deny: stale schema ref {} expected {}",
        spec.family, claimed, expected
    )))
}

fn boundary_field_contract_value(field: &BoundaryFieldSpec) -> IoValue {
    record("field", vec![
        record("label", vec![string(field.label)]),
        record("kind", vec![string(boundary_field_kind_name(field.kind))]),
        record("constraints", vec![sequence(
            boundary_field_constraints(field.kind)
                .iter()
                .map(|constraint| string(*constraint))
                .collect(),
        )]),
    ])
}

fn boundary_field_kind_name(kind: BoundaryFieldKind) -> &'static str {
    match kind {
        BoundaryFieldKind::SchemaId => "schema-id",
        BoundaryFieldKind::AnyRecord => "any-record",
        BoundaryFieldKind::AnySequenceRecord => "any-sequence-record",
        BoundaryFieldKind::ChainRecord => "chain-record",
        BoundaryFieldKind::ChecksRecord => "checks-record",
        BoundaryFieldKind::ConformanceRecord => "conformance-record",
        BoundaryFieldKind::DecisionRecord => "decision-record",
        BoundaryFieldKind::FileRefsRecord => "file-refs-record",
        BoundaryFieldKind::HostcallDescriptorsRecord => "hostcall-descriptors-record",
        BoundaryFieldKind::NonEmptyRefSequenceRecord => "non-empty-ref-sequence-record",
        BoundaryFieldKind::NonEmptyStringRecord => "non-empty-string-record",
        BoundaryFieldKind::ObjectRecord => "object-record",
        BoundaryFieldKind::OptionalRefRecord => "optional-ref-record",
        BoundaryFieldKind::RefAndStringRecord => "ref-and-string-record",
        BoundaryFieldKind::RefRecord => "ref-record",
        BoundaryFieldKind::RefSequenceRecord => "ref-sequence-record",
        BoundaryFieldKind::StableIdRecord => "stable-id-record",
        BoundaryFieldKind::StringAndRefRecord => "string-and-ref-record",
        BoundaryFieldKind::StringRecord => "string-record",
        BoundaryFieldKind::StringSequenceRecord => "string-sequence-record",
        BoundaryFieldKind::UniqueRefSequenceRecord => "unique-ref-sequence-record",
        BoundaryFieldKind::UniqueStringSequenceRecord => "unique-string-sequence-record",
        BoundaryFieldKind::TwoRefsRecord => "two-refs-record",
        BoundaryFieldKind::U64Record => "u64-record",
    }
}

fn boundary_field_constraints(kind: BoundaryFieldKind) -> &'static [&'static str] {
    match kind {
        BoundaryFieldKind::SchemaId => &["schema-id", "exact-current-schema"],
        BoundaryFieldKind::AnyRecord => &["record", "arity-one", "embedded-record"],
        BoundaryFieldKind::AnySequenceRecord => &["record", "arity-one", "sequence"],
        BoundaryFieldKind::ChainRecord => &["record", "chain-fields"],
        BoundaryFieldKind::ChecksRecord => &["record", "checks", "unique-check-names", "known-check-status"],
        BoundaryFieldKind::ConformanceRecord => &["record", "positive-negative-property-refs"],
        BoundaryFieldKind::DecisionRecord => &["record", "string", "decision-pass-or-deny"],
        BoundaryFieldKind::FileRefsRecord => &["record", "sequence", "file-ref-items"],
        BoundaryFieldKind::HostcallDescriptorsRecord => &["record", "sequence", "unique-operation-descriptor", "typed-embedded-record"],
        BoundaryFieldKind::NonEmptyRefSequenceRecord => &["record", "sequence", "content-ref-items", "non-empty"],
        BoundaryFieldKind::NonEmptyStringRecord => &["record", "string", "non-empty"],
        BoundaryFieldKind::ObjectRecord => &["record", "object-ref-and-kind"],
        BoundaryFieldKind::OptionalRefRecord => &["record", "optional-content-ref"],
        BoundaryFieldKind::RefAndStringRecord => &["record", "content-ref", "string"],
        BoundaryFieldKind::RefRecord => &["record", "content-ref"],
        BoundaryFieldKind::RefSequenceRecord => &["record", "sequence", "content-ref-items"],
        BoundaryFieldKind::StableIdRecord => &["record", "string", "stable-id"],
        BoundaryFieldKind::StringAndRefRecord => &["record", "string", "content-ref"],
        BoundaryFieldKind::StringRecord => &["record", "string"],
        BoundaryFieldKind::StringSequenceRecord => &["record", "sequence", "string-items"],
        BoundaryFieldKind::UniqueRefSequenceRecord => &["record", "sequence", "content-ref-items", "unique"],
        BoundaryFieldKind::UniqueStringSequenceRecord => &["record", "sequence", "string-items", "unique"],
        BoundaryFieldKind::TwoRefsRecord => &["record", "two-content-refs"],
        BoundaryFieldKind::U64Record => &["record", "u64"],
    }
}

// r[impl molten.preserves_schema_boundaries.schema_adapter]
// r[impl molten.preserves_schema_boundaries.schema_denials]
pub fn validate_boundary_schema(value: &IoValue, spec: &BoundarySchemaSpec) -> Result<BoundarySchemaValidation> {
    let schema_ref = boundary_schema_ref(spec)?;
    let value_ref = canonical_content_ref(value)?;
    let arity = spec.fields.len();
    let fields = value.collect_simple_record(spec.record_label, Some(arity)).ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: expected <{} ...> with arity {} using schema {}",
            spec.family, spec.record_label, arity, schema_ref
        ))
    })?;
    for (index, field_spec) in spec.fields.iter().enumerate() {
        validate_boundary_field(&fields[index], field_spec, spec, &schema_ref)?;
    }
    Ok(BoundarySchemaValidation {
        family: spec.family.to_string(),
        schema_ref,
        value_ref,
        decision: "pass".to_string(),
        diagnostics: Vec::new(),
    })
}

pub fn boundary_schema_diagnostic_value(validation: &BoundarySchemaValidation) -> IoValue {
    record("preserves-boundary-schema-validation-v1", vec![
        record("family", vec![string(&validation.family)]),
        record("schema-ref", vec![string(validation.schema_ref.as_str())]),
        record("value-ref", vec![string(validation.value_ref.as_str())]),
        record("decision", vec![string(&validation.decision)]),
        record("diagnostics", vec![sequence(validation.diagnostics.iter().map(string).collect())]),
    ])
}

// r[impl molten.preserves_boundary_codegen.typed_codecs]
// r[impl molten.preserves_boundary_codegen.strict_decode]
// r[impl molten.preserves_boundary_codegen.schema_ref_evidence]
pub fn validate_boundary_bytes(bytes: &[u8], spec: &BoundarySchemaSpec) -> Result<BoundaryCodecReport> {
    let decoded = strict_canonical_decode(bytes)?;
    let schema_ref = boundary_schema_ref(spec)?;
    let input_bytes_ref = ContentRef::parse(content_ref_from_bytes(bytes))?;
    let typed_value_ref = boundary_typed_value_ref(spec, decoded.value_ref.as_str())?;
    match validate_boundary_schema(&decoded.value, spec) {
        Ok(validation) => Ok(BoundaryCodecReport {
            family: validation.family,
            schema_ref: validation.schema_ref,
            input_bytes_ref,
            decoded_value_ref: validation.value_ref,
            typed_value_ref,
            decision: "pass".to_string(),
            diagnostics: Vec::new(),
        }),
        Err(error) => Ok(BoundaryCodecReport {
            family: spec.family.to_string(),
            schema_ref,
            input_bytes_ref,
            decoded_value_ref: decoded.value_ref,
            typed_value_ref,
            decision: "deny".to_string(),
            diagnostics: vec![error.to_string()],
        }),
    }
}

pub fn boundary_codec_report_value(report: &BoundaryCodecReport) -> IoValue {
    record("preserves-boundary-codec-report-v1", vec![
        record("family", vec![string(&report.family)]),
        record("schema-ref", vec![string(report.schema_ref.as_str())]),
        record("input-bytes-ref", vec![string(report.input_bytes_ref.as_str())]),
        record("decoded-value-ref", vec![string(report.decoded_value_ref.as_str())]),
        record("typed-value-ref", vec![string(report.typed_value_ref.as_str())]),
        record("decision", vec![string(&report.decision)]),
        record("diagnostics", vec![sequence(report.diagnostics.iter().map(string).collect())]),
    ])
}

fn boundary_typed_value_ref(spec: &BoundarySchemaSpec, decoded_value_ref: &str) -> Result<ContentRef> {
    canonical_content_ref(&record("preserves-boundary-typed-codec-v1", vec![
        record("family", vec![string(spec.family)]),
        record("schema-ref", vec![string(boundary_schema_ref(spec)?.as_str())]),
        record("decoded-value-ref", vec![string(decoded_value_ref)]),
    ]))
}

// r[impl molten.preserves_boundary_field_contracts.field_contracts]
// r[impl molten.preserves_boundary_field_contracts.field_contract_denials]
fn validate_boundary_field(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    match field_spec.kind {
        BoundaryFieldKind::SchemaId => validate_boundary_schema_id(value, spec, schema_ref),
        BoundaryFieldKind::AnyRecord => {
            boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
            Ok(())
        }
        BoundaryFieldKind::AnySequenceRecord => validate_any_sequence_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::ChainRecord => validate_chain_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::ChecksRecord => validate_checks_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::ConformanceRecord => validate_conformance_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::DecisionRecord => validate_decision_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::FileRefsRecord => validate_file_refs_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::HostcallDescriptorsRecord => validate_hostcall_descriptors_boundary_record(
            value,
            field_spec,
            spec,
            schema_ref,
        ),
        BoundaryFieldKind::NonEmptyRefSequenceRecord => {
            validate_ref_sequence_record_with_contract(RefSequenceContractInput { value, label: field_spec.label, spec, schema_ref, require_non_empty: true, require_unique: false })
        }
        BoundaryFieldKind::NonEmptyStringRecord => validate_non_empty_string_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::ObjectRecord => validate_object_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::OptionalRefRecord => validate_optional_ref_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::RefAndStringRecord => validate_ref_and_string_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::RefRecord => validate_ref_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::RefSequenceRecord => validate_ref_sequence_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::StableIdRecord => validate_stable_id_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::StringAndRefRecord => validate_string_and_ref_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::StringRecord => validate_string_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::StringSequenceRecord => validate_string_sequence_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::UniqueRefSequenceRecord => {
            validate_ref_sequence_record_with_contract(RefSequenceContractInput { value, label: field_spec.label, spec, schema_ref, require_non_empty: false, require_unique: true })
        }
        BoundaryFieldKind::UniqueStringSequenceRecord => validate_unique_string_sequence_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::TwoRefsRecord => validate_two_refs_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::U64Record => validate_u64_record(value, field_spec.label, spec, schema_ref),
    }
}

const FIELD_ARITY_ZERO: usize = 0;
const FIELD_ARITY_ONE: usize = 1;
const FIELD_ARITY_TWO: usize = 2;
const FIELD_ARITY_THREE: usize = 3;
