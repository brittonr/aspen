
fn parse_inventory_payload(value: &IoValue) -> Result<(String, Vec<Resource>, Vec<Delegate>)> {
    let fields = value
        .collect_simple_record("federation-inventory-payload", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation inventory payload"))?;
    let peer = record_string(&fields[0], "peer")?;
    let resources_field = value_to_iovalue(&fields[1]);
    let resources_record = resources_field
        .collect_simple_record("resources", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation inventory resources"))?;
    let resource_values = resources_record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("federation inventory resources must be a sequence"))?;
    let resources = resource_values
        .iter()
        .map(|resource| parse_resource(&value_to_iovalue(resource)))
        .collect::<Result<Vec<_>>>()?;
    let delegates_field = value_to_iovalue(&fields[2]);
    let delegates_record = delegates_field
        .collect_simple_record("delegates", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation inventory delegates"))?;
    let delegate_values = delegates_record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("federation inventory delegates must be a sequence"))?;
    let delegates = delegate_values
        .iter()
        .map(|delegate| parse_delegate_unverified(&value_to_iovalue(delegate)))
        .collect::<Result<Vec<_>>>()?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "inventory-does-not-import")?;
    Ok((peer, resources, delegates))
}

fn parse_resource(value: &IoValue) -> Result<Resource> {
    let fields = value
        .collect_simple_record("federated-resource", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected federated resource"))?;
    let resource = Resource::new(
        record_string(&fields[0], "type")?,
        record_string(&fields[1], "ref")?,
        record_string(&fields[2], "schema")?,
        record_string(&fields[3], "transport")?,
        record_string(&fields[4], "source-peer")?,
    );
    validate_resource(&resource)?;
    Ok(resource)
}

fn signature_record(payload: &IoValue, signer: &str, purpose: &str, trust_root: &str, key: &str) -> Result<IoValue> {
    if signer.trim().is_empty() {
        return Err(MoltenError::invalid_harness("federation signer must not be empty"));
    }
    if trust_root.trim().is_empty() {
        return Err(MoltenError::invalid_harness("federation trust root must not be empty"));
    }
    Ok(record("signature", vec![
        record("signer", vec![string(signer)]),
        record("purpose", vec![string(purpose)]),
        record("trust-root", vec![string(trust_root)]),
        record("algorithm", vec![string(SIGNATURE_ALGORITHM)]),
        record("value", vec![string(&signature_for(payload, signer, purpose, trust_root, key)?)]),
    ]))
}

fn verify_signature_record(
    value: &Value<IoValue>,
    payload: &IoValue,
    purpose: &str,
    trust_root: &str,
    key: &str,
) -> Result<(String, String)> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("signature", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation signature"))?;
    let signer = record_string(&fields[0], "signer")?;
    let actual_purpose = record_string(&fields[1], "purpose")?;
    if actual_purpose != purpose {
        return Err(MoltenError::invalid_harness(format!(
            "federation signature purpose {actual_purpose} does not match {purpose}"
        )));
    }
    let actual_trust_root = record_string(&fields[2], "trust-root")?;
    if actual_trust_root != trust_root {
        return Err(MoltenError::invalid_harness(format!(
            "federation signature trust root {actual_trust_root} does not match {trust_root}"
        )));
    }
    let algorithm = record_string(&fields[3], "algorithm")?;
    if algorithm != SIGNATURE_ALGORITHM {
        return Err(MoltenError::invalid_harness(format!("unsupported federation signature algorithm {algorithm}")));
    }
    let signature = record_string(&fields[4], "value")?;
    let expected = signature_for(payload, &signer, purpose, &actual_trust_root, key)?;
    if signature != expected {
        return Err(MoltenError::invalid_harness("federation signature verification failed"));
    }
    Ok((signer, actual_trust_root))
}

fn signature_for(payload: &IoValue, signer: &str, purpose: &str, trust_root: &str, key: &str) -> Result<String> {
    let mut material = canonical_bytes(payload)?;
    material.extend_from_slice(signer.as_bytes());
    material.push(0);
    material.extend_from_slice(purpose.as_bytes());
    material.push(0);
    material.extend_from_slice(trust_root.as_bytes());
    material.push(0);
    material.extend_from_slice(key.as_bytes());
    Ok(content_ref_from_bytes(&material))
}

fn validate_resource(resource: &Resource) -> Result<()> {
    if resource.resource_type.trim().is_empty()
        || resource.schema.trim().is_empty()
        || resource.transport.trim().is_empty()
        || resource.source_peer.trim().is_empty()
    {
        return Err(MoltenError::invalid_harness("federated resource fields must not be empty"));
    }
    require_ref(&resource.resource_ref, "federated resource ref")
}

fn validate_peer(peer: &str) -> Result<()> {
    if peer.trim().is_empty() {
        Err(MoltenError::invalid_harness("federation peer must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn parse_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation checks"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("federation checks must be a sequence"))?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected federation check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("federation evidence missing passing {name} check")))
    }
}

fn record_value(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&record[0]))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&record[0], label)
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/federation/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/federation/parts/mod/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/federation/parts/mod/tests/m000/p002/body.rs"));
}
