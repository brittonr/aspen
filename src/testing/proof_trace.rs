type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;

const MAX_PROOF_TRACE_STEPS: usize = 128;
const MAX_PROOF_TRACE_CHECKS: usize = 32;
const MAX_PROOF_TRACE_DIAGNOSTICS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofReceiptEvidence {
    LifecycleTransition {
        transition_value: IoValue,
        receipt_value: IoValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofTraceStep {
    pub before_state_ref: String,
    pub transition_ref: String,
    pub after_state_ref: String,
    pub check_names: Vec<String>,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub receipt: ProofReceiptEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofTrace<'a> {
    pub initial_state_ref: &'a str,
    pub final_state_ref: &'a str,
    pub steps: &'a [ProofTraceStep],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofTraceValidation {
    pub decision: String,
    pub accepted_steps: usize,
    pub final_state_ref: String,
    pub diagnostics: Vec<String>,
    pub summary: String,
}

pub fn validate_proof_trace(trace: &ProofTrace<'_>) -> ProofTraceValidation {
    match validate_proof_trace_inner(trace) {
        Ok(accepted_steps) => pass_validation(accepted_steps, trace.final_state_ref),
        Err(failure) => deny_validation(failure.accepted_steps, &failure.final_state_ref, failure.diagnostic),
    }
}

fn validate_proof_trace_inner(trace: &ProofTrace<'_>) -> std::result::Result<usize, ProofTraceFailure> {
    if trace.steps.is_empty() {
        return Err(ProofTraceFailure::new(0, trace.initial_state_ref, "proof trace must contain at least one step"));
    }
    if trace.steps.len() > MAX_PROOF_TRACE_STEPS {
        return Err(ProofTraceFailure::new(0, trace.initial_state_ref, "proof trace step count exceeds bound"));
    }
    validate_ref(trace.initial_state_ref, "proof trace initial state", 0, trace.initial_state_ref)?;
    validate_ref(trace.final_state_ref, "proof trace final state", 0, trace.initial_state_ref)?;

    let mut expected_before = trace.initial_state_ref;
    for (index, step) in trace.steps.iter().enumerate() {
        if step.before_state_ref != expected_before {
            return Err(ProofTraceFailure::new(
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
        return Err(ProofTraceFailure::new(
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
    step: &ProofTraceStep,
) -> std::result::Result<(), ProofTraceFailure> {
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
    step: &ProofTraceStep,
) -> std::result::Result<(), ProofTraceFailure> {
    match &step.receipt {
        ProofReceiptEvidence::LifecycleTransition {
            transition_value,
            receipt_value,
        } => {
            let validation =
                crate::lifecycle::validate_transition_receipt(transition_value, receipt_value, Some(&step.receipt_ref))
                    .map_err(|error| {
                        ProofTraceFailure::new(
                            index,
                            final_state_ref,
                            format!("proof trace step {index} receipt invalid: {error}"),
                        )
                    })?;
            if validation.transition_ref != step.transition_ref {
                return Err(ProofTraceFailure::new(
                    index,
                    final_state_ref,
                    format!("proof trace step {index} transition ref does not match receipt"),
                ));
            }
            if validation.decision != step.decision {
                return Err(ProofTraceFailure::new(
                    index,
                    final_state_ref,
                    format!("proof trace step {index} decision does not match receipt"),
                ));
            }
            if validation.diagnostics != step.diagnostics {
                return Err(ProofTraceFailure::new(
                    index,
                    final_state_ref,
                    format!("proof trace step {index} diagnostics do not match receipt"),
                ));
            }
            Ok(())
        }
    }
}

fn validate_ref(
    reference: &str,
    label: &str,
    accepted_steps: usize,
    final_state_ref: &str,
) -> std::result::Result<(), ProofTraceFailure> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        ProofTraceFailure::new(accepted_steps, final_state_ref, format!("{label} ref is not canonical: {error}"))
    })
}

fn validate_decision(
    decision: &str,
    accepted_steps: usize,
    final_state_ref: &str,
) -> std::result::Result<(), ProofTraceFailure> {
    if decision == "pass" || decision == "deny" {
        Ok(())
    } else {
        Err(ProofTraceFailure::new(
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
) -> std::result::Result<(), ProofTraceFailure> {
    if check_names.is_empty() {
        return Err(ProofTraceFailure::new(accepted_steps, final_state_ref, "proof trace checks must not be empty"));
    }
    if check_names.len() > MAX_PROOF_TRACE_CHECKS {
        return Err(ProofTraceFailure::new(accepted_steps, final_state_ref, "proof trace checks exceed bound"));
    }
    let mut prior: Option<&str> = None;
    for check_name in check_names {
        if check_name.trim().is_empty() {
            return Err(ProofTraceFailure::new(
                accepted_steps,
                final_state_ref,
                "proof trace check names must be non-empty",
            ));
        }
        if let Some(prior_check) = prior
            && prior_check >= check_name.as_str()
        {
            return Err(ProofTraceFailure::new(
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
) -> std::result::Result<(), ProofTraceFailure> {
    if diagnostics.len() > MAX_PROOF_TRACE_DIAGNOSTICS {
        return Err(ProofTraceFailure::new(accepted_steps, final_state_ref, "proof trace diagnostics exceed bound"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProofTraceFailure {
    accepted_steps: usize,
    final_state_ref: String,
    diagnostic: String,
}

impl ProofTraceFailure {
    fn new(accepted_steps: usize, final_state_ref: impl AsRef<str>, diagnostic: impl Into<String>) -> Self {
        Self {
            accepted_steps,
            final_state_ref: final_state_ref.as_ref().to_owned(),
            diagnostic: diagnostic.into(),
        }
    }
}

fn pass_validation(accepted_steps: usize, final_state_ref: &str) -> ProofTraceValidation {
    let diagnostics = Vec::new();
    let decision = "pass".to_owned();
    ProofTraceValidation {
        summary: render_proof_trace_summary(&decision, accepted_steps, final_state_ref, &diagnostics),
        decision,
        accepted_steps,
        final_state_ref: final_state_ref.to_owned(),
        diagnostics,
    }
}

fn deny_validation(accepted_steps: usize, final_state_ref: &str, diagnostic: String) -> ProofTraceValidation {
    let diagnostics = vec![diagnostic];
    let decision = "deny".to_owned();
    ProofTraceValidation {
        summary: render_proof_trace_summary(&decision, accepted_steps, final_state_ref, &diagnostics),
        decision,
        accepted_steps,
        final_state_ref: final_state_ref.to_owned(),
        diagnostics,
    }
}

pub fn render_proof_trace_summary(
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

#[cfg(test)]
mod tests {
    use super::*;

    const LIFECYCLE_CHECK: &str = "molten-lifecycle-local-semantics";
    const PROOF_TRACE_LOGICAL_STEP: u64 = 1;

    fn state_ref(name: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(format!("state:{name}").as_bytes())
    }

    fn lifecycle_input(
        from_state: crate::lifecycle::State,
        to_state: crate::lifecycle::State,
        action: crate::lifecycle::Action,
    ) -> crate::lifecycle::TransitionInput {
        crate::lifecycle::TransitionInput {
            entity_kind: crate::lifecycle::EntityKind::Service,
            entity_id: "proof-trace-service".to_owned(),
            from_state,
            to_state,
            action,
            cause: "proof-trace".to_owned(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supervisor_ref: None,
            logical_step: PROOF_TRACE_LOGICAL_STEP,
        }
    }

    fn lifecycle_step(
        before_state_ref: String,
        after_state_ref: String,
        input: crate::lifecycle::TransitionInput,
    ) -> ProofTraceStep {
        let transition = crate::lifecycle::transition_record(&input).expect("transition record");
        let receipt = crate::lifecycle::transition_receipt(&input).expect("transition receipt");
        ProofTraceStep {
            before_state_ref,
            transition_ref: transition.transition_ref,
            after_state_ref,
            check_names: vec![LIFECYCLE_CHECK.to_owned()],
            decision: receipt.decision,
            diagnostics: receipt.diagnostics,
            receipt_ref: receipt.receipt_ref,
            receipt: ProofReceiptEvidence::LifecycleTransition {
                transition_value: transition.value,
                receipt_value: receipt.value,
            },
        }
    }

    fn valid_proof_trace_steps() -> Vec<ProofTraceStep> {
        let declared = state_ref("declared");
        let spawning = state_ref("spawning");
        vec![
            lifecycle_step(
                declared,
                spawning.clone(),
                lifecycle_input(
                    crate::lifecycle::State::Declared,
                    crate::lifecycle::State::Spawning,
                    crate::lifecycle::Action::Spawn,
                ),
            ),
            lifecycle_step(
                spawning.clone(),
                spawning,
                lifecycle_input(
                    crate::lifecycle::State::Spawning,
                    crate::lifecycle::State::Ready,
                    crate::lifecycle::Action::Ready,
                ),
            ),
        ]
    }

    fn validate_steps(steps: &[ProofTraceStep], initial: &str, final_state: &str) -> ProofTraceValidation {
        validate_proof_trace(&ProofTrace {
            initial_state_ref: initial,
            final_state_ref: final_state,
            steps,
        })
    }

    #[test]
    fn proof_trace_replay_accepts_valid_lifecycle_steps_and_renders_summary() {
        // r[verify molten.testing.state_machine_proof.trace_contract]
        // r[verify molten.testing.state_machine_proof.trace_validator]
        let steps = valid_proof_trace_steps();
        let initial = state_ref("declared");
        let final_state = state_ref("spawning");
        let first = validate_steps(&steps, &initial, &final_state);
        let second = validate_steps(&steps, &initial, &final_state);

        assert_eq!(first.decision, "pass");
        assert_eq!(first.accepted_steps, steps.len());
        assert_eq!(first.final_state_ref, final_state);
        assert!(first.diagnostics.is_empty());
        assert_eq!(first.summary, second.summary);
        assert!(first.summary.contains(&format!("accepted_steps={}", steps.len())));
    }

    #[test]
    fn proof_trace_replay_rejects_missing_receipt_ref() {
        // r[verify molten.testing.state_machine_proof.trace_validator_negative]
        let mut steps = valid_proof_trace_steps();
        steps[0].receipt_ref.clear();
        let validation = validate_steps(&steps, &state_ref("declared"), &state_ref("spawning"));

        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics[0].contains("proof trace receipt ref is not canonical"));
    }

    #[test]
    fn proof_trace_replay_rejects_tampered_diagnostics() {
        // r[verify molten.testing.state_machine_proof.trace_validator_negative]
        let mut steps = valid_proof_trace_steps();
        steps[1].diagnostics.clear();
        let validation = validate_steps(&steps, &state_ref("declared"), &state_ref("spawning"));

        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics[0].contains("diagnostics do not match receipt"));
    }

    #[test]
    fn proof_trace_replay_rejects_stale_before_state_and_wrong_final_state() {
        // r[verify molten.testing.state_machine_proof.trace_validator_negative]
        let mut stale_steps = valid_proof_trace_steps();
        stale_steps[1].before_state_ref = state_ref("stale-before");
        let stale = validate_steps(&stale_steps, &state_ref("declared"), &state_ref("spawning"));
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics[0].contains("before-state mismatch"));

        let steps = valid_proof_trace_steps();
        let wrong_final = validate_steps(&steps, &state_ref("declared"), &state_ref("wrong-final"));
        assert_eq!(wrong_final.decision, "deny");
        assert!(wrong_final.diagnostics[0].contains("final-state mismatch"));
    }

    #[test]
    fn proof_trace_replay_rejects_out_of_order_steps() {
        // r[verify molten.testing.state_machine_proof.trace_validator_negative]
        let mut steps = valid_proof_trace_steps();
        steps.reverse();
        let validation = validate_steps(&steps, &state_ref("declared"), &state_ref("spawning"));

        assert_eq!(validation.decision, "deny");
        assert!(validation.diagnostics[0].contains("before-state mismatch"));
    }
}
