use super::MAX_SYSTEM_EXTENSION_ITEMS;
use super::valid_ref;
use super::valid_token;

pub const MAX_RECOVERY_GROUP_MEMBERS: usize = 64;
pub const MAX_RECOVERY_HISTORY: usize = 256;
pub const MAX_RECOVERY_WINDOW_TICKS: u64 = 1_000_000_000;
pub const MAX_RECOVERY_ATTEMPTS: u64 = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RestartClass {
    Permanent,
    Transient,
    Temporary,
}

impl RestartClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Permanent => "permanent",
            Self::Transient => "transient",
            Self::Temporary => "temporary",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryGroupMember {
    pub member_id: String,
    pub restart_class: RestartClass,
    pub after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryGroupDeclaration {
    pub group_id: String,
    pub members: Vec<RecoveryGroupMember>,
    pub window_ticks: u64,
    pub backoff_ticks: u64,
    pub max_restarts_per_member: u64,
    pub max_restarts_per_group: u64,
    pub elapsed_source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryGroupIssue {
    EmptyMembers,
    TooManyMembers(usize),
    MalformedGroupId(String),
    MalformedMemberId(String),
    DuplicateMember(String),
    DuplicateDependency { member: String, dependency: String },
    SelfDependency(String),
    UnknownDependency { member: String, dependency: String },
    OrderingConflict { member: String, dependency: String },
    InvalidWindow(u64),
    InvalidBackoff(u64),
    InvalidMemberBudget(u64),
    InvalidGroupBudget(u64),
    InvalidElapsedSource(String),
}

// r[impl molten.system_extension.recovery_groups]
pub fn validate_recovery_group(group: &RecoveryGroupDeclaration) -> Vec<RecoveryGroupIssue> {
    let mut issues = Vec::new();
    if !valid_token(&group.group_id) {
        issues.push(RecoveryGroupIssue::MalformedGroupId(group.group_id.clone()));
    }
    if !valid_ref(&group.elapsed_source_ref) {
        issues.push(RecoveryGroupIssue::InvalidElapsedSource(group.elapsed_source_ref.clone()));
    }
    if group.window_ticks == 0 || group.window_ticks > MAX_RECOVERY_WINDOW_TICKS {
        issues.push(RecoveryGroupIssue::InvalidWindow(group.window_ticks));
    }
    if group.backoff_ticks > group.window_ticks {
        issues.push(RecoveryGroupIssue::InvalidBackoff(group.backoff_ticks));
    }
    if group.max_restarts_per_member == 0 || group.max_restarts_per_member > MAX_RECOVERY_ATTEMPTS {
        issues.push(RecoveryGroupIssue::InvalidMemberBudget(group.max_restarts_per_member));
    }
    if group.max_restarts_per_group == 0 || group.max_restarts_per_group > MAX_RECOVERY_ATTEMPTS {
        issues.push(RecoveryGroupIssue::InvalidGroupBudget(group.max_restarts_per_group));
    }
    if group.members.is_empty() {
        issues.push(RecoveryGroupIssue::EmptyMembers);
    }
    if group.members.len() > MAX_RECOVERY_GROUP_MEMBERS || group.members.len() > MAX_SYSTEM_EXTENSION_ITEMS {
        issues.push(RecoveryGroupIssue::TooManyMembers(group.members.len()));
    }
    let mut member_ids = Vec::with_capacity(group.members.len());
    for member in &group.members {
        if !valid_token(&member.member_id) {
            issues.push(RecoveryGroupIssue::MalformedMemberId(member.member_id.clone()));
        }
        if member_ids.contains(&member.member_id) {
            issues.push(RecoveryGroupIssue::DuplicateMember(member.member_id.clone()));
        }
        if member.after.len() != member.after.iter().collect::<std::collections::BTreeSet<_>>().len() {
            let mut seen = std::collections::BTreeSet::new();
            for dependency in &member.after {
                if !seen.insert(dependency) {
                    issues.push(RecoveryGroupIssue::DuplicateDependency {
                        member: member.member_id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
        }
        member_ids.push(member.member_id.clone());
    }
    for (index, member) in group.members.iter().enumerate() {
        for dependency in &member.after {
            if dependency == &member.member_id {
                issues.push(RecoveryGroupIssue::SelfDependency(member.member_id.clone()));
                continue;
            }
            let Some(dependency_index) = member_ids.iter().position(|member_id| member_id == dependency) else {
                issues.push(RecoveryGroupIssue::UnknownDependency {
                    member: member.member_id.clone(),
                    dependency: dependency.clone(),
                });
                continue;
            };
            if dependency_index >= index {
                issues.push(RecoveryGroupIssue::OrderingConflict {
                    member: member.member_id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    issues
}
