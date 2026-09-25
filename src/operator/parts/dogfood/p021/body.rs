
fn file_ref_mismatch_diagnostics(expected: &[(String, String)], observed: &[(String, String)]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for diagnostic in duplicate_file_ref_diagnostics(expected, "evidence")? {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "Nix dogfood verify diagnostics",
        )?;
    }
    for diagnostic in duplicate_file_ref_diagnostics(observed, "observed output")? {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "Nix dogfood verify diagnostics",
        )?;
    }
    if expected.len() != observed.len() {
        diagnostics.push_limited_value(
            format!("file ref count mismatch: evidence={} observed={}", expected.len(), observed.len()),
            MAX_OPERATOR_DIAGNOSTICS,
            "Nix dogfood verify diagnostics",
        )?;
    }
    for (expected_name, expected_ref) in expected {
        match observed.iter().find(|(observed_name, _)| observed_name == expected_name) {
            Some((_, observed_ref)) => {
                if let Some(diagnostic) = mismatch_diagnostic(expected_name, expected_ref, observed_ref) {
                    diagnostics.push_limited_value(
                        diagnostic,
                        MAX_OPERATOR_DIAGNOSTICS,
                        "Nix dogfood verify diagnostics",
                    )?;
                }
            }
            None => diagnostics.push_limited_value(
                format!("file ref missing from observed output: {expected_name}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?,
        }
    }
    for (observed_name, _) in observed {
        if !expected.iter().any(|(expected_name, _)| expected_name == observed_name) {
            diagnostics.push_limited_value(
                format!("unexpected observed file ref: {observed_name}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?;
        }
    }
    Ok(diagnostics)
}

fn duplicate_file_ref_diagnostics(refs: &[(String, String)], label: &str) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let mut seen_names = Vec::new();
    for (name, _) in refs {
        if seen_names.iter().any(|seen_name: &String| seen_name == name) {
            diagnostics.push_limited_value(
                format!("duplicate file ref path in {label}: {name}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "duplicate file ref diagnostics",
            )?;
        } else {
            seen_names.push_limited_value(name.clone(), MAX_OPERATOR_REFS, "duplicate file ref names")?;
        }
    }
    Ok(diagnostics)
}
