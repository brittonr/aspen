type Error = molten::error::MoltenError;
type Outcome<T> = molten::error::Result<T>;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Source = molten::harness::ReproBundle;
type Value = preserves::IOValue;

pub(crate) fn run(
    bundle_value: &Value,
    out: &Path,
    reveal_receipt_values: &[Value],
    failure_out: Option<&PathBuf>,
) -> Outcome<()> {
    let source = parse(bundle_value, failure_out)?;
    validate(&source, reveal_receipt_values, bundle_value, failure_out)?;
    let verify_receipt = verify_receipt(&source, bundle_value, failure_out)?;
    let report_value = report_value(&source, bundle_value, failure_out)?;
    let suite_value = suite_value(report_value, bundle_value, failure_out)?;
    let export = write(Write {
        bundle_value,
        source: &source,
        report_value,
        suite_value: &suite_value,
        verify_receipt: verify_receipt.as_ref(),
        reveal_receipt_values,
        out,
    });
    if let Err(error) = export {
        failure(failure_out, &error, bundle_value)?;
        return Err(error);
    }
    println!("repro bundle unpacked to {}", out.display());
    Ok(())
}

fn parse(bundle_value: &Value, failure_out: Option<&PathBuf>) -> Outcome<Source> {
    match molten::harness::parse_repro_bundle(bundle_value) {
        Ok(source) => Ok(source),
        Err(error) => {
            failure(failure_out, &error, bundle_value)?;
            Err(error)
        }
    }
}

fn validate(
    source: &Source,
    reveal_receipt_values: &[Value],
    bundle_value: &Value,
    failure_out: Option<&PathBuf>,
) -> Outcome<()> {
    if source.loss_classification.as_deref() == Some("requires-reveal") {
        return validate_required_reveals(source, reveal_receipt_values, bundle_value, failure_out);
    }
    if reveal_receipt_values.is_empty() {
        return Ok(());
    }
    let error = invalid("reveal receipts are only accepted for encrypted-private repro bundles");
    failure(failure_out, &error, bundle_value)?;
    Err(error)
}

fn validate_required_reveals(
    source: &Source,
    reveal_receipt_values: &[Value],
    bundle_value: &Value,
    failure_out: Option<&PathBuf>,
) -> Outcome<()> {
    if let Err(error) = check_receipts(&source.encrypted_refs, reveal_receipt_values) {
        failure(failure_out, &error, bundle_value)?;
        return Err(error);
    }
    Ok(())
}

fn verify_receipt(source: &Source, bundle_value: &Value, failure_out: Option<&PathBuf>) -> Outcome<Option<Value>> {
    if source.loss_classification.as_deref().unwrap_or("gate-preserving") != "gate-preserving" {
        return Ok(None);
    }
    match molten::harness::repro_verify_receipt_value(bundle_value) {
        Ok(receipt) => Ok(Some(receipt)),
        Err(error) => {
            failure(failure_out, &error, bundle_value)?;
            Err(error)
        }
    }
}

fn report_value<'a>(source: &'a Source, bundle_value: &Value, failure_out: Option<&PathBuf>) -> Outcome<&'a Value> {
    match source.report_value.as_ref() {
        Some(report_value) => Ok(report_value),
        None => {
            let error = invalid("repro unpack requires an embedded report");
            failure(failure_out, &error, bundle_value)?;
            Err(error)
        }
    }
}

fn suite_value(report_value: &Value, bundle_value: &Value, failure_out: Option<&PathBuf>) -> Outcome<Value> {
    match molten::harness::report_suite_value(report_value) {
        Ok(suite_value) => Ok(suite_value),
        Err(error) => {
            failure(failure_out, &error, bundle_value)?;
            Err(error)
        }
    }
}

struct Write<'a> {
    bundle_value: &'a Value,
    source: &'a Source,
    report_value: &'a Value,
    suite_value: &'a Value,
    verify_receipt: Option<&'a Value>,
    reveal_receipt_values: &'a [Value],
    out: &'a Path,
}

fn write(input: Write<'_>) -> Outcome<()> {
    let Write {
        bundle_value,
        source,
        report_value,
        suite_value,
        verify_receipt,
        reveal_receipt_values,
        out,
    } = input;
    let mut payloads = vec![
        super::materialization_payload("refs.preserves", molten::preserves_rail::to_text(bundle_value)?),
        super::materialization_payload("report.preserves", molten::preserves_rail::to_text(report_value)?),
        super::materialization_payload("suite.preserves", molten::preserves_rail::to_text(suite_value)?),
        super::materialization_payload("summary.txt", molten::harness::repro_bundle_summary(bundle_value)?),
        super::materialization_payload("commands.txt", super::REPORT_COMMANDS),
    ];
    super::push_optional_payload(&mut payloads, "gate-receipt.preserves", source.receipt_value.as_ref())?;
    super::push_optional_payload(&mut payloads, "verify-receipt.preserves", verify_receipt)?;
    super::push_optional_payload(
        &mut payloads,
        "redaction-transform-receipt.preserves",
        source.redaction_transform_receipt_value.as_ref(),
    )?;
    super::push_optional_payload(
        &mut payloads,
        "redaction-transform-manifest.preserves",
        source.redaction_transform_manifest_value.as_ref(),
    )?;
    for (index, receipt) in reveal_receipt_values.iter().enumerate() {
        payloads.push(super::materialization_payload(
            &format!("reveal-receipt-{index}.preserves"),
            molten::preserves_rail::to_text(receipt)?,
        ));
    }
    super::materialize_repro_payloads(out, "repro-unpack-v1", &payloads)
}

fn check_receipts(encrypted_refs: &[String], receipt_values: &[Value]) -> Outcome<()> {
    if encrypted_refs.is_empty() {
        return Err(invalid("encrypted-private repro bundle has no encrypted refs to reveal"));
    }
    if receipt_values.is_empty() {
        return Err(invalid("encrypted-private repro unpack requires at least one passing reveal receipt"));
    }
    let expected_refs = encrypted_refs.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let mut authorized_refs = std::collections::BTreeSet::new();
    for receipt_value in receipt_values {
        let receipt = molten::secrets::parse_reveal_receipt(receipt_value)?;
        if receipt.decision != "pass" {
            return Err(invalid("unauthorized reveal receipt cannot unpack private repro material"));
        }
        let encrypted_ref = receipt
            .encrypted_ref
            .as_ref()
            .ok_or_else(|| invalid("reveal receipt does not bind an encrypted repro reference"))?;
        if !expected_refs.contains(encrypted_ref) {
            return Err(invalid("reveal receipt encrypted ref is not part of this repro bundle"));
        }
        authorized_refs.insert(encrypted_ref.clone());
    }
    for encrypted_ref in encrypted_refs {
        if !authorized_refs.contains(encrypted_ref) {
            return Err(invalid("reveal receipts do not authorize every encrypted repro reference"));
        }
    }
    Ok(())
}

fn invalid(message: &str) -> Error {
    Error::invalid_harness(message)
}

fn failure(failure_out: Option<&PathBuf>, error: &Error, value: &Value) -> Outcome<()> {
    super::super::io::write_optional_artifact_failure(failure_out, "unpack", error, value)
}
