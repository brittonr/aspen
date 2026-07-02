
pub fn octet_gate_policy_value(input: &OctetGateInput) -> IoValue {
    record("octet-gate-policy-v1", vec![
        string(crate::preserves_rail::OCTET_GATE_POLICY_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        record("command", vec![sequence(vec![
            string("cargo"),
            string("octet"),
            string("check"),
            string("--artifact-dir"),
            string(input.artifacts_dir.to_string_lossy()),
        ])]),
        record("required-artifacts", vec![sequence(vec![
            string(COMMAND_NAME),
            string(STATUS_NAME),
            string(SUMMARY_NAME),
            string(OBJECT_CORPUS_RECEIPT_NAME),
        ])]),
        record("deny-statuses", vec![sequence(vec![
            string("warning-only"),
            string("lint-failure"),
            string("integration-failure"),
            string("missing"),
            string("malformed"),
            string("stale"),
        ])]),
        record("critical-lints", vec![sequence(CRITICAL_LINTS.iter().map(string).collect())]),
        record("quarantine-policy", vec![record("none", Vec::new())]),
        checks_value(&[
            Check {
                name: "strict-profile",
                status: "pass",
            },
            Check {
                name: "warning-only-denies",
                status: "pass",
            },
            Check {
                name: "required-artifacts-bound",
                status: "pass",
            },
        ]),
    ])
}

struct OctetGateReceiptInput<'a> {
    decision: &'a str,
    policy_ref: &'a str,
    command_ref: Option<&'a str>,
    status_ref: Option<&'a str>,
    summary_ref: Option<&'a str>,
    structured_findings_ref: Option<&'a str>,
    object_corpus_ref: Option<&'a str>,
    fingerprint_evidence_ref: Option<&'a str>,
    config_hash: Option<&'a str>,
    profile_hash: Option<&'a str>,
    toolchain: Option<&'a str>,
    counts: &'a FindingCounts,
    diagnostics: &'a [String],
    checks: &'a [Check],
}

struct OctetSourceGateValidationValueInput<'a> {
    decision: &'a str,
    requirement_ref: &'a str,
    gate_receipt_ref: Option<&'a str>,
    policy_ref: Option<&'a str>,
    status_ref: Option<&'a str>,
    summary_ref: Option<&'a str>,
    findings_ref: Option<&'a str>,
    object_corpus_ref: Option<&'a str>,
    fingerprint_ref: Option<&'a str>,
    counts: &'a FindingCounts,
    diagnostics: &'a [String],
    checks: &'a [Check],
}

fn octet_source_gate_validation_value(input: OctetSourceGateValidationValueInput<'_>) -> IoValue {
    record("octet-source-gate-validation-v1", vec![
        string(crate::preserves_rail::OCTET_SOURCE_GATE_VALIDATION_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("requirement", vec![string(input.requirement_ref)]),
        record("gate-receipt", vec![optional_ref(input.gate_receipt_ref)]),
        record("gate-policy", vec![optional_ref(input.policy_ref)]),
        record("status", vec![optional_ref(input.status_ref)]),
        record("summary", vec![optional_ref(input.summary_ref)]),
        record("findings", vec![optional_ref(input.findings_ref)]),
        record("object-corpus", vec![optional_ref(input.object_corpus_ref)]),
        record("fingerprint", vec![optional_ref(input.fingerprint_ref)]),
        counts_value(input.counts),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(input.checks),
    ])
}

fn parse_octet_gate_receipt(value: &IoValue) -> Result<ParsedOctetGateReceipt> {
    let fields = value
        .collect_simple_record("octet-gate-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <octet-gate-receipt-v1 ...>"))?;
    require_schema(&fields[0], OCTET_GATE_RECEIPT_SCHEMA, "octet gate receipt")?;
    let metadata = value_to_iovalue(&fields[11]);
    let metadata_fields = metadata
        .collect_simple_record("metadata", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected octet gate metadata record"))?;
    Ok(ParsedOctetGateReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        policy_ref: record_string(&fields[2], "policy")?,
        command_ref: record_optional_string(&fields[3], "command")?,
        status_ref: record_optional_string(&fields[4], "status")?,
        summary_ref: record_optional_string(&fields[5], "summary")?,
        findings_ref: record_optional_string(&fields[6], "findings")?,
        object_corpus_ref: record_optional_string(&fields[7], "object-corpus")?,
        fingerprint_ref: record_optional_string(&fields[8], "fingerprint")?,
        config_hash: record_optional_string(&metadata_fields[0], "config-hash")?,
        profile_hash: record_optional_string(&metadata_fields[1], "profile-hash")?,
        toolchain: record_optional_string(&metadata_fields[2], "toolchain")?,
        counts: parse_counts(&fields[12])?,
        diagnostics: record_string_sequence(&fields[13], "diagnostics")?,
        checks: parse_check_pairs(&fields[14])?,
    })
}

fn parsed_check_pass(parsed: &ParsedOctetGateReceipt, name: &str) -> bool {
    parsed.checks.iter().any(|(check_name, status)| check_name == name && status == "pass")
}

fn parse_counts(value: &Value<IoValue>) -> Result<FindingCounts> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("counts", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected counts record"))?;
    Ok(FindingCounts {
        total: record_u64(&fields[0], "findings")?,
        warnings: record_u64(&fields[1], "warnings")?,
        errors: record_u64(&fields[2], "errors")?,
        autofixable: record_u64(&fields[3], "autofixable")?,
        critical: record_u64(&fields[4], "critical")?,
        uncovered: record_u64(&fields[5], "uncovered")?,
    })
}

fn parse_check_pairs(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected checks record"))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected checks sequence"))?;
    items
        .iter()
        .map(|item| {
            let item = value_to_iovalue(item);
            let fields = item
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected check record"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn record_optional_string(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    optional_string(&fields[0], label)
}

fn optional_string(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let fields = value
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional {label}")))?;
    Ok(Some(required_string(&fields[0], label)?))
}

fn is_content_ref(value: &str) -> bool {
    is_prefixed_blake3_hex_ref(value, BLAKE3_REF_PREFIX) || is_prefixed_blake3_hex_ref(value, B3_REF_PREFIX)
}

fn is_prefixed_blake3_hex_ref(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(is_lowercase_blake3_hex)
}

fn is_lowercase_blake3_hex(value: &str) -> bool {
    value.len() == BLAKE3_HEX_LENGTH && value.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn octet_raw_artifact_value(label: &'static str, schema: &str, name: &str, file: &GateFile) -> IoValue {
    record(label, vec![
        string(schema),
        record("name", vec![string(name)]),
        record("content-ref", vec![string(&file.artifact_ref)]),
        record("content", vec![string(&file.text)]),
        checks_value(&[
            Check {
                name: "raw-content-ref-bound",
                status: "pass",
            },
            Check {
                name: "diagnostic-artifact",
                status: "pass",
            },
        ]),
    ])
}

fn octet_artifact_ledger_receipt_value(
    decision: &str,
    artifacts_dir: &str,
    imported_refs: &[String],
    diagnostics: &[String],
    checks: &[Check],
) -> IoValue {
    record("octet-artifact-ledger-receipt-v1", vec![
        string(crate::preserves_rail::OCTET_ARTIFACT_LEDGER_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("artifacts-dir", vec![string(artifacts_dir)]),
        record("imported", vec![sequence(imported_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(checks),
    ])
}

fn octet_gate_receipt_value(input: OctetGateReceiptInput<'_>) -> IoValue {
    record("octet-gate-receipt-v1", vec![
        string(OCTET_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("policy", vec![string(input.policy_ref)]),
        record("command", vec![optional_ref(input.command_ref)]),
        record("status", vec![optional_ref(input.status_ref)]),
        record("summary", vec![optional_ref(input.summary_ref)]),
        record("findings", vec![optional_ref(input.structured_findings_ref)]),
        record("object-corpus", vec![optional_ref(input.object_corpus_ref)]),
        record("fingerprint", vec![optional_ref(input.fingerprint_evidence_ref)]),
        record("baseline", vec![record("none", Vec::new())]),
        record("review-refs", vec![sequence(Vec::new())]),
        record("metadata", vec![
            record("config-hash", vec![optional_ref(input.config_hash)]),
            record("profile-hash", vec![optional_ref(input.profile_hash)]),
            record("toolchain", vec![optional_ref(input.toolchain)]),
        ]),
        counts_value(input.counts),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(input.checks),
    ])
}

fn read_required_file(
    artifacts_dir: &Path,
    name: &'static str,
    check_name: &'static str,
    checks: &mut impl crate::bounded::VecSink<Check>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Option<GateFile> {
    let path = artifacts_dir.join(name);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            push_check(checks, check_name, false);
            push_diagnostic(diagnostics, format!("missing or unreadable {name}: {error}"));
            return None;
        }
    };
    let text = match String::from_utf8(bytes.clone()) {
        Ok(text) => text,
        Err(error) => {
            push_check(checks, check_name, false);
            push_diagnostic(diagnostics, format!("{name} is not UTF-8 text: {error}"));
            return None;
        }
    };
    push_check(checks, check_name, true);
    Some(GateFile {
        artifact_ref: bytes_ref(&bytes),
        text,
    })
}
