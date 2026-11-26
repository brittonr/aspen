//! View-change recovery ordering.

// r[verify molten.consensus.fast_path_model.view_change_recovery]
#[test]
fn recovery_orders_agreed_set_or_empty_marker_before_resume() {
    let initial = crate::succession::RecoveryState {
        acceleration_view: super::support::INITIAL_VIEW,
        base_view: super::support::INITIAL_VIEW,
        last_normal_view: super::support::INITIAL_VIEW,
        phase: crate::succession::RecoveryPhase::Normal,
        accepted_commands: super::support::strings(&["command-a"]),
        recovery_set: std::collections::BTreeSet::new(),
        marker_ref: None,
    };
    let paused =
        crate::succession::transition_recovery(&initial, &crate::succession::RecoveryAction::BeginViewChange {
            new_base_view: super::support::NEXT_VIEW,
        })
        .expect("pause recovery");
    let agreed =
        crate::succession::transition_recovery(&paused, &crate::succession::RecoveryAction::AgreeRecoverySet {
            commands: super::support::strings(&["command-a"]),
        })
        .expect("agree recovery");
    assert_eq!(
        crate::succession::transition_recovery(&agreed, &crate::succession::RecoveryAction::ResumeNormalView {
            new_acceleration_view: super::support::NEXT_VIEW,
        }),
        Err(crate::succession::RecoveryIssue::ResumeBeforeMarker)
    );
    let marked =
        crate::succession::transition_recovery(&agreed, &crate::succession::RecoveryAction::CommitRecoveryMarker {
            marker_ref: "blake3:recovery-marker".to_owned(),
        })
        .expect("commit marker");
    let resumed =
        crate::succession::transition_recovery(&marked, &crate::succession::RecoveryAction::ResumeNormalView {
            new_acceleration_view: super::support::NEXT_VIEW,
        })
        .expect("resume normal");
    assert_eq!(resumed.phase, crate::succession::RecoveryPhase::Normal);
    assert_eq!(resumed.acceleration_view, super::support::NEXT_VIEW);
}

// r[verify molten.consensus.fast_path_model.view_change_recovery]
#[test]
fn cascading_recovery_cannot_drop_previously_accepted_commands() {
    let state = crate::succession::RecoveryState {
        acceleration_view: super::support::INITIAL_VIEW,
        base_view: super::support::NEXT_VIEW,
        last_normal_view: super::support::INITIAL_VIEW,
        phase: crate::succession::RecoveryPhase::Paused,
        accepted_commands: super::support::strings(&["command-a"]),
        recovery_set: super::support::strings(&["command-a"]),
        marker_ref: None,
    };
    assert_eq!(
        crate::succession::transition_recovery(&state, &crate::succession::RecoveryAction::AgreeRecoverySet {
            commands: std::collections::BTreeSet::new(),
        }),
        Err(crate::succession::RecoveryIssue::RecoverySetDropsAcceptedCommand)
    );
}
