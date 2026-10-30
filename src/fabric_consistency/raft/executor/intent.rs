use super::*;

fn proposal_plan(state: &ReplicaState) -> ReplicaTransition {
    apply_replica_event(state, ReplicaEvent::Propose {
        request_ref: super::tests::test_ref("intent-request"),
        command_ref: super::tests::test_ref("intent-command"),
        command_schema_ref: super::tests::test_ref("intent-schema"),
    })
    .expect("proposal")
}

#[tokio::test]
async fn each_proposal_phase_failure_quarantines_without_clearing_the_marker() {
    let (leader, _) = super::tests::elect_node_a();
    let plan = proposal_plan(&leader);
    let mut successful = super::super::executor_tests::RecordingPorts::default();
    assert!(matches!(
        execute_planned_transition(&leader, plan.clone(), &mut successful).await,
        ReplicaExecutionOutcome::Applied(_)
    ));
    assert_eq!(successful.hard_states.last(), Some(&(plan.next.current_term, plan.next.voted_for.clone(), false)));
    for index in 0..successful.executed.len() {
        if successful.executed[index] == ReplicaEffectKind::Send {
            continue;
        }
        let mut ports = super::super::executor_tests::RecordingPorts {
            fail_at: Some(index),
            ..super::super::executor_tests::RecordingPorts::default()
        };
        let outcome = execute_planned_transition(&leader, plan.clone(), &mut ports).await;
        let ReplicaExecutionOutcome::Failed(failed) = outcome else {
            panic!("phase {index} must quarantine")
        };
        assert_eq!(failed.retained.lifecycle, ReplicaLifecycle::Stopped);
        assert!(matches!(failed.retained.rejoin, ReplicaRejoin::Recovering(_)));
        assert_eq!(ports.executed.len(), index + 1, "no effects after uncertainty");
        assert!(!ports.hard_states.iter().any(|state| !state.2), "failure must not clear intent");
        let failed_sequence = u32::try_from(index).expect("bounded effect sequence");
        assert!(failed.completed.iter().all(|item| item.sequence < failed_sequence));
    }
}

#[tokio::test]
async fn fences_are_admitted_before_any_proposal_effect() {
    let (mut leader, _) = super::tests::elect_node_a();
    let plan = proposal_plan(&leader);
    leader.profile.max_effects_per_step = plan.effects.len() + 1;
    let mut ports = super::super::executor_tests::RecordingPorts::default();
    let outcome = execute_planned_transition(&leader, plan, &mut ports).await;
    assert!(matches!(outcome, ReplicaExecutionOutcome::Denied { retained, .. } if retained == leader));
    assert!(ports.executed.is_empty(), "denial precedes even the intent write");
}

#[tokio::test]
async fn recovery_keeps_the_marker_and_admission_clears_exact_planned_vote_only_at_end() {
    let group = super::tests::active_group();
    let state = super::tests::started_state(&group, super::tests::NODE_A);
    let mut recovering = effects::quarantine(&state);
    recovering.lifecycle = ReplicaLifecycle::Running;
    let marker = ReplicaEffect::PersistHardState {
        term: state.current_term,
        voted_for: state.voted_for.clone(),
        is_recovery_required: true,
    };
    let mut ports = super::super::executor_tests::RecordingPorts::default();
    let recovery = ReplicaTransition {
        next: recovering.clone(),
        effects: vec![marker, ReplicaEffect::FlushLog {
            through_index: state.commit_index,
        }],
    };
    assert!(matches!(
        execute_planned_transition(&recovering, recovery, &mut ports).await,
        ReplicaExecutionOutcome::Applied(_)
    ));
    assert_eq!(
        ports.hard_states,
        vec![(state.current_term, state.voted_for.clone(), true)],
        "recovery reuses its initial marker without a clear"
    );

    let mut admitted = state.clone();
    admitted.current_term += 1;
    admitted.voted_for = Some(super::tests::NODE_B.to_owned());
    let effects = vec![
        ReplicaEffect::PersistHardState {
            term: admitted.current_term,
            voted_for: admitted.voted_for.clone(),
            is_recovery_required: false,
        },
        ReplicaEffect::FlushLog {
            through_index: state.commit_index,
        },
        ReplicaEffect::PersistHardState {
            term: admitted.current_term,
            voted_for: admitted.voted_for.clone(),
            is_recovery_required: false,
        },
    ];
    let mut ports = super::super::executor_tests::RecordingPorts::default();
    let outcome = execute_planned_transition(
        &recovering,
        ReplicaTransition {
            next: admitted.clone(),
            effects,
        },
        &mut ports,
    )
    .await;
    let ReplicaExecutionOutcome::Applied(applied) = outcome else {
        panic!("admission")
    };
    assert!(ports.hard_states[..ports.hard_states.len() - 1].iter().all(|state| state.2));
    assert_eq!(ports.hard_states.last(), Some(&(admitted.current_term, admitted.voted_for.clone(), false)));
    assert_eq!(applied.observations.last().map(|item| item.kind), Some(ReplicaEffectKind::PersistHardState));
}
