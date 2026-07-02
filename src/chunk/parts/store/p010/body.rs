
fn lineage_producer() -> Result<crate::evidence_chain::ChainProducer> {
    Ok(crate::evidence_chain::ChainProducer::new(
        "molten-chunk-lineage",
        canonical_hash(&record("chunk-lineage-producer-key", vec![string("molten")]))?,
    ))
}

fn lineage_context_refs(
    manifest: &ChunkManifest,
    receipt: &ChunkStoreReceipt,
) -> Result<Vec<crate::evidence_chain::ChainContextRef>> {
    let mut refs = vec![
        crate::evidence_chain::ChainContextRef::new("manifest", manifest.manifest_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("chunk-root", manifest.root_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("metadata", manifest.metadata_ref.clone()),
        crate::evidence_chain::ChainContextRef::new(
            "operation",
            canonical_hash(&record("chunk-lineage-operation", vec![string(&receipt.operation)]))?,
        ),
    ];
    for chunk in &manifest.chunks {
        push_bounded(
            &mut refs,
            crate::evidence_chain::ChainContextRef::new("chunk", chunk.chunk_ref.clone()),
            MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage context refs",
        )?;
    }
    for detail in &receipt.details {
        collect_detail_context_refs(DetailContextRefsInput {
            value: detail,
            refs: &mut refs,
        })?;
    }
    Ok(refs)
}

struct DetailContextRefsInput<'a> {
    value: &'a IoValue,
    refs: &'a mut Vec<crate::evidence_chain::ChainContextRef>,
}

fn collect_detail_context_refs(input: DetailContextRefsInput<'_>) -> Result<()> {
    let mut pending = Vec::with_capacity(1);
    push_bounded(&mut pending, input.value.clone(), MAX_CHUNK_STORE_CONTEXT_REFS, "chunk lineage detail scan values")?;
    let refs = input.refs;
    while let Some(current) = pending.pop() {
        if let Some(text) = current.as_string() {
            collect_detail_context_refs_push_text(DetailTextInput {
                text: text.into_owned(),
                refs,
            })?;
            continue;
        }
        collect_detail_context_refs_push_children(DetailChildInput {
            value: &current,
            pending: &mut pending,
        })?;
    }
    Ok(())
}

struct DetailTextInput<'a> {
    text: String,
    refs: &'a mut Vec<crate::evidence_chain::ChainContextRef>,
}

fn collect_detail_context_refs_push_text(input: DetailTextInput<'_>) -> Result<()> {
    if validate_content_ref(&input.text).is_ok() {
        push_bounded(
            input.refs,
            crate::evidence_chain::ChainContextRef::new("detail-ref", input.text),
            MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage detail refs",
        )?;
    } else if input.text.starts_with("iroh-local-chunk:") {
        push_bounded(
            input.refs,
            crate::evidence_chain::ChainContextRef::new(
                "ticket",
                canonical_hash(&record("iroh-ticket", vec![string(input.text)]))?,
            ),
            MAX_CHUNK_STORE_CONTEXT_REFS,
            "chunk lineage detail refs",
        )?;
    }
    Ok(())
}

struct DetailChildInput<'a> {
    value: &'a IoValue,
    pending: &'a mut Vec<IoValue>,
}

fn collect_detail_context_refs_push_children(input: DetailChildInput<'_>) -> Result<()> {
    if let Some(sequence) = input.value.collect_sequence() {
        for item in sequence.iter().rev() {
            collect_detail_context_refs_push_child(DetailPushInput {
                values: input.pending,
                value: value_to_iovalue(item),
            })?;
        }
        return Ok(());
    }
    match input.value.value_class() {
        ValueClass::Atomic(_) | ValueClass::Embedded => {}
        ValueClass::Compound(CompoundClass::Record)
        | ValueClass::Compound(CompoundClass::Sequence)
        | ValueClass::Compound(CompoundClass::Set) => {
            let mut children = Vec::new();
            for child in input.value.iter() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: value_to_iovalue(&child),
                })?;
            }
            for child in children.into_iter().rev() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: input.pending,
                    value: child,
                })?;
            }
        }
        ValueClass::Compound(CompoundClass::Dictionary) => {
            let mut children = Vec::new();
            for (key, value) in input.value.entries() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: value_to_iovalue(&key),
                })?;
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: &mut children,
                    value: value_to_iovalue(&value),
                })?;
            }
            for child in children.into_iter().rev() {
                collect_detail_context_refs_push_child(DetailPushInput {
                    values: input.pending,
                    value: child,
                })?;
            }
        }
    }
    Ok(())
}

struct DetailPushInput<'a> {
    values: &'a mut Vec<IoValue>,
    value: IoValue,
}

fn collect_detail_context_refs_push_child(input: DetailPushInput<'_>) -> Result<()> {
    push_bounded(input.values, input.value, MAX_CHUNK_STORE_CONTEXT_REFS, "chunk lineage detail scan values")
}

struct LineageValueInput<'a> {
    manifest_ref: &'a str,
    root_ref: &'a str,
    link_values: &'a [IoValue],
    receipt_values: &'a [IoValue],
    verify_receipt_value: &'a IoValue,
    predicate_values: &'a [IoValue],
}

fn lineage_value(input: &LineageValueInput<'_>) -> IoValue {
    record("chunk-lineage-v1", vec![
        string(CHUNK_LINEAGE_SCHEMA),
        record("manifest", vec![string(input.manifest_ref)]),
        record("root", vec![string(input.root_ref)]),
        record("links", vec![sequence(input.link_values.to_vec())]),
        record("receipts", vec![sequence(input.receipt_values.to_vec())]),
        record("verify-receipt", vec![input.verify_receipt_value.clone()]),
        record("predicates", vec![sequence(input.predicate_values.to_vec())]),
        record("checks", vec![sequence(
            [
                "manifest-root-binding",
                "receipt-payload-binding",
                "lineage-no-global-head",
                "lineage-continuity",
                "lineage-predicate-receipts",
            ]
            .iter()
            .map(|name| record("check", vec![string(*name), string("pass")]))
            .collect(),
        )]),
    ])
}

fn parse_lineage_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let checks = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected <checks ...> field"))?;
    let check_values = checks[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected sequence for lineage checks"))?;
    let mut parsed = Vec::new();
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "lineage check name")?;
        let status = required_string(&check[1], "lineage check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("lineage check {name} status is {status}")));
        }
        push_bounded(&mut parsed, name, MAX_CHUNK_STORE_CHECKS, "chunk lineage checks")?;
    }
    Ok(parsed)
}

fn require_lineage_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("chunk lineage missing {expected} check")))
    }
}

fn lineage_record_value(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    Ok(value_to_iovalue(&record[0]))
}

fn require_lineage_context(
    context_refs: &[crate::evidence_chain::ChainContextRef],
    label: &str,
    expected: &str,
) -> Result<()> {
    if context_refs.iter().any(|context| context.label == label && context.artifact_ref == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("chunk lineage link missing {label} context ref {expected}")))
    }
}

fn require_chunk_lineage_predicate<'a>(
    predicates: &'a [crate::evidence_chain::ChainPredicateReceipt],
    expected_kind: &str,
) -> Result<&'a crate::evidence_chain::ChainPredicateReceipt> {
    predicates
        .iter()
        .find(|predicate| predicate.predicate == expected_kind && predicate.decision == "pass")
        .ok_or_else(|| {
            MoltenError::invalid_harness(format!("chunk lineage missing passing {expected_kind} predicate receipt"))
        })
}

fn validate_chunk_lineage_verify_receipt(
    value: &IoValue,
    chain: &crate::evidence_chain::ChainScope,
    link_refs: &[String],
    receipt_refs: &[String],
    predicate_receipt_refs: &[String],
) -> Result<()> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage missing chain verify receipt"))?;
    let schema = required_string(&receipt[0], "chunk lineage verify schema")?;
    if schema != crate::preserves_rail::EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!("unsupported chunk lineage verify schema {schema}")));
    }
    let decision = record_string(&receipt[1], "decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "chunk lineage verify receipt decision must be pass, got {decision}"
        )));
    }
    let receipt_chain = parse_lineage_chain_scope(&receipt[2])?;
    if &receipt_chain != chain {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt chain scope mismatch"));
    }
    let anchor_ref = record_optional_ref(&receipt[3], "anchor")?
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage verify receipt missing anchor"))?;
    let expected_head = record_optional_ref(&receipt[4], "expected-head")?
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage verify receipt missing expected head"))?;
    if Some(&anchor_ref) != link_refs.first() || Some(&expected_head) != link_refs.last() {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt does not bind lineage anchor/head"));
    }
    if record_string_sequence(&receipt[5], "discovered-heads")? != vec![expected_head] {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt discovered head mismatch"));
    }
    if record_string_sequence(&receipt[6], "verified-links")? != link_refs {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt links mismatch"));
    }
    if record_string_sequence(&receipt[7], "payloads")? != receipt_refs {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt payload refs mismatch"));
    }
    if record_string_sequence(&receipt[8], "predicates")? != predicate_receipt_refs {
        return Err(MoltenError::invalid_harness("chunk lineage verify receipt predicate refs mismatch"));
    }
    Ok(())
}
