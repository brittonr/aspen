//! Checker result binding and bounded report validation for external live reliability evidence.

pub(super) fn validate_checked(expected: &super::Expected, evidence: &super::Observed) -> Result<(), super::Issue> {
    let history_ref =
        crate::live_reliability::checker::canonical_ref(&evidence.history).map_err(|_| super::Issue::History)?;
    let report_ref =
        crate::live_reliability::checker::canonical_ref(&evidence.report).map_err(|_| super::Issue::Checker)?;
    if expected.checked.history_ref != history_ref
        || expected.checked.report_ref != report_ref
        || expected.identities.get("checker") != Some(&expected.checked.checker_ref)
        || expected.checked.report != evidence.report
    {
        return Err(super::Issue::Checker);
    }
    match (
        expected.identities.get("reference-checker"),
        &evidence.reference_report,
        &expected.reference_checked,
    ) {
        (None, None, None) => {}
        (Some(reference), Some(report), Some(checked)) => {
            if reference != &checked.checker_ref
                || checked.checker_ref == expected.checked.checker_ref
                || checked.executable_blake3 == expected.checked.executable_blake3
                || checked.history_ref != history_ref
                || checked.report != *report
                || checked.report_ref
                    != crate::live_reliability::checker::canonical_ref(report).map_err(|_| super::Issue::Checker)?
            {
                return Err(super::Issue::Checker);
            }
        }
        _ => return Err(super::Issue::Checker),
    }
    Ok(())
}

pub(super) fn validate_report<'a>(
    expected: &super::Expected,
    evidence: &'a super::Observed,
) -> Result<&'a str, super::Issue> {
    let report = &evidence.report;
    let verdict = report["verdict"].as_str().ok_or(super::Issue::Checker)?;
    if report["schema_version"] != 1
        || report["model"] != "register"
        || report["checker_id"] != "chaoscontrol-semantic-checker.v1"
        || report["profile_id"] != evidence.history["profile"]["profile_id"]
        || report["bounds"] != evidence.history["profile"]["bounds"]
        || report["completeness"] != evidence.history["completeness"]
        || !report["history_id"].as_str().is_some_and(crate::live_reliability::checker::reference)
        || !matches!(verdict, "valid" | "invalid" | "unknown")
    {
        return Err(super::Issue::Checker);
    }
    let size = serde_json::to_vec(report).map_err(|_| super::Issue::Witness)?.len();
    if size > expected.max_witness_bytes
        || !report["witness"].is_array()
        || (verdict == "invalid") != report["invalid"].is_object()
        || (verdict == "unknown") != report["unknown"].is_object()
        || (verdict != "valid" && report["witness"].as_array().is_some_and(|w| !w.is_empty()))
    {
        return Err(super::Issue::Witness);
    }
    if let Some(reference) = &evidence.reference_report
        && (reference["history_id"] != report["history_id"]
            || reference["model"] != report["model"]
            || reference["bounds"] != report["bounds"]
            || !matches!(reference["verdict"].as_str(), Some("valid" | "invalid" | "unknown"))
            || !expected
                .identities
                .get("reference-checker")
                .is_some_and(|r| crate::live_reliability::checker::reference(r)))
    {
        return Err(super::Issue::Checker);
    }
    Ok(verdict)
}
