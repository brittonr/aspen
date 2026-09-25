
fn validate_sealed_report_bundle(report_value: &IoValue, bundle: &super::schema::ReproBundle) -> Result<()> {
    if bundle.redaction_policy_ref.is_none() || bundle.redaction_gate_ref.is_none() {
        return Err(MoltenError::invalid_harness("sealed report repro bundle missing redaction preflight evidence"));
    }
    let embedded_receipt_value = bundle
        .receipt_value
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("sealed report repro bundle missing embedded gate receipt"))?;
    let embedded_receipt_ref = bundle
        .gate_receipt_ref
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("sealed report repro bundle missing gate receipt ref"))?;
    let receipt = parse_receipt(embedded_receipt_value)?;
    if &receipt.receipt_ref != embedded_receipt_ref {
        return Err(MoltenError::invalid_harness(
            "sealed repro bundle gate receipt ref does not match embedded receipt",
        ));
    }
    if receipt.artifact_kind != "report" {
        return Err(MoltenError::invalid_harness(format!(
            "sealed repro bundle must embed a report gate receipt, got {}",
            receipt.artifact_kind
        )));
    }
    if receipt.artifact_ref != bundle.artifact_ref || receipt.report_ref != bundle.artifact_ref {
        return Err(MoltenError::invalid_harness(
            "sealed repro bundle gate receipt does not bind the embedded report ref",
        ));
    }
    let expected_report_check = check_report(report_value, "report".to_string(), None)?;
    let expected_receipt_value = receipt_value(&expected_report_check);
    let expected_receipt_ref = canonical_hash(&expected_receipt_value)?;
    let actual_receipt_ref = canonical_hash(embedded_receipt_value)?;
    if actual_receipt_ref != expected_receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "sealed repro bundle embedded gate receipt does not match report: receipt hashes to {actual_receipt_ref}, expected {expected_receipt_ref}"
        )));
    }
    Ok(())
}
