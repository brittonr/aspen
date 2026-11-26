//! The chunk-store receipt record: pass and denial constructors and the parser.

pub(in crate::blocks) struct ChunkReceiptValueInput<'a> {
    pub(in crate::blocks) operation: &'a str,
    pub(in crate::blocks) decision: &'a str,
    pub(in crate::blocks) manifest_ref: Option<&'a str>,
    pub(in crate::blocks) chunk_refs: &'a [String],
    pub(in crate::blocks) checks: Vec<(&'a str, &'a str)>,
    pub(in crate::blocks) details: Vec<preserves::IOValue>,
}

pub(in crate::blocks) fn receipt_value(input: ChunkReceiptValueInput<'_>) -> preserves::IOValue {
    crate::preserves_rail::record("chunk-store-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::CHUNK_STORE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("manifest", vec![
            input
                .manifest_ref
                .map(crate::preserves_rail::string)
                .unwrap_or_else(|| crate::preserves_rail::record("none", vec![])),
        ]),
        crate::preserves_rail::record("chunks", vec![crate::preserves_rail::sequence(
            input.chunk_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
            input
                .checks
                .into_iter()
                .map(|(name, status)| {
                    crate::preserves_rail::record("check", vec![
                        crate::preserves_rail::string(name),
                        crate::preserves_rail::string(status),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("details", vec![crate::preserves_rail::sequence(input.details)]),
    ])
}

pub(in crate::blocks) fn denial_receipt_value(
    operation: &str,
    manifest_ref: Option<&str>,
    chunk_refs: &[String],
    reason: impl Into<String>,
    checks: Vec<(&str, &str)>,
) -> preserves::IOValue {
    let reason = reason.into();
    receipt_value(ChunkReceiptValueInput {
        operation,
        decision: "deny",
        manifest_ref,
        chunk_refs,
        checks,
        details: vec![crate::preserves_rail::record("reason", vec![
            crate::preserves_rail::string(&reason),
        ])],
    })
}

pub fn parse_receipt_value(
    value: &preserves::IOValue,
    expected_receipt_ref: Option<&str>,
) -> crate::error::Result<crate::blocks::ChunkReceipt> {
    let fields = crate::blocks::wire::fields::simple_record(value, "chunk-store-receipt-v1", 7)?;
    crate::blocks::wire::fields::require_schema(
        &fields[0],
        crate::preserves_rail::CHUNK_STORE_RECEIPT_SCHEMA,
        "chunk store receipt",
    )?;
    let operation = crate::blocks::wire::fields::record_string(&fields[1], "operation")?;
    let decision = crate::blocks::wire::fields::record_string(&fields[2], "decision")?;
    let manifest_ref = crate::blocks::wire::fields::record_optional_string(&fields[3], "manifest")?;
    let chunk_refs = crate::blocks::wire::fields::record_string_sequence(&fields[4], "chunks")?;
    let check_values = crate::blocks::wire::fields::record_sequence(&fields[5], "checks")?;
    let details = crate::blocks::wire::fields::record_sequence(&fields[6], "details")?;
    if operation.is_empty() {
        return Err(crate::error::Failure::invalid_harness("chunk store receipt operation must not be empty"));
    }
    if decision != "pass" && decision != "deny" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk store receipt decision must be pass or deny, got {decision}"
        )));
    }
    if let Some(manifest_ref) = &manifest_ref {
        crate::blocks::layout::filename_for_ref(manifest_ref)?;
    }
    for chunk_ref in &chunk_refs {
        crate::blocks::layout::filename_for_ref(chunk_ref)?;
    }
    let mut checks = Vec::new();
    for check_value in &check_values {
        let check = crate::blocks::wire::fields::simple_record(check_value, "check", 2)?;
        let name = crate::blocks::wire::fields::required_string(&check[0], "check name")?;
        let status = crate::blocks::wire::fields::required_string(&check[1], "check status")?;
        if name.is_empty() {
            return Err(crate::error::Failure::invalid_harness("chunk store receipt check name must not be empty"));
        }
        if status != "pass" && status != "fail" {
            return Err(crate::error::Failure::invalid_harness(format!(
                "chunk store receipt check status must be pass or fail, got {status}"
            )));
        }
        crate::bounded::push(
            &mut checks,
            crate::blocks::ChunkReceiptCheck { name, status },
            crate::blocks::limits::MAX_CHUNK_STORE_CHECKS,
            "chunk store receipt checks",
        )?;
    }
    let receipt_ref = crate::preserves_rail::canonical_hash(value)?;
    if let Some(expected) = expected_receipt_ref
        && receipt_ref != expected
    {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk store receipt hash mismatch: got {receipt_ref}, expected {expected}"
        )));
    }
    Ok(crate::blocks::ChunkReceipt {
        receipt_ref,
        operation,
        decision,
        manifest_ref,
        chunk_refs,
        checks,
        details,
        value: value.clone(),
    })
}
