type IoValue = preserves::IOValue;

const MAX_STEPS: usize = 128;
const MAX_CHECKS: usize = 32;
const MAX_DIAGNOSTICS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptEvidence {
    LifecycleTransition {
        transition_value: IoValue,
        receipt_value: IoValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub before_state_ref: String,
    pub transition_ref: String,
    pub after_state_ref: String,
    pub check_names: Vec<String>,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt: ReceiptEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run<'a> {
    pub initial_state_ref: &'a str,
    pub final_state_ref: &'a str,
    pub steps: &'a [Step],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validation {
    pub decision: String,
    pub accepted_steps: usize,
    pub final_state_ref: String,
    pub diagnostics: Vec<String>,
    pub summary: String,
}

pub fn validate(trace: &Run<'_>) -> Validation {
    match validate_inner(trace) {
        Ok(accepted_steps) => pass_validation(accepted_steps, trace.final_state_ref),
        Err(failure) => deny_validation(failure.accepted_steps, &failure.final_state_ref, failure.diagnostic),
    }
}

fn validate_inner(trace: &Run<'_>) -> std::result::Result<usize, Failure> {
    if trace.steps.is_empty() {
        return Err(Failure::new(0, trace.initial_state_ref, "proof trace must contain at least one step"));
    }
    if trace.steps.len() > MAX_STEPS {
        return Err(Failure::new(0, trace.initial_state_ref, "proof trace step count exceeds bound"));
    }
    validate_ref(trace.initial_state_ref, "proof trace initial state", 0, trace.initial_state_ref)?;
    validate_ref(trace.final_state_ref, "proof trace final state", 0, trace.initial_state_ref)?;

    let mut expected_before = trace.initial_state_ref;
    for (index, step) in trace.steps.iter().enumerate() {
        if step.before_state_ref != expected_before {
            return Err(Failure::new(
                index,
                expected_before,
                format!(
                    "proof trace step {index} before-state mismatch: got {}, expected {expected_before}",
                    step.before_state_ref
                ),
            ));
        }
        validate_step_contract(index, expected_before, step)?;
        validate_step_receipt(index, expected_before, step)?;
        expected_before = &step.after_state_ref;
    }

    if expected_before != trace.final_state_ref {
        return Err(Failure::new(
            trace.steps.len(),
            expected_before,
            format!("proof trace final-state mismatch: got {expected_before}, expected {}", trace.final_state_ref),
        ));
    }
    Ok(trace.steps.len())
}

fn validate_step_contract(
    index: usize,
    final_state_ref: &str,
    step: &Step,
) -> std::result::Result<(), Failure> {
    validate_ref(&step.before_state_ref, "proof trace before state", index, final_state_ref)?;
    validate_ref(&step.transition_ref, "proof trace transition", index, final_state_ref)?;
    validate_ref(&step.after_state_ref, "proof trace after state", index, final_state_ref)?;
    validate_ref(&step.receipt_ref, "proof trace receipt", index, final_state_ref)?;
    validate_decision(&step.decision, index, final_state_ref)?;
    validate_check_names(&step.check_names, index, final_state_ref)?;
    validate_diagnostics(&step.diagnostics, index, final_state_ref)?;
    Ok(())
}

fn validate_step_receipt(
    index: usize,
    final_state_ref: &str,
    step: &Step,
) -> std::result::Result<(), Failure> {
    match &step.receipt {
        ReceiptEvidence::LifecycleTransition {
            transition_value,
            receipt_value,
        } => {
            let validation =
                crate::lifecycle::validate_transition_receipt(transition_value, receipt_value, Some(&step.receipt_ref))
                    .map_err(|error| {
                        Failure::new(
                            index,
                            final_state_ref,
                            format!("proof trace step {index} receipt invalid: {error}"),
                        )
                    })?;
            if validation.transition_ref != step.transition_ref {
                return Err(Failure::new(
                    index,
                    final_state_ref,
                    format!("proof trace step {index} transition ref does not match receipt"),
                ));
            }
            if validation.decision != step.decision {
                return Err(Failure::new(
                    index,
                    final_state_ref,
                    format!("proof trace step {index} decision does not match receipt"),
                ));
            }
            if validation.diagnostics != step.diagnostics {
                return Err(Failure::new(
                    index,
                    final_state_ref,
                    format!("proof trace step {index} diagnostics do not match receipt"),
                ));
            }
            Ok(())
        }
    }
}
