//! The replay capsule closure and its materialization port.

use crate::world_commit::WorldCommitPublicationPort;
use crate::world_commit::WorldImmutableObjectPort;
use crate::world_replay::WorldReplayMaterializationPort;

/// The capsule closure, served from the recorded world-commit store, the
/// recorded inputs and the boundary. Every member is re-identified from its
/// bytes; possession of these bytes grants no admission.
pub(super) struct CapsuleStore<'a> {
    pub(super) trace: &'a molten_core::world_replay::WorldTransitionTrace,
    pub(super) boundary: &'a molten_core::world_replay::WorldReplayObservationBoundary,
    pub(super) source: &'a crate::world_commit::LocalWorldCommitStore,
    pub(super) objects: std::collections::BTreeMap<String, Vec<u8>>,
    pub(super) materialized: usize,
}

impl CapsuleStore<'_> {
    /// The member bytes and the identity re-derived from them.
    fn payload(
        &self,
        object_ref: &str,
        roles: &[molten_core::world_replay::WorldReplayCapsuleMemberRole],
    ) -> crate::error::Result<(Vec<u8>, String)> {
        if roles.contains(&molten_core::world_replay::WorldReplayCapsuleMemberRole::Trace) {
            let record = crate::world_replay::canonical_world_transition_trace(self.trace)?;
            let identity = molten_core::world_replay::identify_world_transition_trace(self.trace)
                .map_err(|issue| super::super::invalid(format!("trace identity: {issue:?}")))?;
            return Ok((record.bytes, identity));
        }
        if roles.contains(&molten_core::world_replay::WorldReplayCapsuleMemberRole::WorldCommit) {
            let commit_ref = molten_core::world_commit::WorldCommitRef::new(object_ref.to_string())
                .map_err(|error| super::super::invalid(format!("capsule commit ref: {error:?}")))?;
            let bytes = self.source.read_commit(&commit_ref).map_err(super::super::runtime::store_error)?;
            let commit = crate::world_commit::parse_canonical_world_commit_with_ref(
                &bytes,
                &commit_ref,
                &super::super::runtime::commit_bounds(),
            )?;
            return Ok((bytes, commit.commit_ref.as_str().to_string()));
        }
        if roles.contains(&molten_core::world_replay::WorldReplayCapsuleMemberRole::SourceInventory) {
            let identity = molten_core::world_replay::identify_replay_source_inventory(&self.boundary.inventory)
                .map_err(|issue| super::super::invalid(format!("inventory identity: {issue:?}")))?;
            return Ok((super::history::inventory_bytes(&self.boundary.inventory)?, identity));
        }
        if roles.contains(&molten_core::world_replay::WorldReplayCapsuleMemberRole::ObservationLedger) {
            let identity = molten_core::world_replay::identify_replay_observation_ledger(&self.boundary.ledger)
                .map_err(|issue| super::super::invalid(format!("ledger identity: {issue:?}")))?;
            return Ok((super::history::ledger_bytes(&self.boundary.ledger)?, identity));
        }
        let root_kind = roles.iter().find_map(|role| match role {
            molten_core::world_replay::WorldReplayCapsuleMemberRole::TypedRoot(kind) => Some(*kind),
            _ => None,
        });
        let bytes = match root_kind {
            Some(kind) => self.root(kind, object_ref)?,
            None => self
                .objects
                .get(object_ref)
                .cloned()
                .ok_or_else(|| super::super::invalid("capsule member is absent from the recorded closure"))?,
        };
        let identity = crate::preserves_rail::content_ref_from_bytes(&bytes);
        Ok((bytes, identity))
    }

    fn root(&self, kind: molten_core::world_commit::RootKind, object_ref: &str) -> crate::error::Result<Vec<u8>> {
        let root = molten_core::world_commit::WorldRootRef::parse(kind, object_ref.to_string())
            .map_err(|error| super::super::invalid(format!("capsule root ref: {error:?}")))?;
        self.source.read_root(&root).map_err(super::super::runtime::store_error)
    }

    pub(super) fn capsule(
        &self,
        requirements: Vec<molten_core::world_replay::WorldReplayClosureRequirement>,
    ) -> crate::error::Result<molten_core::world_replay::WorldReplayCapsule> {
        let mut roles =
            std::collections::BTreeMap::<String, Vec<molten_core::world_replay::WorldReplayCapsuleMemberRole>>::new();
        for requirement in requirements {
            roles.entry(requirement.object_ref).or_default().push(requirement.role);
        }
        let mut members = Vec::with_capacity(roles.len());
        for (object_ref, mut member_roles) in roles {
            member_roles.sort();
            member_roles.dedup();
            let (bytes, _) = self.payload(&object_ref, &member_roles)?;
            members.push(molten_core::world_replay::WorldReplayCapsuleMember {
                object_ref,
                roles: member_roles,
                codec: molten_core::world_replay::WorldReplayMemberCodec::CanonicalPreservesV1,
                byte_length: u64::try_from(bytes.len())
                    .map_err(|_| super::super::invalid("capsule member exceeds u64"))?,
                protection: molten_core::world_replay::WorldReplayMemberProtection::Public,
            });
        }
        let mut capsule = molten_core::world_replay::WorldReplayCapsule {
            schema: molten_core::world_replay::WORLD_REPLAY_CAPSULE_SCHEMA.to_string(),
            capsule_ref: String::new(),
            trace_ref: self.trace.trace_ref.clone(),
            profile: self.trace.profile.clone(),
            members,
            non_claims: molten_core::world_replay::world_replay_non_claims(),
        };
        capsule.capsule_ref = molten_core::world_replay::identify_world_replay_capsule(&capsule)
            .map_err(|issue| super::super::invalid(format!("capsule identity: {issue:?}")))?;
        Ok(capsule)
    }
}

impl WorldReplayMaterializationPort for CapsuleStore<'_> {
    fn materialize(
        &mut self,
        member: &molten_core::world_replay::WorldReplayCapsuleMember,
    ) -> crate::error::Result<crate::world_replay::WorldReplayMaterializationObservation> {
        let (bytes, identity) = self.payload(&member.object_ref, &member.roles)?;
        let is_verified = identity == member.object_ref && u64::try_from(bytes.len()).ok() == Some(member.byte_length);
        if is_verified {
            self.materialized = self
                .materialized
                .checked_add(1)
                .ok_or_else(|| super::super::invalid("materialized member count overflowed"))?;
        }
        Ok(crate::world_replay::WorldReplayMaterializationObservation {
            object_ref: member.object_ref.clone(),
            observation_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "aspen-replay-member-materialized-v1",
                vec![
                    crate::preserves_rail::string(&member.object_ref),
                    crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(&bytes)),
                ],
            ))?,
            available: true,
            identity_verified: is_verified,
        })
    }
}
