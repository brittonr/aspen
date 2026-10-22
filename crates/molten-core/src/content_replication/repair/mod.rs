//! Closure repair at the canonical-content boundary.
//!
//! The planner works on a required-closure set and a bounded inventory of
//! per-replica, per-object verification facts. Donor eligibility is per object:
//! a replica donates exactly the required objects it holds with a passing
//! verification result. The plan is complete when every required object has an
//! intact verifiable copy, and it is a typed incomplete repair when one does
//! not. The incomplete plan still carries the transfers for the objects that do
//! have donors.
//!
//! Repair recovers bytes matching already-established identities. It creates no
//! new content identity, and it proves nothing about which head is current.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::content_replication::Issue;

pub const MAX_REPAIR_OBJECTS: usize = 256;
pub const MAX_REPAIR_INVENTORY: usize = 4_096;

/// One per-replica, per-object verification fact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReplicaObjectFact {
    pub peer_id: String,
    pub fault_domain: String,
    pub object_ref: String,
    pub present: bool,
    pub identity_verified: bool,
}

impl ReplicaObjectFact {
    /// Whether the replica is an eligible donor for the object.
    pub fn is_intact_donor(&self) -> bool {
        self.present && self.identity_verified
    }
}

/// The required closure set and the bounded inventory to repair it from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureRepairInput {
    pub required_objects: Vec<String>,
    pub inventory: Vec<ReplicaObjectFact>,
    pub max_transfers: usize,
}

/// One planned per-object transfer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClosureTransfer {
    pub object_ref: String,
    pub donor_peer_id: String,
    pub donor_fault_domain: String,
}

/// Whether the repair can recover the whole required closure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepairCompleteness {
    /// Every required object has an intact verifiable copy.
    Complete,
    /// At least one required object has no intact verifiable copy.
    Incomplete { unrecoverable: Vec<String> },
}

impl RepairCompleteness {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete { .. } => "incomplete",
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// The bounded repair plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureRepairPlan {
    pub plan_ref: String,
    pub completeness: RepairCompleteness,
    pub transfers: Vec<ClosureTransfer>,
    pub repaired_objects: Vec<String>,
    pub unrecoverable_objects: Vec<String>,
    pub issues: Vec<Issue>,
    pub non_claims: Vec<String>,
}

/// Plan a closure repair over the bounded inventory.
///
/// The plan is complete when every required object has an intact verifiable
/// copy. Otherwise the plan is a typed incomplete repair that names the
/// unrecoverable identities and keeps the transfers for the objects that do
/// have donors.
pub fn plan_closure_repair(input: &ClosureRepairInput) -> Result<ClosureRepairPlan, Issue> {
    let issues = validate_input(input);
    if !issues.is_empty() {
        return Err(issues.first().copied().unwrap_or(Issue::InvalidManifest));
    }
    let mut required = input.required_objects.clone();
    required.sort();
    required.dedup();
    let mut donors: BTreeMap<&str, Vec<&ReplicaObjectFact>> = BTreeMap::new();
    for fact in &input.inventory {
        donors.entry(fact.object_ref.as_str()).or_default().push(fact);
    }
    let mut transfers = Vec::with_capacity(required.len());
    let mut repaired = Vec::with_capacity(required.len());
    let mut unrecoverable = Vec::new();
    for object in &required {
        let mut candidates = donors
            .get(object.as_str())
            .map(|facts| facts.iter().filter(|fact| fact.is_intact_donor()).copied().collect::<Vec<_>>())
            .unwrap_or_default();
        candidates.sort_by(|left, right| {
            (left.fault_domain.as_str(), left.peer_id.as_str())
                .cmp(&(right.fault_domain.as_str(), right.peer_id.as_str()))
        });
        let Some(donor) = candidates.first() else {
            unrecoverable.push(object.clone());
            continue;
        };
        transfers.push(ClosureTransfer {
            object_ref: object.clone(),
            donor_peer_id: donor.peer_id.clone(),
            donor_fault_domain: donor.fault_domain.clone(),
        });
        repaired.push(object.clone());
    }
    let completeness = if unrecoverable.is_empty() {
        RepairCompleteness::Complete
    } else {
        RepairCompleteness::Incomplete {
            unrecoverable: unrecoverable.clone(),
        }
    };
    let plan = ClosureRepairPlan {
        plan_ref: plan_ref(&required, &transfers, &completeness),
        completeness,
        transfers,
        repaired_objects: repaired,
        unrecoverable_objects: unrecoverable,
        issues: Vec::new(),
        non_claims: vec![
            "repair-recovered-bytes-are-not-authority".to_string(),
            "repair-does-not-establish-current-placement".to_string(),
        ],
    };
    Ok(plan)
}

/// Verify one assembled object against its established identity.
///
/// Verification stays on the receiving side. A mismatch rejects that object
/// only and never poisons the objects that verified.
pub fn verify_assembled_object(expected_ref: &str, assembled_ref: &str) -> Result<(), Issue> {
    if expected_ref != assembled_ref {
        return Err(Issue::InvalidReference);
    }
    Ok(())
}

/// Verify the chosen donor at transfer time.
///
/// A donor that flapped between the plan and the transfer no longer holds an
/// intact verifiable copy, so the transfer is refused.
pub fn verify_transfer_donor(transfer: &ClosureTransfer, fact: &ReplicaObjectFact) -> Result<(), Issue> {
    if transfer.object_ref != fact.object_ref || transfer.donor_peer_id != fact.peer_id {
        return Err(Issue::InvalidReference);
    }
    if !fact.is_intact_donor() {
        return Err(Issue::NoVerifiedSource);
    }
    Ok(())
}

fn validate_input(input: &ClosureRepairInput) -> Vec<Issue> {
    let mut issues = Vec::new();
    if input.required_objects.is_empty() {
        issues.push(Issue::InvalidManifest);
    }
    if input.required_objects.len() > MAX_REPAIR_OBJECTS {
        issues.push(Issue::TooManyContents);
    }
    if input.inventory.len() > MAX_REPAIR_INVENTORY {
        issues.push(Issue::TooManyPeers);
    }
    if input.max_transfers == 0 {
        issues.push(Issue::InvalidResourceLimit);
    }
    let mut seen = BTreeSet::new();
    for fact in &input.inventory {
        if fact.peer_id.trim().is_empty() || fact.object_ref.trim().is_empty() {
            issues.push(Issue::InvalidReference);
            continue;
        }
        if !seen.insert((fact.peer_id.as_str(), fact.object_ref.as_str())) {
            issues.push(Issue::DuplicateReplica);
        }
    }
    issues.sort();
    issues.dedup();
    issues
}

fn plan_ref(required: &[String], transfers: &[ClosureTransfer], completeness: &RepairCompleteness) -> String {
    crate::content_replication::identity::identify_repair_plan(required, transfers, completeness)
        .unwrap_or_else(|_| format!("blake3:{:064x}", 0))
}
