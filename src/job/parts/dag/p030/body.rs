
fn check_admission_readiness(
    admission: &JobAdmissionReceipt,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    checks: &mut impl crate::bounded::VecSink<(&'static str, &'static str)>,
) {
    let required_admission_checks = [
        "target-closure-present",
        "trellis-topology",
        "executable-artifact-gate",
        "capability-authority-context",
        "resource-profile",
        "sync-evidence-bound",
        "strict-octet-source-gate-bound",
        "no-execution",
    ];
    let has_required_admission_checks = required_admission_checks
        .iter()
        .all(|required| admission.checks.iter().any(|check| check == *required));
    push_check(checks, "admission-checkset", has_required_admission_checks);
    if !has_required_admission_checks {
        diagnostics.push_item("job execution admission receipt is missing required target-side checks".to_string());
    }

    let has_authority_receipts = !admission.authority_receipt_refs.is_empty();
    push_check(checks, "authority-receipt-binding", has_authority_receipts);
    if !has_authority_receipts {
        diagnostics.push_item("job execution admission has no authority receipt refs".to_string());
    }

    let has_resource_profile = admission.resource_verdict == "pass";
    push_check(checks, "resource-profile-binding", has_resource_profile);
    if !has_resource_profile {
        diagnostics.push_item(format!("job execution resource verdict is {}", admission.resource_verdict));
    }
}

struct SelectionInput<'a> {
    target_registry: &'a FilePath,
    request: &'a JobExecutionRequest,
    admission: &'a JobAdmissionReceipt,
    dag: &'a JobDag,
}
