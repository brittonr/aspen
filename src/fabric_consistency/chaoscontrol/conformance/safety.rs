//! Molten-side safety evaluation over supplied committed-apply facts.
//!
//! The producer validates chain continuity, divergence, rollback, and lossless
//! gaps. Molten additionally binds each logical operation to one command index
//! and one application receipt, so a second committed application cannot hide
//! behind an otherwise well-formed chain. This module performs no I/O.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    History,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("history")
    }
}

impl std::error::Error for Issue {}

/// One committed apply with the Molten identities the producer chain omits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedChain {
    pub operation_ref: String,
    pub participant_ref: String,
    pub command_ref: String,
    pub application_receipt_ref: String,
    pub chain: chaoscontrol_smr::ChainObservation,
}

impl From<&super::wire::ApplyBinding> for AppliedChain {
    fn from(binding: &super::wire::ApplyBinding) -> Self {
        Self {
            operation_ref: binding.operation_ref.clone(),
            participant_ref: binding.participant_ref.clone(),
            command_ref: binding.command_ref.clone(),
            application_receipt_ref: binding.application_receipt_ref.clone(),
            chain: binding.chain.clone(),
        }
    }
}

/// Molten-specific violations that a well-formed chain cannot express.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationClass {
    DuplicateApplication,
    OperationIndexDrift,
    CommandDrift,
    ReceiptReuse,
}

impl ViolationClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DuplicateApplication => "operation-applied-more-than-once",
            Self::OperationIndexDrift => "operation-command-index-drift",
            Self::CommandDrift => "operation-command-identity-drift",
            Self::ReceiptReuse => "reused-application-receipt",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub class: ViolationClass,
    pub operation_ref: String,
    pub command_index: u64,
}

/// The producer history plus Molten's operation-level observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub history: chaoscontrol_smr::HistoryReport,
    pub violations: Vec<Violation>,
}

impl Report {
    /// A lossless gap is an observer defect, not a consensus-safety failure.
    pub fn is_safety_failure(&self) -> bool {
        self.history
            .violations
            .iter()
            .any(|violation| violation.class != chaoscontrol_smr::SafetyViolationClass::LosslessGap)
            || !self.violations.is_empty()
    }

    pub fn is_observer_incomplete(&self) -> bool {
        self.history
            .violations
            .iter()
            .any(|violation| violation.class == chaoscontrol_smr::SafetyViolationClass::LosslessGap)
    }

    pub fn verdict(&self) -> chaoscontrol_smr::SafetyVerdict {
        if self.is_safety_failure() {
            chaoscontrol_smr::SafetyVerdict::Fail
        } else {
            self.history.verdict.clone()
        }
    }
}

/// Evaluate one bounded history and its operation identity bindings.
// r[impl molten.consensus.chaoscontrol_safety]
pub fn evaluate(plan: &chaoscontrol_smr::SmrWorkloadPlan, applies: &[AppliedChain]) -> Result<Report, Issue> {
    let chains = applies.iter().map(|apply| apply.chain.clone()).collect::<Vec<_>>();
    let history = chaoscontrol_smr::validate_history(plan, &chains).map_err(|_| Issue::History)?;
    let mut operations = BTreeMap::<&str, (u64, &str)>::new();
    let mut replica_operations = BTreeMap::<(&str, &str), (u64, &str)>::new();
    let mut receipts = BTreeMap::<&str, (&str, u64, &str)>::new();
    let mut violations = Vec::new();
    for apply in applies {
        let index = apply.chain.command_index;
        let receipt = apply.application_receipt_ref.as_str();
        match receipts.get(receipt).copied() {
            None => {
                receipts.insert(receipt, (apply.participant_ref.as_str(), index, apply.operation_ref.as_str()));
            }
            Some((participant, observed_index, operation))
                if participant == apply.participant_ref.as_str()
                    && observed_index == index
                    && operation == apply.operation_ref.as_str() => {}
            Some(_) => violations.push(Violation {
                class: ViolationClass::ReceiptReuse,
                operation_ref: apply.operation_ref.clone(),
                command_index: index,
            }),
        }
        match replica_operations.get(&(apply.participant_ref.as_str(), apply.operation_ref.as_str())).copied() {
            None => {
                replica_operations
                    .insert((apply.participant_ref.as_str(), apply.operation_ref.as_str()), (index, receipt));
            }
            Some((observed_index, observed_receipt)) if observed_index == index && observed_receipt == receipt => {}
            Some(_) => violations.push(Violation {
                class: ViolationClass::DuplicateApplication,
                operation_ref: apply.operation_ref.clone(),
                command_index: index,
            }),
        }
        match operations.get(apply.operation_ref.as_str()).copied() {
            None => {
                operations.insert(apply.operation_ref.as_str(), (index, apply.command_ref.as_str()));
            }
            Some((prior_index, prior_command)) => {
                if prior_index != index {
                    violations.push(Violation {
                        class: ViolationClass::OperationIndexDrift,
                        operation_ref: apply.operation_ref.clone(),
                        command_index: index,
                    });
                }
                if prior_command != apply.command_ref.as_str() {
                    violations.push(Violation {
                        class: ViolationClass::CommandDrift,
                        operation_ref: apply.operation_ref.clone(),
                        command_index: index,
                    });
                }
            }
        }
    }
    Ok(Report { history, violations })
}
