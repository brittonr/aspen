use super::FailureClass;
use super::MAX_RECOVERY_HISTORY;
use super::RecoveryGroupDeclaration;
use super::RecoveryGroupMember;
use super::RestartClass;
use super::validate_recovery_group;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryExit {
    pub member_id: String,
    pub was_successful: bool,
    pub failure: Option<FailureClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAttempt {
    pub member_id: String,
    pub observed_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryPlanInput<'a> {
    pub group: &'a RecoveryGroupDeclaration,
    pub exit: &'a RecoveryExit,
    pub history: &'a [RecoveryAttempt],
    pub now_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    Stop { member_id: String },
    Restart { member_id: String },
    Quarantine { member_id: String },
    EscalateGroup { group_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDecision {
    Stop,
    Restart,
    Quarantine,
    EscalateGroup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryPlan {
    pub decision: RecoveryDecision,
    pub actions: Vec<RecoveryAction>,
    pub consumed_restart: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryPlanIssue {
    InvalidGroup,
    UnknownMember(String),
    HistoryBoundExceeded,
    FutureObservation {
        member_id: String,
        observed_at_tick: u64,
        now_tick: u64,
    },
}

// r[impl molten.system_extension.recovery_groups]
// r[impl molten.system_extension.restart_rate_budget]
pub fn plan_recovery(input: &RecoveryPlanInput<'_>) -> Result<RecoveryPlan, RecoveryPlanIssue> {
    if !validate_recovery_group(input.group).is_empty() {
        return Err(RecoveryPlanIssue::InvalidGroup);
    }
    let failed_index = input
        .group
        .members
        .iter()
        .position(|member| member.member_id == input.exit.member_id)
        .ok_or_else(|| RecoveryPlanIssue::UnknownMember(input.exit.member_id.clone()))?;
    if input.history.len() > MAX_RECOVERY_HISTORY {
        return Err(RecoveryPlanIssue::HistoryBoundExceeded);
    }
    for attempt in input.history {
        if !input.group.members.iter().any(|member| member.member_id == attempt.member_id) {
            return Err(RecoveryPlanIssue::UnknownMember(attempt.member_id.clone()));
        }
        if attempt.observed_at_tick > input.now_tick {
            return Err(RecoveryPlanIssue::FutureObservation {
                member_id: attempt.member_id.clone(),
                observed_at_tick: attempt.observed_at_tick,
                now_tick: input.now_tick,
            });
        }
    }

    let failed = &input.group.members[failed_index];
    let needs_restart = match failed.restart_class {
        RestartClass::Permanent => input.exit.was_successful || input.exit.failure == Some(FailureClass::Retryable),
        RestartClass::Transient => !input.exit.was_successful && input.exit.failure == Some(FailureClass::Retryable),
        RestartClass::Temporary => false,
    };
    let affected = &input.group.members[failed_index..];
    if !needs_restart {
        if input.exit.was_successful {
            return Ok(RecoveryPlan {
                decision: RecoveryDecision::Stop,
                actions: vec![RecoveryAction::Stop {
                    member_id: failed.member_id.clone(),
                }],
                consumed_restart: false,
            });
        }
        return Ok(quarantine_plan(affected, &failed.member_id));
    }

    let window_start = input.now_tick.saturating_sub(input.group.window_ticks);
    let in_window = input.history.iter().filter(|attempt| attempt.observed_at_tick >= window_start);
    let member_attempts = in_window.clone().filter(|attempt| attempt.member_id == failed.member_id).count() as u64;
    let group_attempts = in_window.count() as u64;
    if group_attempts >= input.group.max_restarts_per_group {
        return Ok(escalate_plan(input.group, affected));
    }
    if member_attempts >= input.group.max_restarts_per_member {
        return Ok(quarantine_plan(affected, &failed.member_id));
    }
    let last_member_attempt = input
        .history
        .iter()
        .filter(|attempt| attempt.member_id == failed.member_id)
        .map(|attempt| attempt.observed_at_tick)
        .max();
    if last_member_attempt.is_some_and(|observed| input.now_tick - observed < input.group.backoff_ticks) {
        return Ok(quarantine_plan(affected, &failed.member_id));
    }

    let mut actions = Vec::with_capacity(affected.len().saturating_mul(2));
    for member in affected.iter().rev() {
        actions.push(RecoveryAction::Stop {
            member_id: member.member_id.clone(),
        });
    }
    for member in affected {
        actions.push(RecoveryAction::Restart {
            member_id: member.member_id.clone(),
        });
    }
    Ok(RecoveryPlan {
        decision: RecoveryDecision::Restart,
        actions,
        consumed_restart: true,
    })
}

fn quarantine_plan(affected: &[RecoveryGroupMember], failed_member: &str) -> RecoveryPlan {
    let mut actions = Vec::with_capacity(affected.len().saturating_add(1));
    for member in affected.iter().rev() {
        actions.push(RecoveryAction::Stop {
            member_id: member.member_id.clone(),
        });
    }
    actions.push(RecoveryAction::Quarantine {
        member_id: failed_member.to_string(),
    });
    RecoveryPlan {
        decision: RecoveryDecision::Quarantine,
        actions,
        consumed_restart: false,
    }
}

fn escalate_plan(group: &RecoveryGroupDeclaration, affected: &[RecoveryGroupMember]) -> RecoveryPlan {
    let mut actions = Vec::with_capacity(affected.len().saturating_add(1));
    for member in affected.iter().rev() {
        actions.push(RecoveryAction::Stop {
            member_id: member.member_id.clone(),
        });
    }
    actions.push(RecoveryAction::EscalateGroup {
        group_id: group.group_id.clone(),
    });
    RecoveryPlan {
        decision: RecoveryDecision::EscalateGroup,
        actions,
        consumed_restart: false,
    }
}
