
fn parse_chain_scope(value: &Value<IoValue>) -> Result<crate::evidence_chain::ChainScope> {
    let value = value_to_iovalue(value);
    let chain = value
        .collect_simple_record("chain", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain scope record"))?;
    Ok(crate::evidence_chain::ChainScope::new(
        record_string(&chain[0], "scope")?,
        record_string(&chain[1], "id")?,
        record_string(&chain[2], "epoch")?,
    ))
}

fn chain_scope_value(chain: &crate::evidence_chain::ChainScope) -> IoValue {
    record("chain", vec![
        record("scope", vec![string(&chain.scope)]),
        record("id", vec![string(&chain.id)]),
        record("epoch", vec![string(&chain.epoch)]),
    ])
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_field(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let optional = value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_ref(&some[0], label).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected <none> or <some ref> for {label}")))
    }
}

fn parse_ref_sequence_field(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    values.iter().map(|value| required_ref(value, label)).collect()
}

fn parse_check_names(value: &Value<IoValue>) -> Result<Vec<String>> {
    let record = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected <checks ...> field"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected sequence for checks"))?;
    values
        .iter()
        .map(|value| {
            let value = value_to_iovalue(value);
            let check = value
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected check record"))?;
            let name = required_string(&check[0], "check name")?;
            let status = required_string(&check[1], "check status")?;
            if status != "pass" {
                return Err(MoltenError::invalid_harness(format!("check {name} status is {status}")));
            }
            Ok(name)
        })
        .collect()
}

fn require_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("chain bundle missing {expected} check")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> field")))?;
    required_string(&record[0], label)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    validate_content_ref(&reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })?;
    Ok(reference)
}

fn receipt_value(input: &ReceiptValueInput<'_>) -> IoValue {
    let mut refs = vec![record("artifact-ref", vec![string("bundle"), string(input.bundle_ref)])];
    if let Some(verify_ref) = input.verify_ref {
        refs.push(record("artifact-ref", vec![string("verify-receipt"), string(verify_ref)]));
    }
    record("iroh-repro-exchange-receipt-v1", vec![
        string(REPRO_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("node", vec![string(input.node)]),
        record("peer", vec![string(input.peer)]),
        record("ticket", vec![string(input.ticket)]),
        record("artifact-refs", vec![sequence(refs)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("content-addressed-bundle"), string("pass")]),
            record("check", vec![string("sealed-repro-verified"), string("pass")]),
            record("check", vec![string("transport-does-not-grant-trust"), string("pass")]),
        ])]),
    ])
}

fn blob_path(root: &Path, bundle_ref: &str) -> Result<std::path::PathBuf> {
    let hex = content_ref_hex(bundle_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("unsupported Iroh bundle ref {bundle_ref}: {error}")))?;
    Ok(root.join("blobs").join(format!("blake3_{hex}.bin")))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/iroh/parts/exchange/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/iroh/parts/exchange/tests/m000/p001/body.rs"));
}
