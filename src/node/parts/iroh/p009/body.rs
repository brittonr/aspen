
fn checks_value(checks: &[(&'static str, &'static str)]) -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        checks
            .iter()
            .map(|(name, status)| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(status),
                ])
            })
            .collect(),
    )])
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}
