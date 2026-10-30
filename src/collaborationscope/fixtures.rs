use super::*;

pub(crate) const POLICY: &str = r#"{"backends":{},"contracts":{"collaboration-scope":{"abilities":["resource/read","resource/use","resource/share","resource/bind","resource/move","scope/create","scope/membership"],"description":"Bounded collaboration fixture","id":"collaboration-scope","resource_prefixes":["molten://collaboration/"]}},"schema_version":"ucan-nickel-contracts.policy.v1"}"#;

pub(crate) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()))
}
pub(crate) fn space_ref() -> ScopeRef {
    ScopeRef::new(reference("live-collaboration-scope")).expect("scope")
}

pub(crate) fn open_store(root: &std::path::Path) -> crate::error::Result<Database> {
    let (adapter, domain) = open_adapter(root)?;
    Database::new(adapter, domain)
}

/// Reopen a store that a crashed process held open. Redb repairs such a store on
/// open, which quarantines it; the restart reconciles that repair explicitly by
/// the store's own consistency before the store may serve.
pub(crate) fn recover_store(root: &std::path::Path) -> crate::error::Result<Database> {
    let (adapter, domain) = open_adapter(root)?;
    Database::new(adapter.reconcile_repaired_on_open()?, domain)
}

fn open_adapter(
    root: &std::path::Path,
) -> crate::error::Result<(crate::fabric_durability::RedbDurableStateAdapter, crate::fabric_durability::AtomicityDomain)>
{
    let mut profile =
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::LiveRedb).profile;
    profile.max_operation_bytes = MAX_RECORD_BYTES;
    profile.max_namespace_bytes = MAX_RECORD_BYTES * 8;
    profile.max_ordered_entries = 128;
    let profile = crate::fabric_durability::durable_profile(&profile)?;
    let mut descriptor = crate::fabric_durability::tests::descriptor();
    descriptor.atomicity_domain.max_bytes = MAX_RECORD_BYTES;
    descriptor.quota_bytes = MAX_RECORD_BYTES * 8;
    let domain = descriptor.atomicity_domain.clone();
    Ok((crate::fabric_durability::RedbDurableStateAdapter::open(root, profile, descriptor)?, domain))
}

pub(crate) fn open_application(
    root: &std::path::Path,
    values_root: &std::path::Path,
) -> crate::error::Result<Application> {
    application(open_store(root)?, values_root)
}

pub(crate) fn recover_application(
    root: &std::path::Path,
    values_root: &std::path::Path,
) -> crate::error::Result<Application> {
    application(recover_store(root)?, values_root)
}

fn application(mut store: Database, values_root: &std::path::Path) -> crate::error::Result<Application> {
    if store.load()?.is_none() {
        store.initialize(&initial_state()?)?;
    }
    let values = crate::system_extension::CapabilityCallbackValuePort::new(crate::node_state::Scope::open(
        crate::node_state::NamespaceKind::Ledger,
        values_root,
    )?)?;
    Application::open(store, Box::new(values))
}

pub(crate) fn initial_state() -> crate::error::Result<State> {
    let authority = Signers::new();
    let owner = subject_ref(&authority.issuer_ref(), authority.audience.as_str())?;
    let decision = DecisionRef::new(reference("declared-initial-membership")).map_err(|_| denied(Denial::Domain))?;
    let operation =
        molten_core::nominal::OperationRef::new("initial-membership").map_err(|_| denied(Denial::Domain))?;
    let member = Membership {
        reference: MembershipRef::new(reference("initial-member")).map_err(|_| denied(Denial::Domain))?,
        subject: owner.clone(),
        role: MembershipRole::Owner,
        abilities: [
            Ability::Read,
            Ability::Use,
            Ability::Share,
            Ability::Membership,
            Ability::Create,
            Ability::Bind,
            Ability::Move,
        ]
        .into_iter()
        .collect(),
        start_epoch: 1,
        expires_at: None,
        decision,
        operation,
    };
    let scope = CollaborationScope {
        reference: space_ref(),
        kind: ScopeKind::Group,
        owner: owner.clone(),
        revision: 1,
        membership_epoch: 1,
        policy: policy_ref(POLICY)?,
        organization_floor: None,
        members: [(owner, member)].into_iter().collect(),
    };
    let resource = molten_core::nominal::ResourceRef::new(reference("initial-scoped-resource"))
        .map_err(|_| denied(Denial::Domain))?;
    let binding = ScopedResourceBinding {
        reference: BindingRef::new(reference("initial-resource-binding")).map_err(|_| denied(Denial::Domain))?,
        resource: resource.clone(),
        owner_scope: scope.reference.clone(),
        owner: scope.owner.clone(),
        revision: 1,
        policy: scope.policy.clone(),
        required_abilities: [Ability::Read, Ability::Use, Ability::Share, Ability::Move].into_iter().collect(),
        operation: molten_core::nominal::OperationRef::new("initial-binding").map_err(|_| denied(Denial::Domain))?,
        decision: DecisionRef::new(reference("initial-binding-evidence")).map_err(|_| denied(Denial::Domain))?,
    };
    Ok(State {
        scopes: [(scope.reference.clone(), scope)].into_iter().collect(),
        bindings: [(resource, binding)].into_iter().collect(),
        operations: Default::default(),
        currentness: Currentness {
            service_generation: 1,
            consistency_epoch: 1,
            committed_index: 0,
            logical_now: 0,
            revocation: molten_core::nominal::RevocationRef::new(reference("no-revocations-input"))
                .map_err(|_| denied(Denial::Domain))?,
            observation: molten_core::nominal::EvidenceRef::new(reference("initial-currentness"))
                .map_err(|_| denied(Denial::Domain))?,
        },
    })
}

static EMPTY_PROOFS: basalt::ucan::ProofCollection = basalt::ucan::ProofCollection::empty();
struct NoCaveats;
impl basalt::ucan::CaveatPolicySet for NoCaveats {
    fn policy_for(&self, _: &basalt::ucan::CaveatIdentifier) -> Option<&dyn basalt::ucan::CaveatPolicy> {
        None
    }
}

pub(crate) fn add_member(state: &State, label: &str) -> crate::error::Result<Mutation> {
    let scope = state.scopes.get(&space_ref()).ok_or_else(|| denied(Denial::Missing))?;
    let member = Membership {
        reference: MembershipRef::new(reference(label)).map_err(|_| denied(Denial::Domain))?,
        subject: SubjectRef::new(reference(&format!("subject-{label}"))).map_err(|_| denied(Denial::Domain))?,
        role: MembershipRole::Participant,
        abilities: [Ability::Read].into_iter().collect(),
        start_epoch: scope.membership_epoch + 1,
        expires_at: Some(5),
        decision: DecisionRef::new(reference("requested-membership-evidence")).map_err(|_| denied(Denial::Domain))?,
        operation: molten_core::nominal::OperationRef::new(format!("operation-{}", state.currentness.committed_index))
            .map_err(|_| denied(Denial::Domain))?,
    };
    Ok(Mutation::PutMember {
        scope: space_ref(),
        member,
    })
}

pub(crate) fn current_from_replica(
    state: &crate::fabric_consistency::raft::ReplicaState,
    ledger: &crate::fabric_consistency::raft::ReplicaLedger,
    observation: &crate::fabric_consistency::raft::ReplicaObservation,
    application: &Application,
) -> crate::error::Result<CurrentState> {
    let crate::fabric_consistency::raft::ReplicaObservationKind::Read {
        mode,
        disposition,
        observed_index,
        ..
    } = &observation.kind
    else {
        return Err(denied(Denial::Partition));
    };
    let proof = ledger
        .records()
        .iter()
        .rev()
        .find(|record| {
            record.kind == crate::fabric_consistency::raft::ReplicaKind::ReadCurrentness
                && record.source_ref == observation.receipt_ref
        })
        .ok_or_else(|| denied(Denial::Partition))?;
    if *mode != crate::fabric_consistency::ReadMode::Linearizable
        || *disposition != crate::fabric_consistency::raft::ReadDisposition::Current
        || *observed_index != state.last_applied
        || state.last_applied != state.commit_index
        || proof.term != state.current_term
        || proof.index != *observed_index
        || proof.quorum_evidence_ref.is_none()
        || proof.quorum_members.len() < 2
    {
        return Err(denied(Denial::Partition));
    }
    Ok(CurrentState {
        state: application.state_ref().clone(),
        currentness: Currentness {
            service_generation: state.profile.service_generation,
            consistency_epoch: state.current_term,
            committed_index: *observed_index,
            logical_now: *observed_index,
            observation: molten_core::nominal::EvidenceRef::new(observation.receipt_ref.clone())
                .map_err(|_| denied(Denial::Domain))?,
            revocation: application.state().currentness.revocation.clone(),
        },
    })
}

#[path = "fixtures/audience.rs"]
mod audience;

#[path = "fixtures/authority.rs"]
mod authority;
pub(crate) use authority::*;
