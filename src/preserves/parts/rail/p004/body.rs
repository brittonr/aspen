
pub fn optional_content_ref_string(value: &Value<IoValue>, field: &str) -> Result<Option<String>> {
    Ok(optional_content_ref(value, field)?.map(ContentRef::into_string))
}

pub fn required_sequence_field<'a>(
    value: &'a Value<IoValue>,
    field: &str,
) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

pub fn record_string_field(value: &Value<IoValue>, record_name: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record_fields(&value, record_name, 1)?;
    required_string_field(&record[0], field)
}

pub fn record_content_ref(value: &Value<IoValue>, record_name: &str, field: &str) -> Result<ContentRef> {
    let value = value_to_iovalue(value);
    let record = simple_record_fields(&value, record_name, 1)?;
    required_content_ref(&record[0], field)
}

pub fn record_content_ref_string(value: &Value<IoValue>, record_name: &str, field: &str) -> Result<String> {
    Ok(record_content_ref(value, record_name, field)?.into_string())
}

pub fn record_content_ref_sequence(
    value: &Value<IoValue>,
    record_name: &str,
    field: &str,
    maximum: u64,
) -> Result<Vec<ContentRef>> {
    let maximum = crate::bounded::usize_from_u64(maximum, field)?;
    let value = value_to_iovalue(value);
    let record = simple_record_fields(&value, record_name, 1)?;
    let values = required_sequence_field(&record[0], field)?;
    ensure_toolkit_count_at_most(values.len(), maximum, field)?;
    let mut refs = Vec::with_capacity(values.len());
    for item in values.iter() {
        refs.push(required_content_ref(item, field)?);
    }
    Ok(refs)
}

pub fn record_content_ref_strings(
    value: &Value<IoValue>,
    record_name: &str,
    field: &str,
    maximum: u64,
) -> Result<Vec<String>> {
    Ok(record_content_ref_sequence(value, record_name, field, maximum)?
        .into_iter()
        .map(ContentRef::into_string)
        .collect())
}

pub fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

pub fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

// r[impl molten.preserves_rail_toolkit.check_sets]
pub fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

pub fn parse_checks_record(
    value: &Value<IoValue>,
    maximum: u64,
    context: &str,
) -> Result<Vec<ParsedCheck>> {
    let maximum = crate::bounded::usize_from_u64(maximum, "checks")?;
    let value = value_to_iovalue(value);
    let record = simple_record_fields(&value, "checks", 1)?;
    let values = required_sequence_field(&record[0], "checks")?;
    ensure_toolkit_count_at_most(values.len(), maximum, "checks")?;
    let mut checks = Vec::with_capacity(values.len());
    let mut seen = std::collections::BTreeSet::new();
    for item in values.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record_fields(&item, "check", 2)?;
        let name = required_string_field(&check[0], "check name")?;
        let status = required_string_field(&check[1], "check status")?;
        if !matches!(status.as_str(), "pass" | "fail" | "deny") {
            return Err(MoltenError::invalid_harness(format!("unsupported {context} check status {status}")));
        }
        if !seen.insert(name.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate {context} check {name}")));
        }
        checks.push(ParsedCheck { name, status });
    }
    Ok(checks)
}

pub fn require_checks_present(checks: &[ParsedCheck], expected: &[&str], context: &str) -> Result<()> {
    for expected in expected {
        if !checks.iter().any(|check| check.name == *expected) {
            return Err(MoltenError::invalid_harness(format!("missing {context} check {expected}")));
        }
    }
    Ok(())
}

fn ensure_toolkit_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryFieldKind {
    SchemaId,
    AnyRecord,
    AnySequenceRecord,
    ChainRecord,
    ChecksRecord,
    ConformanceRecord,
    DecisionRecord,
    FileRefsRecord,
    HostcallDescriptorsRecord,
    NonEmptyRefSequenceRecord,
    NonEmptyStringRecord,
    ObjectRecord,
    OptionalRefRecord,
    RefAndStringRecord,
    RefRecord,
    RefSequenceRecord,
    StableIdRecord,
    StringAndRefRecord,
    StringRecord,
    StringSequenceRecord,
    UniqueRefSequenceRecord,
    UniqueStringSequenceRecord,
    TwoRefsRecord,
    U64Record,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryFieldSpec {
    pub label: &'static str,
    pub kind: BoundaryFieldKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundarySchemaSpec {
    pub family: &'static str,
    pub version: &'static str,
    pub record_label: &'static str,
    pub schema_id: &'static str,
    pub fields: &'static [BoundaryFieldSpec],
}

impl BoundarySchemaSpec {
    pub fn arity(&self) -> Result<u64> {
        crate::bounded::u64_from_usize(self.fields.len(), "boundary schema arity")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundarySchemaValidation {
    pub family: String,
    pub schema_ref: ContentRef,
    pub value_ref: ContentRef,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryCodecReport {
    pub family: String,
    pub schema_ref: ContentRef,
    pub input_bytes_ref: ContentRef,
    pub decoded_value_ref: ContentRef,
    pub typed_value_ref: ContentRef,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

const SCHEMA_FIELD: BoundaryFieldSpec = BoundaryFieldSpec {
    label: "schema-id",
    kind: BoundaryFieldKind::SchemaId,
};

const NODE_CONTROL_INGRESS_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "transport", kind: BoundaryFieldKind::NonEmptyStringRecord },
    BoundaryFieldSpec { label: "topic", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "from-peer", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "to-node", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "sequence", kind: BoundaryFieldKind::NonEmptyStringRecord },
    BoundaryFieldSpec { label: "operation", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "request-ref", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "request", kind: BoundaryFieldKind::AnyRecord },
    BoundaryFieldSpec { label: "peer-bootstrap", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "authority", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "policy", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "resource", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "evidence", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "decision", kind: BoundaryFieldKind::DecisionRecord },
    BoundaryFieldSpec { label: "plugin", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "manifest", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "operation", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "hostcall", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "executor", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "effect", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "authority", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "capability-grants", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "resource", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "evaluation-turn", kind: BoundaryFieldKind::U64Record },
    BoundaryFieldSpec { label: "diagnostics", kind: BoundaryFieldKind::StringSequenceRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const PLUGIN_EXTENSION_CONTRACT_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "extension-id", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "version", kind: BoundaryFieldKind::NonEmptyStringRecord },
    BoundaryFieldSpec { label: "host-abi", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "lifecycle", kind: BoundaryFieldKind::UniqueStringSequenceRecord },
    BoundaryFieldSpec { label: "hostcalls", kind: BoundaryFieldKind::HostcallDescriptorsRecord },
    BoundaryFieldSpec { label: "conformance", kind: BoundaryFieldKind::ConformanceRecord },
    BoundaryFieldSpec { label: "policy", kind: BoundaryFieldKind::NonEmptyRefSequenceRecord },
    BoundaryFieldSpec { label: "supply-chain", kind: BoundaryFieldKind::NonEmptyRefSequenceRecord },
    BoundaryFieldSpec { label: "profile", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const RETENTION_RECEIPT_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "decision", kind: BoundaryFieldKind::DecisionRecord },
    BoundaryFieldSpec { label: "action", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "object", kind: BoundaryFieldKind::ObjectRecord },
    BoundaryFieldSpec { label: "class", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "requester", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "index", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "pins", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "retained", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "remote", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "tombstone", kind: BoundaryFieldKind::OptionalRefRecord },
    BoundaryFieldSpec { label: "diagnostics", kind: BoundaryFieldKind::StringSequenceRecord },
    BoundaryFieldSpec { label: "policy", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "chain", kind: BoundaryFieldKind::ChainRecord },
    BoundaryFieldSpec { label: "anchor", kind: BoundaryFieldKind::OptionalRefRecord },
    BoundaryFieldSpec { label: "head", kind: BoundaryFieldKind::OptionalRefRecord },
    BoundaryFieldSpec { label: "artifacts", kind: BoundaryFieldKind::AnySequenceRecord },
    BoundaryFieldSpec { label: "verify-receipts", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "checkpoints", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const OPERATOR_RELEASE_EVIDENCE_BUNDLE_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "output-path", kind: BoundaryFieldKind::StringAndRefRecord },
    BoundaryFieldSpec { label: "members", kind: BoundaryFieldKind::FileRefsRecord },
    BoundaryFieldSpec { label: "dogfood", kind: BoundaryFieldKind::TwoRefsRecord },
    BoundaryFieldSpec { label: "replay", kind: BoundaryFieldKind::TwoRefsRecord },
    BoundaryFieldSpec { label: "nix", kind: BoundaryFieldKind::TwoRefsRecord },
    BoundaryFieldSpec { label: "nextest", kind: BoundaryFieldKind::RefAndStringRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

pub const NODE_CONTROL_INGRESS_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "node-control-ingress-envelope",
    version: "v1",
    record_label: "node-control-ingress-envelope-v1",
    schema_id: NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA,
    fields: NODE_CONTROL_INGRESS_BOUNDARY_FIELDS,
};
