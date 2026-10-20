// r[verify molten.system_extension.recovery_groups]
// r[verify molten.system_extension.restart_rate_budget]
use super::*;

const TEST_ELAPSED_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn member(member_id: &str, restart_class: RestartClass, after: &[&str]) -> RecoveryGroupMember {
    RecoveryGroupMember {
        member_id: member_id.to_string(),
        restart_class,
        after: after.iter().map(|value| (*value).to_string()).collect(),
    }
}

fn valid_group() -> RecoveryGroupDeclaration {
    RecoveryGroupDeclaration {
        group_id: "group-main".to_string(),
        members: vec![
            member("verifier", RestartClass::Permanent, &[]),
            member("session", RestartClass::Permanent, &["verifier"]),
            member("worker", RestartClass::Transient, &["session"]),
        ],
        window_ticks: 100,
        backoff_ticks: 5,
        max_restarts_per_member: 3,
        max_restarts_per_group: 6,
        elapsed_source_ref: TEST_ELAPSED_REF.to_string(),
    }
}

fn exit(member_id: &str, was_successful: bool, failure: Option<FailureClass>) -> RecoveryExit {
    RecoveryExit {
        member_id: member_id.to_string(),
        was_successful,
        failure,
    }
}

fn attempt(member_id: &str, observed_at_tick: u64) -> RecoveryAttempt {
    RecoveryAttempt {
        member_id: member_id.to_string(),
        observed_at_tick,
    }
}

fn plan(
    group: &RecoveryGroupDeclaration,
    exit: &RecoveryExit,
    history: &[RecoveryAttempt],
    now_tick: u64,
) -> RecoveryPlan {
    plan_recovery(&RecoveryPlanInput {
        group,
        exit,
        history,
        now_tick,
    })
    .expect("recovery plan")
}

#[test]
fn valid_group_admits() {
    assert!(validate_recovery_group(&valid_group()).is_empty());
}

#[test]
fn duplicate_unknown_self_and_cyclic_members_are_rejected() {
    let mut duplicate = valid_group();
    duplicate.members.push(member("worker", RestartClass::Temporary, &[]));
    assert!(validate_recovery_group(&duplicate).iter().any(|issue| matches!(
        issue,
        RecoveryGroupIssue::DuplicateMember(value) if value == "worker"
    )));

    let mut unknown = valid_group();
    unknown.members[1].after.push("missing".to_string());
    assert!(validate_recovery_group(&unknown).iter().any(|issue| matches!(
        issue,
        RecoveryGroupIssue::UnknownDependency { dependency, .. } if dependency == "missing"
    )));

    let mut self_dependency = valid_group();
    self_dependency.members[1].after.push("session".to_string());
    assert!(
        validate_recovery_group(&self_dependency)
            .iter()
            .any(|issue| matches!(issue, RecoveryGroupIssue::SelfDependency(value) if value == "session"))
    );

    let mut cycle = valid_group();
    cycle.members[0].after = vec!["worker".to_string()];
    assert!(
        validate_recovery_group(&cycle)
            .iter()
            .any(|issue| matches!(issue, RecoveryGroupIssue::OrderingConflict { .. }))
    );
}

#[test]
fn invalid_bounds_and_elapsed_source_are_rejected() {
    let mut group = valid_group();
    group.window_ticks = 0;
    group.backoff_ticks = 1;
    group.max_restarts_per_member = 0;
    group.max_restarts_per_group = 0;
    group.elapsed_source_ref = "not-a-ref".to_string();
    let issues = validate_recovery_group(&group);
    assert!(issues.iter().any(|issue| matches!(issue, RecoveryGroupIssue::InvalidWindow(0))));
    assert!(issues.iter().any(|issue| matches!(issue, RecoveryGroupIssue::InvalidMemberBudget(0))));
    assert!(issues.iter().any(|issue| matches!(issue, RecoveryGroupIssue::InvalidGroupBudget(0))));
    assert!(issues.iter().any(|issue| matches!(issue, RecoveryGroupIssue::InvalidElapsedSource(_))));
}

#[test]
fn one_member_plan_matches_existing_supervision_behavior() {
    let group = RecoveryGroupDeclaration {
        group_id: "group-one".to_string(),
        members: vec![member("worker", RestartClass::Permanent, &[])],
        window_ticks: 100,
        backoff_ticks: 0,
        max_restarts_per_member: 2,
        max_restarts_per_group: 4,
        elapsed_source_ref: TEST_ELAPSED_REF.to_string(),
    };
    let retryable = exit("worker", false, Some(FailureClass::Retryable));
    assert_eq!(plan(&group, &retryable, &[], 10).decision, RecoveryDecision::Restart);
    assert_eq!(plan(&group, &retryable, &[], 10).actions, vec![
        RecoveryAction::Stop {
            member_id: "worker".to_string(),
        },
        RecoveryAction::Restart {
            member_id: "worker".to_string(),
        },
    ]);

    let exhausted = [attempt("worker", 1), attempt("worker", 2)];
    assert_eq!(plan(&group, &retryable, &exhausted, 10).decision, RecoveryDecision::Quarantine);
    assert_eq!(plan(&group, &retryable, &exhausted, 10).actions, vec![
        RecoveryAction::Stop {
            member_id: "worker".to_string(),
        },
        RecoveryAction::Quarantine {
            member_id: "worker".to_string(),
        },
    ]);

    let fatal = exit("worker", false, Some(FailureClass::Fatal));
    assert_eq!(plan(&group, &fatal, &[], 10).decision, RecoveryDecision::Quarantine);
}

#[test]
fn dependent_group_restarts_only_the_affected_suffix_in_order() {
    let group = valid_group();
    let session_failure = exit("session", false, Some(FailureClass::Retryable));
    assert_eq!(plan(&group, &session_failure, &[], 10).actions, vec![
        RecoveryAction::Stop {
            member_id: "worker".to_string(),
        },
        RecoveryAction::Stop {
            member_id: "session".to_string(),
        },
        RecoveryAction::Restart {
            member_id: "session".to_string(),
        },
        RecoveryAction::Restart {
            member_id: "worker".to_string(),
        },
    ]);

    let worker_failure = exit("worker", false, Some(FailureClass::Retryable));
    assert_eq!(plan(&group, &worker_failure, &[], 10).actions, vec![
        RecoveryAction::Stop {
            member_id: "worker".to_string(),
        },
        RecoveryAction::Restart {
            member_id: "worker".to_string(),
        },
    ]);
}

#[test]
fn restart_classes_distinguish_success_and_abnormal_exit() {
    let group = valid_group();
    let mut temporary_group = group.clone();
    temporary_group.members[2].restart_class = RestartClass::Temporary;
    let temporary = exit("worker", true, None);
    assert_eq!(plan(&temporary_group, &temporary, &[], 10).decision, RecoveryDecision::Stop);

    let transient_success = exit("worker", true, None);
    assert_eq!(plan(&group, &transient_success, &[], 10).decision, RecoveryDecision::Stop);

    let permanent_success = exit("session", true, None);
    assert_eq!(plan(&group, &permanent_success, &[], 10).decision, RecoveryDecision::Restart);

    let transient_failure = exit("worker", false, Some(FailureClass::Retryable));
    assert_eq!(plan(&group, &transient_failure, &[], 10).decision, RecoveryDecision::Restart);
}

#[test]
fn windowed_budgets_backoff_and_group_escalation_are_enforced() {
    let group = valid_group();
    let failure = exit("session", false, Some(FailureClass::Retryable));
    let member_exhausted = [attempt("session", 1), attempt("session", 2), attempt("session", 3)];
    assert_eq!(plan(&group, &failure, &member_exhausted, 10).decision, RecoveryDecision::Quarantine);

    let group_exhausted = [
        attempt("verifier", 1),
        attempt("session", 2),
        attempt("worker", 3),
        attempt("verifier", 4),
        attempt("session", 5),
        attempt("worker", 6),
    ];
    assert_eq!(plan(&group, &failure, &group_exhausted, 10).decision, RecoveryDecision::EscalateGroup);

    let backing_off = [attempt("session", 9)];
    assert_eq!(plan(&group, &failure, &backing_off, 10).decision, RecoveryDecision::Quarantine);

    let expired_attempts = [attempt("session", 1)];
    assert_eq!(plan(&group, &failure, &expired_attempts, 200).decision, RecoveryDecision::Restart);
}

#[test]
fn invalid_history_and_unknown_members_are_rejected() {
    let group = valid_group();
    let failure = exit("session", false, Some(FailureClass::Retryable));
    let future = [attempt("session", 11)];
    assert!(
        plan_recovery(&RecoveryPlanInput {
            group: &group,
            exit: &failure,
            history: &future,
            now_tick: 10,
        })
        .is_err()
    );
    let unknown = exit("missing", false, Some(FailureClass::Retryable));
    assert!(
        plan_recovery(&RecoveryPlanInput {
            group: &group,
            exit: &unknown,
            history: &[],
            now_tick: 10,
        })
        .is_err()
    );
}

#[test]
fn recovery_planning_is_deterministic() {
    let group = valid_group();
    let failure = exit("session", false, Some(FailureClass::Retryable));
    let history = [attempt("session", 1)];
    assert_eq!(plan(&group, &failure, &history, 10), plan(&group, &failure, &history, 10));
}
