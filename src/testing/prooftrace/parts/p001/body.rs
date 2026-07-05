fn validate_ref(
    reference: &str,
    label: &str,
    accepted_steps: usize,
    final_state_ref: &str,
) -> std::result::Result<(), Failure> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        Failure::new(accepted_steps, final_state_ref, format!("{label} ref is not canonical: {error}"))
    })
}

fn validate_decision(
    decision: &str,
    accepted_steps: usize,
    final_state_ref: &str,
) -> std::result::Result<(), Failure> {
    if decision == "pass" || decision == "deny" {
        Ok(())
    } else {
        Err(Failure::new(
            accepted_steps,
            final_state_ref,
            format!("proof trace decision must be pass or deny, got {decision}"),
        ))
    }
}

fn validate_check_names(
    check_names: &[String],
    accepted_steps: usize,
    final_state_ref: &str,
) -> std::result::Result<(), Failure> {
    if check_names.is_empty() {
        return Err(Failure::new(accepted_steps, final_state_ref, "proof trace checks must not be empty"));
    }
    if check_names.len() > MAX_CHECKS {
        return Err(Failure::new(accepted_steps, final_state_ref, "proof trace checks exceed bound"));
    }
    let mut prior: Option<&str> = None;
    for check_name in check_names {
        if check_name.trim().is_empty() {
            return Err(Failure::new(
                accepted_steps,
                final_state_ref,
                "proof trace check names must be non-empty",
            ));
        }
        if let Some(prior_check) = prior
            && prior_check >= check_name.as_str()
        {
            return Err(Failure::new(
                accepted_steps,
                final_state_ref,
                "proof trace check names must be sorted and unique",
            ));
        }
        prior = Some(check_name);
    }
    Ok(())
}

fn validate_diagnostics(
    diagnostics: &[String],
    accepted_steps: usize,
    final_state_ref: &str,
) -> std::result::Result<(), Failure> {
    if diagnostics.len() > MAX_DIAGNOSTICS {
        return Err(Failure::new(accepted_steps, final_state_ref, "proof trace diagnostics exceed bound"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Failure {
    accepted_steps: usize,
    final_state_ref: String,
    diagnostic: String,
}

impl Failure {
    fn new(accepted_steps: usize, final_state_ref: impl AsRef<str>, diagnostic: impl Into<String>) -> Self {
        Self {
            accepted_steps,
            final_state_ref: final_state_ref.as_ref().to_owned(),
            diagnostic: diagnostic.into(),
        }
    }
}

fn pass_validation(accepted_steps: usize, final_state_ref: &str) -> Validation {
    let diagnostics = Vec::new();
    let decision = "pass".to_owned();
    Validation {
        summary: render_summary(&decision, accepted_steps, final_state_ref, &diagnostics),
        decision,
        accepted_steps,
        final_state_ref: final_state_ref.to_owned(),
        diagnostics,
    }
}

fn deny_validation(accepted_steps: usize, final_state_ref: &str, diagnostic: String) -> Validation {
    let diagnostics = vec![diagnostic];
    let decision = "deny".to_owned();
    Validation {
        summary: render_summary(&decision, accepted_steps, final_state_ref, &diagnostics),
        decision,
        accepted_steps,
        final_state_ref: final_state_ref.to_owned(),
        diagnostics,
    }
}

fn render_summary(
    decision: &str,
    accepted_steps: usize,
    final_state_ref: &str,
    diagnostics: &[String],
) -> String {
    format!(
        "state-machine-proof-trace decision={decision} accepted_steps={accepted_steps} final_state_ref={final_state_ref} diagnostics={}",
        diagnostics.join("|")
    )
}
