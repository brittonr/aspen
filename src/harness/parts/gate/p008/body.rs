
pub fn check_value(value: &IoValue) -> Result<Check> {
    if value.collect_simple_record("harness-failure-v1", None).is_some() {
        let failure = super::schema::parse_failure(value)?;
        return Err(MoltenError::invalid_harness(format!(
            "harness failure artifact {} phase={} kind={} cannot satisfy pass evidence gate",
            failure.failure_ref, failure.phase, failure.kind
        )));
    }

    if value.collect_simple_record("harness-report-v1", None).is_some() {
        return check_report(value, "report".to_string(), None);
    }

    if value.collect_simple_record("harness-repro-bundle-v1", None).is_some() {
        let bundle = super::schema::parse_repro_bundle(value)?;
        return match bundle.kind {
            super::schema::ReproBundleKind::Report => {
                if let Some(loss_classification) = bundle.loss_classification.as_deref()
                    && loss_classification != "gate-preserving"
                {
                    return Err(MoltenError::invalid_harness(format!(
                        "{} repro bundle {} is {loss_classification} and cannot satisfy pass evidence gates without an explicit gate-preserving policy",
                        bundle.export_profile.as_deref().unwrap_or("profiled"),
                        bundle.bundle_ref
                    )));
                }
                let report_value = bundle
                    .report_value
                    .clone()
                    .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing report value"))?;
                validate_sealed_report_bundle(&report_value, &bundle)?;
                let mut check = check_report(&report_value, "repro-bundle".to_string(), Some(bundle.bundle_ref))?;
                check.redaction_policy_ref = bundle.redaction_policy_ref;
                check.redaction_gate_ref = bundle.redaction_gate_ref;
                Ok(check)
            }
            super::schema::ReproBundleKind::Failure => Err(MoltenError::invalid_harness(format!(
                "failure repro bundle {} wrapping {} cannot satisfy pass evidence gate",
                bundle.bundle_ref, bundle.artifact_ref
            ))),
        };
    }

    Err(MoltenError::invalid_harness(
        "expected harness report or report repro bundle as pass evidence; failure artifacts are diagnostics only",
    ))
}

pub fn sealed_repro_bundle_value_with_command(report_value: &IoValue, command: &[String]) -> Result<IoValue> {
    let report_check = check_report(report_value, "report".to_string(), None)?;
    let report_receipt_value = receipt_value(&report_check);
    super::schema::sealed_repro_bundle_value_with_command_and_receipt(report_value, command, &report_receipt_value)
}
