
fn view_result(registry_root: &Path, ledger_root: Option<&Path>, request: &Request) -> Result<CoreResult> {
    let reference = required_arg_string(&request.args, "reference")?;
    let should_include_payload = arg_bool(&request.args, "payload", false)?;
    let should_redact_payload = arg_bool(&request.args, "redacted", true)?;
    crate::catalog::view(registry_root, ledger_root, &crate::catalog::ViewInput {
        reference,
        include_payload: should_include_payload,
        redacted: should_redact_payload,
        visibility: request.visibility.clone(),
    })
    .map(CoreResult::Query)
}
