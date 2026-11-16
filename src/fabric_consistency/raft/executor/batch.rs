use super::*;

struct IntentAdmission {
    is_guarded: bool,
    has_prepend: bool,
    effect_count: usize,
}

pub(super) async fn execute_planned_transition<P: LiveReplicaEffectPorts>(
    retained: &ReplicaState,
    planned: ReplicaTransition,
    ports: &mut P,
) -> ReplicaExecutionOutcome {
    // Borrow the planned payloads: fencing must not duplicate commands or entries.
    let is_guarded = planned.effects.iter().any(effects::requires_durable_intent);
    let has_prepend = is_guarded && !matches!(planned.effects.first(), Some(ReplicaEffect::PersistHardState { .. }));
    let has_clear = is_guarded && !matches!(planned.next.rejoin, ReplicaRejoin::Recovering(_));
    let effect_count = planned
        .effects
        .len()
        .saturating_add(usize::from(has_prepend))
        .saturating_add(usize::from(has_clear));
    if effect_count > retained.profile.max_effects_per_step || u32::try_from(effect_count).is_err() {
        return ReplicaExecutionOutcome::Denied {
            retained: retained.clone(),
            diagnostic: "live Raft durable intent exceeds the admitted effect bound".to_string(),
        };
    }
    execute_intents(retained, planned, ports, IntentAdmission {
        is_guarded,
        has_prepend,
        effect_count,
    })
    .await
}

async fn execute_intents<P: LiveReplicaEffectPorts>(
    retained: &ReplicaState,
    planned: ReplicaTransition,
    ports: &mut P,
    admission: IntentAdmission,
) -> ReplicaExecutionOutcome {
    let mut completed = Vec::with_capacity(admission.effect_count);
    let mut transport_failures = Vec::with_capacity(admission.effect_count);
    for index in 0..admission.effect_count {
        let effect =
            index.checked_sub(usize::from(admission.has_prepend)).and_then(|offset| planned.effects.get(offset));
        let kind = effect.map_or(ReplicaEffectKind::PersistHardState, effects::effect_kind);
        // Admission checked the entire fenced batch fits the sequence domain.
        let sequence = index as u32;
        let result = match effect {
            Some(effect) => effects::execute_effect(ports, effect, admission.is_guarded, &planned.next).await,
            None if admission.has_prepend && index == 0 => {
                ports.persist_hard_state(retained.current_term, retained.voted_for.as_deref(), true)
            }
            None => ports.persist_hard_state(planned.next.current_term, planned.next.voted_for.as_deref(), false),
        };
        let evidence_ref = match result {
            Ok(reference) => reference,
            Err(error) if kind == ReplicaEffectKind::Send => {
                // Raft tolerates message loss. Keep the planned authority and run later
                // effects, especially timers, without claiming successful delivery.
                if transport_failures.len() < admission.effect_count {
                    transport_failures.push(ReplicaSendFailureObservation {
                        sequence,
                        diagnostic: effects::bounded_transport_diagnostic(&error),
                    });
                }
                continue;
            }
            Err(error) => {
                return ReplicaExecutionOutcome::Failed(Box::new(FailedReplicaTransition {
                    retained: effects::quarantine(retained),
                    planned: planned.next,
                    completed,
                    transport_failures,
                    failed_kind: kind,
                    diagnostic: error.to_string(),
                }));
            }
        };
        if let Err(error) = crate::preserves_rail::validate_content_ref(&evidence_ref) {
            return ReplicaExecutionOutcome::Failed(Box::new(FailedReplicaTransition {
                retained: effects::quarantine(retained),
                planned: planned.next,
                completed,
                transport_failures,
                failed_kind: kind,
                diagnostic: format!("live Raft effect returned invalid evidence ref: {error}"),
            }));
        }
        completed.push(ReplicaEffectObservation {
            sequence,
            kind,
            evidence_ref,
        });
    }
    ReplicaExecutionOutcome::Applied(ExecutedReplicaTransition {
        next: planned.next,
        observations: completed,
        transport_failures,
    })
}
