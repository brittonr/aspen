//! Static, scripted, and in-memory membership mechanisms.
//!
//! r[impl molten.modularity.fabric_boundary.adapters]

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
pub struct StaticProvider {
    snapshot: MembershipProviderSnapshot,
}

impl StaticProvider {
    pub fn new(snapshot: MembershipProviderSnapshot) -> FabricPortResult<Self> {
        require_provider_kind(&snapshot, MembershipProviderKind::Static)?;
        Ok(Self { snapshot })
    }
}

impl MembershipPlacementProvider for StaticProvider {
    fn provider_kind(&self) -> MembershipProviderKind {
        MembershipProviderKind::Static
    }

    fn snapshot(&mut self) -> FabricPortResult<MembershipProviderSnapshot> {
        Ok(self.snapshot.clone())
    }
}

#[derive(Debug, Clone)]
pub struct PolicyManagedProvider {
    current: MembershipProviderSnapshot,
}

impl PolicyManagedProvider {
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

impl MembershipPlacementProvider for PolicyManagedProvider {
    fn provider_kind(&self) -> MembershipProviderKind {
        self.current.profile.provider_kind
    }

    fn snapshot(&mut self) -> FabricPortResult<MembershipProviderSnapshot> {
        Ok(self.current.clone())
    }
}

#[derive(Debug, Clone)]
pub struct DeterministicSimulationProvider {
    snapshots: Vec<MembershipProviderSnapshot>,
    cursor: usize,
}

impl DeterministicSimulationProvider {
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

impl MembershipPlacementProvider for DeterministicSimulationProvider {
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
