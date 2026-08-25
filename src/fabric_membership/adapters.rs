//! Static, scripted, and in-memory membership mechanisms.
//!
//! r[impl molten.modularity.fabric_boundary.adapters]

use std::collections::BTreeMap;

use super::*;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "membership mechanisms implement the application-owned typed port contract"
)]
use crate::fabric::FabricPortError;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "membership mechanisms implement the application-owned typed port contract"
)]
use crate::fabric::FabricPortResult;

#[derive(Debug, Clone)]
pub struct StaticMembershipProvider {
    snapshot: MembershipProviderSnapshot,
}

impl StaticMembershipProvider {
    pub fn new(snapshot: MembershipProviderSnapshot) -> FabricPortResult<Self> {
        require_provider_kind(&snapshot, MembershipProviderKind::Static)?;
        Ok(Self { snapshot })
    }
}

impl MembershipPlacementProvider for StaticMembershipProvider {
    fn provider_kind(&self) -> MembershipProviderKind {
        MembershipProviderKind::Static
    }

    fn snapshot(&mut self) -> FabricPortResult<MembershipProviderSnapshot> {
        Ok(self.snapshot.clone())
    }
}

#[derive(Debug, Clone)]
pub struct PolicyManagedMembershipProvider {
    current: MembershipProviderSnapshot,
}

impl PolicyManagedMembershipProvider {
    pub fn new(current: MembershipProviderSnapshot) -> FabricPortResult<Self> {
        if !matches!(
            current.profile.provider_kind,
            MembershipProviderKind::PolicyManaged | MembershipProviderKind::ConsistencyBacked
        ) {
            return Err(FabricPortError::malformed(
                "policy-managed provider requires a live policy or consistency profile",
            ));
        }
        Ok(Self { current })
    }

    pub fn replace_snapshot(&mut self, next: MembershipProviderSnapshot) -> FabricPortResult<()> {
        if next.profile.provider_kind != self.current.profile.provider_kind {
            return Err(FabricPortError::malformed("live provider kind cannot change during snapshot replacement"));
        }
        if next.profile.profile_ref != self.current.profile.profile_ref {
            return Err(FabricPortError::malformed("live provider profile cannot drift during snapshot replacement"));
        }
        if next.view.epoch <= self.current.view.epoch {
            return Err(FabricPortError::malformed("live membership view epoch must advance"));
        }
        self.current = next;
        Ok(())
    }
}

impl MembershipPlacementProvider for PolicyManagedMembershipProvider {
    fn provider_kind(&self) -> MembershipProviderKind {
        self.current.profile.provider_kind
    }

    fn snapshot(&mut self) -> FabricPortResult<MembershipProviderSnapshot> {
        Ok(self.current.clone())
    }
}

#[derive(Debug, Clone)]
pub struct DeterministicSimulationMembershipProvider {
    snapshots: Vec<MembershipProviderSnapshot>,
    cursor: usize,
}

impl DeterministicSimulationMembershipProvider {
    pub fn new(snapshots: Vec<MembershipProviderSnapshot>) -> FabricPortResult<Self> {
        if snapshots.is_empty() {
            return Err(FabricPortError::malformed("deterministic membership provider requires at least one snapshot"));
        }
        let mut previous_epoch = None;
        let expected_profile_ref = snapshots[0].profile.profile_ref.clone();
        let expected_authority_scope = snapshots[0].profile.authority_scope.clone();
        for snapshot in &snapshots {
            require_provider_kind(snapshot, MembershipProviderKind::DeterministicSimulation)?;
            if snapshot.profile.profile_ref != expected_profile_ref
                || snapshot.profile.authority_scope != expected_authority_scope
            {
                return Err(FabricPortError::malformed(
                    "deterministic membership snapshots cannot drift source profile or authority scope",
                ));
            }
            if previous_epoch.is_some_and(|epoch| snapshot.view.epoch <= epoch) {
                return Err(FabricPortError::malformed(
                    "deterministic membership snapshots must have increasing epochs",
                ));
            }
            previous_epoch = Some(snapshot.view.epoch);
        }
        Ok(Self { snapshots, cursor: 0 })
    }
}

impl MembershipPlacementProvider for DeterministicSimulationMembershipProvider {
    fn provider_kind(&self) -> MembershipProviderKind {
        MembershipProviderKind::DeterministicSimulation
    }

    fn snapshot(&mut self) -> FabricPortResult<MembershipProviderSnapshot> {
        let snapshot = self
            .snapshots
            .get(self.cursor)
            .ok_or_else(|| FabricPortError::capability("deterministic membership snapshot stream is exhausted"))?
            .clone();
        self.cursor = self
            .cursor
            .checked_add(1)
            .ok_or_else(|| FabricPortError::malformed("deterministic membership snapshot cursor overflow"))?;
        Ok(snapshot)
    }
}

#[derive(Debug, Default)]
pub struct InMemoryAssignmentPersistence {
    pub assignments: BTreeMap<String, RoleAssignment>,
    pub intents: Vec<String>,
    pub commits: Vec<String>,
    pub fail_intent: bool,
    pub fail_commit: bool,
}

impl AssignmentPersistence for InMemoryAssignmentPersistence {
    fn record_intent(
        &mut self,
        current: &RoleAssignment,
        transition: &AssignmentTransition,
    ) -> FabricPortResult<String> {
        if self.fail_intent {
            return Err(FabricPortError::storage("injected assignment intent failure"));
        }
        let value = crate::preserves_rail::record("fabric-assignment-intent-v1", vec![
            crate::preserves_rail::string(&current.assignment_id),
            crate::preserves_rail::string(current.state.as_str()),
            crate::preserves_rail::string(transition.next.state.as_str()),
            crate::preserves_rail::u64_value(current.assignment_epoch),
        ]);
        let intent_ref = crate::preserves_rail::canonical_hash(&value).map_err(FabricPortError::from)?;
        self.intents.push(intent_ref.clone());
        Ok(intent_ref)
    }

    fn commit(
        &mut self,
        transition: &AssignmentTransition,
        intent_ref: &str,
        role_effect_ref: Option<&str>,
    ) -> FabricPortResult<String> {
        if self.fail_commit {
            return Err(FabricPortError::uncertain("injected assignment commit failure"));
        }
        let value = crate::preserves_rail::record("fabric-assignment-commit-v1", vec![
            crate::preserves_rail::string(intent_ref),
            crate::preserves_rail::string(role_effect_ref.unwrap_or("no-role-effect")),
            crate::preserves_rail::string(&transition.next.assignment_id),
            crate::preserves_rail::string(transition.next.state.as_str()),
        ]);
        let commit_ref = crate::preserves_rail::canonical_hash(&value).map_err(FabricPortError::from)?;
        self.assignments.insert(transition.next.assignment_id.clone(), transition.next.clone());
        self.commits.push(commit_ref.clone());
        Ok(commit_ref)
    }
}

fn require_provider_kind(
    snapshot: &MembershipProviderSnapshot,
    expected: MembershipProviderKind,
) -> FabricPortResult<()> {
    if snapshot.profile.provider_kind == expected {
        Ok(())
    } else {
        Err(FabricPortError::malformed(format!(
            "membership provider profile kind {} does not match adapter {}",
            snapshot.profile.provider_kind.as_str(),
            expected.as_str()
        )))
    }
}
