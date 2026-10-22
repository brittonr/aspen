use std::collections::BTreeSet;

use super::*;

/// Why a scenario declaration is rejected before any process starts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioIssue {
    EmptyScenarioId,
    /// The consensus path belongs to the consensus campaigns.
    ConsensusPathExcluded,
    /// The boundary is not the one the profile applies its fault at.
    ProfileBoundaryMismatch,
    BudgetOutOfBounds(BudgetKind),
    /// The fault hold is zero for a held fault, nonzero for an unheld one, or
    /// does not fit inside the wall-clock budget.
    FaultHoldOutOfBounds,
    StabilizationOutOfBounds,
    ExpectedHistoryEmpty,
    ExpectedHistoryOverBudget,
    DuplicateExpectedRequest(String),
}

/// Validate a declaration. An empty result admits it.
// r[impl molten.outside_in.real_stack]
// r[impl molten.outside_in.validation]
pub fn validate_scenario(declaration: &ScenarioDeclaration) -> Vec<ScenarioIssue> {
    let mut issues = Vec::new();
    if declaration.scenario_id.trim().is_empty() {
        issues.push(ScenarioIssue::EmptyScenarioId);
    }
    if declaration.path == ScenarioPath::Consensus {
        issues.push(ScenarioIssue::ConsensusPathExcluded);
    }
    if declaration.boundary != declaration.profile.boundary() {
        issues.push(ScenarioIssue::ProfileBoundaryMismatch);
    }
    issues.extend(budget_issues(&declaration.budgets));
    issues.extend(timing_issues(declaration));
    issues.extend(expected_issues(declaration));
    issues.sort();
    issues.dedup();
    issues
}

fn budget_issues(budgets: &ScenarioBudgets) -> Vec<ScenarioIssue> {
    let checks = [
        (BudgetKind::Processes, u64::from(budgets.max_processes), u64::from(MAX_SCENARIO_PROCESSES)),
        (BudgetKind::WallClock, budgets.wall_clock_ms, MAX_SCENARIO_WALL_CLOCK_MS),
        (BudgetKind::RetainedOutput, budgets.max_retained_output_bytes, MAX_RETAINED_OUTPUT_BYTES),
        (
            BudgetKind::ClientOperations,
            u64::from(budgets.max_client_operations),
            u64::from(MAX_CLIENT_OPERATIONS),
        ),
    ];
    checks
        .into_iter()
        .filter(|(_, value, maximum)| *value == 0 || value > maximum)
        .map(|(kind, _, _)| ScenarioIssue::BudgetOutOfBounds(kind))
        .collect()
}

fn timing_issues(declaration: &ScenarioDeclaration) -> Vec<ScenarioIssue> {
    let mut issues = Vec::new();
    let wall_clock = declaration.budgets.wall_clock_ms;
    let is_held = matches!(declaration.profile, FaultProfile::PauseResume | FaultProfile::PartitionHeal);
    let is_hold_valid = if is_held {
        declaration.fault_hold_ms > 0 && declaration.fault_hold_ms < wall_clock
    } else {
        declaration.fault_hold_ms == 0
    };
    if !is_hold_valid {
        issues.push(ScenarioIssue::FaultHoldOutOfBounds);
    }
    let settled = declaration.fault_hold_ms.saturating_add(declaration.stabilization_ms);
    if settled >= wall_clock {
        issues.push(ScenarioIssue::StabilizationOutOfBounds);
    }
    issues
}

fn expected_issues(declaration: &ScenarioDeclaration) -> Vec<ScenarioIssue> {
    let expected = &declaration.expected_processed;
    let mut issues = Vec::new();
    if expected.is_empty() {
        issues.push(ScenarioIssue::ExpectedHistoryEmpty);
    }
    let is_over_budget =
        u32::try_from(expected.len()).map_or(true, |count| count > declaration.budgets.max_client_operations);
    if is_over_budget {
        issues.push(ScenarioIssue::ExpectedHistoryOverBudget);
        return issues;
    }
    let mut seen = BTreeSet::new();
    for request in expected {
        if !seen.insert(request.as_str()) {
            issues.push(ScenarioIssue::DuplicateExpectedRequest(request.clone()));
        }
    }
    issues
}
