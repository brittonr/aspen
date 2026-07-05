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
    ) -> Step {
        let transition = crate::lifecycle::transition_record(&input).expect("transition record");
        let receipt = crate::lifecycle::transition_receipt(&input).expect("transition receipt");
        Step {
            before_state_ref,
            transition_ref: transition.transition_ref,
            after_state_ref,
            check_names: vec![LIFECYCLE_CHECK.to_owned()],
            decision: receipt.decision,
            diagnostics: receipt.diagnostics,
            receipt_ref: receipt.receipt_ref,
            receipt: ReceiptEvidence::LifecycleTransition {
                transition_value: transition.value,
                receipt_value: receipt.value,
            },
        }
    }

    fn valid_steps() -> Vec<Step> {
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
