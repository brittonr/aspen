//! The lineage record codec and the check, context, predicate, and verify-receipt bindings it
//! requires.

pub(super) struct LineageValueInput<'a> {
    pub(super) manifest_ref: &'a str,
    pub(super) root_ref: &'a str,
    pub(super) link_values: &'a [preserves::IOValue],
    pub(super) receipt_values: &'a [preserves::IOValue],
    pub(super) verify_receipt_value: &'a preserves::IOValue,
    pub(super) predicate_values: &'a [preserves::IOValue],
}

pub(super) fn lineage_value(input: &LineageValueInput<'_>) -> preserves::IOValue {
    crate::preserves_rail::record("chunk-lineage-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::CHUNK_LINEAGE_SCHEMA),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(input.manifest_ref)]),
        crate::preserves_rail::record("root", vec![crate::preserves_rail::string(input.root_ref)]),
        crate::preserves_rail::record("links", vec![crate::preserves_rail::sequence(input.link_values.to_vec())]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(input.receipt_values.to_vec())]),
        crate::preserves_rail::record("verify-receipt", vec![input.verify_receipt_value.clone()]),
        crate::preserves_rail::record("predicates", vec![crate::preserves_rail::sequence(
            input.predicate_values.to_vec(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
            [
                "manifest-root-binding",
                "receipt-payload-binding",
                "lineage-no-global-head",
                "lineage-continuity",
                "lineage-predicate-receipts",
            ]
            .iter()
            .map(|name| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(*name),
                    crate::preserves_rail::string("pass"),
                ])
            })
            .collect(),
        )]),
    ])
}

pub(super) fn parse_lineage_checks(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<String>> {
    let checks = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <checks ...> field"))?;
    let check_values = checks[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected sequence for lineage checks"))?;
    let mut parsed = Vec::new();
    for check_value in check_values.iter() {
        let check_value = crate::preserves_rail::value_to_iovalue(check_value);
        let check = crate::blocks::wire::fields::simple_record(&check_value, "check", 2)?;
        let name = crate::blocks::wire::fields::required_string(&check[0], "lineage check name")?;
        let status = crate::blocks::wire::fields::required_string(&check[1], "lineage check status")?;
        if status != "pass" {
            return Err(crate::error::Failure::invalid_harness(format!("lineage check {name} status is {status}")));
        }
        crate::bounded::push(&mut parsed, name, crate::blocks::limits::MAX_CHUNK_STORE_CHECKS, "chunk lineage checks")?;
    }
    Ok(parsed)
}

pub(super) fn require_lineage_check(checks: &[String], expected: &str) -> crate::error::Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("chunk lineage missing {expected} check")))
    }
}

pub(super) fn lineage_record_value(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<preserves::IOValue> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("expected <{label} ...> field")))?;
    Ok(crate::preserves_rail::value_to_iovalue(&record[0]))
}

pub(super) fn require_lineage_context(
    context_refs: &[crate::evidence_chain::ChainContextRef],
    label: &str,
    expected: &str,
) -> crate::error::Result<()> {
    if context_refs.iter().any(|context| context.label == label && context.artifact_ref == expected) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!(
            "chunk lineage link missing {label} context ref {expected}"
        )))
    }
}

pub(super) fn require_chunk_lineage_predicate<'a>(
    predicates: &'a [crate::evidence_chain::ChainPredicateReceipt],
    expected_kind: &str,
) -> crate::error::Result<&'a crate::evidence_chain::ChainPredicateReceipt> {
    predicates
        .iter()
        .find(|predicate| predicate.predicate == expected_kind && predicate.decision == "pass")
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness(format!(
                "chunk lineage missing passing {expected_kind} predicate receipt"
            ))
        })
}

pub(super) fn validate_chunk_lineage_verify_receipt(
    value: &preserves::IOValue,
    chain: &crate::evidence_chain::ChainScope,
    link_refs: &[String],
    receipt_refs: &[String],
    predicate_receipt_refs: &[String],
) -> crate::error::Result<()> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| crate::error::Failure::invalid_harness("chunk lineage missing chain verify receipt"))?;
    let schema = crate::blocks::wire::fields::required_string(&receipt[0], "chunk lineage verify schema")?;
    if schema != crate::preserves_rail::EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported chunk lineage verify schema {schema}"
        )));
    }
    let decision = crate::blocks::wire::fields::record_string(&receipt[1], "decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "chunk lineage verify receipt decision must be pass, got {decision}"
        )));
    }
    let receipt_chain = parse_lineage_chain_scope(&receipt[2])?;
    if &receipt_chain != chain {
        return Err(crate::error::Failure::invalid_harness("chunk lineage verify receipt chain scope mismatch"));
    }
    let anchor_ref = crate::blocks::wire::fields::record_optional_ref(&receipt[3], "anchor")?
        .ok_or_else(|| crate::error::Failure::invalid_harness("chunk lineage verify receipt missing anchor"))?;
    let expected_head = crate::blocks::wire::fields::record_optional_ref(&receipt[4], "expected-head")?
        .ok_or_else(|| crate::error::Failure::invalid_harness("chunk lineage verify receipt missing expected head"))?;
    if Some(&anchor_ref) != link_refs.first() || Some(&expected_head) != link_refs.last() {
        return Err(crate::error::Failure::invalid_harness(
            "chunk lineage verify receipt does not bind lineage anchor/head",
        ));
    }
    if crate::blocks::wire::fields::record_string_sequence(&receipt[5], "discovered-heads")? != vec![expected_head] {
        return Err(crate::error::Failure::invalid_harness("chunk lineage verify receipt discovered head mismatch"));
    }
    if crate::blocks::wire::fields::record_string_sequence(&receipt[6], "verified-links")? != link_refs {
        return Err(crate::error::Failure::invalid_harness("chunk lineage verify receipt links mismatch"));
    }
    if crate::blocks::wire::fields::record_string_sequence(&receipt[7], "payloads")? != receipt_refs {
        return Err(crate::error::Failure::invalid_harness("chunk lineage verify receipt payload refs mismatch"));
    }
    if crate::blocks::wire::fields::record_string_sequence(&receipt[8], "predicates")? != predicate_receipt_refs {
        return Err(crate::error::Failure::invalid_harness("chunk lineage verify receipt predicate refs mismatch"));
    }
    Ok(())
}

fn parse_lineage_chain_scope(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<crate::evidence_chain::ChainScope> {
    let chain = value
        .collect_simple_record("chain", Some(3))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected chain scope field"))?;
    Ok(crate::evidence_chain::ChainScope::new(
        crate::blocks::wire::fields::record_string(&chain[0], "scope")?,
        crate::blocks::wire::fields::record_string(&chain[1], "id")?,
        crate::blocks::wire::fields::record_string(&chain[2], "epoch")?,
    ))
}
