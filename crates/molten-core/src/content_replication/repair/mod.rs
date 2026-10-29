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
pub struct ClosureInput {
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
pub enum Completeness {
    /// Every required object has an intact verifiable copy.
    Complete,
    /// At least one required object has no intact verifiable copy.
    Incomplete { unrecoverable: Vec<String> },
}

impl Completeness {
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
pub struct ClosurePlan {
    pub plan_ref: String,
    pub completeness: Completeness,
    pub transfers: Vec<ClosureTransfer>,
    pub repaired_objects: Vec<String>,
    pub unrecoverable_objects: Vec<String>,
    pub issues: Vec<super::Issue>,
    pub non_claims: Vec<String>,
}

/// Plan a closure repair over the bounded inventory.
///
/// The plan is complete when every required object has an intact verifiable
/// copy. Otherwise the plan is a typed incomplete repair that names the
/// unrecoverable identities and keeps the transfers for the objects that do
/// have donors.
pub fn plan_closure(input: &ClosureInput) -> Result<ClosurePlan, super::Issue> {
    let issues = validate_input(input);
    if let Some(issue) = issues.first() {
        return Err(*issue);
    }
    let mut required = input.required_objects.clone();
    required.sort();
    required.dedup();
    let mut donors: std::collections::BTreeMap<&str, Vec<&ReplicaObjectFact>> = std::collections::BTreeMap::new();
    for fact in &input.inventory {
        donors.entry(fact.object_ref.as_str()).or_default().push(fact);
    }
    let mut transfers = Vec::with_capacity(required.len());
    let mut repaired = Vec::with_capacity(required.len());
    let mut unrecoverable = Vec::with_capacity(required.len());
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
        Completeness::Complete
    } else {
        Completeness::Incomplete {
            unrecoverable: unrecoverable.clone(),
        }
    };
    let plan = ClosurePlan {
        plan_ref: super::identity::identify_repair_plan(&required, &transfers, &completeness)?,
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
pub fn verify_assembled_object(expected_ref: &str, assembled_ref: &str) -> Result<(), super::Issue> {
    if expected_ref != assembled_ref {
        return Err(super::Issue::InvalidReference);
    }
    Ok(())
}

/// Verify the chosen donor at transfer time.
///
/// A donor that flapped between the plan and the transfer no longer holds an
/// intact verifiable copy, so the transfer is refused.
pub fn verify_transfer_donor(transfer: &ClosureTransfer, fact: &ReplicaObjectFact) -> Result<(), super::Issue> {
    if transfer.object_ref != fact.object_ref || transfer.donor_peer_id != fact.peer_id {
        return Err(super::Issue::InvalidReference);
    }
    if !fact.is_intact_donor() {
        return Err(super::Issue::NoVerifiedSource);
    }
    Ok(())
}

fn validate_input(input: &ClosureInput) -> Vec<super::Issue> {
    let mut issues = Vec::with_capacity(input.inventory.len().saturating_add(4));
    if input.required_objects.is_empty() {
        issues.push(super::Issue::InvalidManifest);
    }
    if input.required_objects.len() > MAX_REPAIR_OBJECTS {
        issues.push(super::Issue::TooManyContents);
    }
    if input.inventory.len() > MAX_REPAIR_INVENTORY {
        issues.push(super::Issue::TooManyPeers);
    }
    if input.max_transfers == 0 {
        issues.push(super::Issue::InvalidResourceLimit);
    }
    let mut seen = std::collections::BTreeSet::new();
    for fact in &input.inventory {
        if fact.peer_id.trim().is_empty() || fact.object_ref.trim().is_empty() {
            issues.push(super::Issue::InvalidReference);
            continue;
        }
        if !seen.insert((fact.peer_id.as_str(), fact.object_ref.as_str())) {
            issues.push(super::Issue::DuplicateReplica);
        }
    }
    issues.sort();
    issues.dedup();
    issues
}
