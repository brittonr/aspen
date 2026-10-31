pub fn advance(root: &std::path::Path, generation: u64) -> super::Result<()> {
    let consumer = molten::node_state::DirectoryRoot::open_existing(&root.join("consumer"))?;
    let mut head_store = molten::world_head::LocalWorldHeadStore::open(
        &consumer.namespace(molten::node_state::NamespaceKind::Storage)?,
    )?;
    let mut store = molten::world_witness::Database::new(&mut head_store)?;
    let canonical = molten::world_head::canonical_world_head_claim(&super::domain::claim(generation)?)?;
    let observed = store.current(&super::domain::branch()?)?;
    let mut authority = super::authority::open(&root.join("operator"))?;
    let admission = authority.observe(&canonical, observed.as_ref())?;
    let decision =
        molten_core::world_head::plan_world_head_transition(&molten_core::world_head::WorldHeadPlanRequest {
            claim_ref: canonical.claim_ref.clone(),
            claim: canonical.claim.clone(),
            current: observed.clone(),
            history: super::domain::history()?,
            policy: super::domain::policy()?,
            authentication: admission.authentication.observation.clone(),
            authority: admission.authority.clone(),
            currentness: molten_core::world_head::WorldHeadCurrentnessObservation {
                durable_generation_observed: true,
                independent_ref: None,
            },
            bounds: molten_core::world_head::WorldHeadBounds::standard(),
        });
    let molten_core::world_head::WorldHeadDecision::Admitted(plan) = decision else {
        return Err(format!("actual authenticated and authorized pilot planning denied: {decision:?}").into());
    };
    let input = currentness(root, &canonical, generation - 1)?;
    let mut input = molten::world_witness::Transition {
        claim: &canonical,
        plan: &plan,
        authentication_ref: admission.authentication_ref(),
        authority_ref: admission.authority_ref(),
        currentness: input,
    };
    input.currentness.retained = store.retained(&super::domain::branch()?)?;
    let stage = molten::world_witness::Stage::prepare(&input)?;
    let mut provider = super::index::open(root)?;
    let outcome = molten::world_witness::advance(&mut store, &mut provider, input, &mut authority)?;
    let molten::world_witness::Outcome::Committed { receipt_ref } = outcome else {
        return Err(format!("pilot did not confirm external and local completion: {outcome:?}").into());
    };
    let bytes = store.reservation(&stage.operation_ref)?.ok_or("committed head lacks its external reservation")?;
    let reserved: molten::world_witness::provider::Reservation = serde_json::from_slice(&bytes)?;
    std::fs::write(root.join("consumer").join(format!("operation-{generation}")), stage.operation_ref)?;
    let local = store.current(&super::domain::branch()?)?.ok_or("missing local head after commit")?;
    if local.generation != generation {
        return Err("pilot head generation differs after commit".into());
    }
    println!(
        "committed generation={generation} external_ordinal={} receipt={receipt_ref}",
        reserved.claim.generation
    );
    Ok(())
}

pub fn check(root: &std::path::Path) -> super::Result<bool> {
    let consumer = molten::node_state::DirectoryRoot::open_existing(&root.join("consumer"))?;
    let mut head_store = molten::world_head::LocalWorldHeadStore::open(
        &consumer.namespace(molten::node_state::NamespaceKind::Storage)?,
    )?;
    let store = molten::world_witness::Database::new(&mut head_store)?;
    let local = store.current(&super::domain::branch()?)?.ok_or("pilot head absent")?;
    let canonical = molten::world_head::canonical_world_head_claim(&super::domain::claim(local.generation)?)?;
    let operation = std::fs::read_to_string(root.join("consumer").join(format!("operation-{}", local.generation)))?;
    let mut input = currentness(root, &canonical, local.generation)?;
    let bytes = store.reservation(&operation)?.ok_or("local head lacks its external reservation")?;
    let reserved: molten::world_witness::provider::Reservation = serde_json::from_slice(&bytes)?;
    input.operation_id = operation;
    let mut provider = super::index::open(root)?;
    // The actual public consumer read must challenge the independent current head,
    // rather than trust either locally stored generation or receipt.
    let admission = match molten::world_witness::observe_current_head(
        &store,
        &mut provider,
        molten::world_witness::ReadInput {
            claim: &canonical,
            input,
        },
    ) {
        Ok(admission) => admission,
        Err(denial) => {
            let external = provider.query(0)?;
            let committed = external.snapshot.committed.ok_or("external index has no committed head")?;
            // Only an authenticated, strictly newer external claim identifies rollback;
            // an arbitrary read failure remains an error, never a rollback verdict.
            if committed == reserved.claim || committed.generation <= reserved.claim.generation {
                return Err(denial.into());
            }
            println!(
                "rollback-detected local_generation={} local_external_ordinal={} external_ordinal={} denial={denial}",
                local.generation, reserved.claim.generation, committed.generation
            );
            return Ok(false);
        }
    };
    let highest = admission.highest.ok_or("independent read lacks highest")?;
    println!(
        "current generation={} external_ordinal={} external_revision={}",
        local.generation, reserved.claim.generation, highest.checkpoint.revision
    );
    Ok(true)
}

fn currentness(
    root: &std::path::Path,
    claim: &molten::world_head::CanonicalWorldHeadClaim,
    local_generation: u64,
) -> super::Result<molten_core::world_witness::CurrentnessInput> {
    let revision = std::fs::read_to_string(root.join("provider-revision"))?;
    Ok(molten_core::world_witness::CurrentnessInput {
        selected_profile: molten_core::world_witness::AssuranceProfile::IndependentWitnessV1,
        policy: super::index::policy(revision)?,
        operation_id: "not-yet-staged".into(),
        subject: molten_core::world_witness::WitnessSubject {
            branch_id: claim.claim.branch_id.clone(),
            claim_ref: claim.claim_ref.clone(),
            generation: claim.claim.successor_generation,
        },
        local_generation,
        retained: molten_core::world_witness::RetainedWitnessState::Empty,
        observation: None,
        now: super::domain::now()?,
    })
}

const RESERVATION_FILE: &str = "abandoned-reservation.json";

fn local(
    store: &molten::world_witness::Database<'_>,
) -> super::Result<(molten_core::world_head::WorldHeadState, molten_core::world_witness::HighestWitnessState)> {
    let branch = super::domain::branch()?;
    let head = store.current(&branch)?.ok_or("pilot head absent")?;
    let molten_core::world_witness::RetainedWitnessState::Present(highest) = store.retained(&branch)? else {
        return Err("abandoned competitor requires retained witness state".into());
    };
    Ok((head, highest))
}

/// Reserves a competing successor externally, then orphans it. The consumer never stages
/// that claim, so its local compare-and-swap provably did not commit.
pub fn abandon(root: &std::path::Path) -> super::Result<()> {
    let consumer = molten::node_state::DirectoryRoot::open_existing(&root.join("consumer"))?;
    let mut head_store = molten::world_head::LocalWorldHeadStore::open(
        &consumer.namespace(molten::node_state::NamespaceKind::Storage)?,
    )?;
    let store = molten::world_witness::Database::new(&mut head_store)?;
    let (head, highest) = local(&store)?;
    let generation = head.generation.checked_add(1).ok_or("pilot generation overflow")?;
    let competitor = molten::world_head::canonical_world_head_claim(&super::domain::abandoned_claim(generation)?)?;
    let candidate = molten::world_witness::provider::ordinal::Candidate {
        digest: molten::world_witness::provider::normalize::claim_digest(&competitor.claim_ref)?,
        predecessor: Some(molten::world_witness::provider::normalize::claim_digest(&highest.subject.claim_ref)?),
    };
    let mut provider = super::index::open(root)?;
    let observation = provider.query(highest.checkpoint.revision)?;
    let planned = molten::world_witness::provider::ordinal::plan(&observation.snapshot, &candidate)?;
    let reservation = provider.reserve(observation.snapshot.revision, planned)?;
    let (after, retained) = local(&store)?;
    if after != head || retained != highest {
        return Err("consumer state changed while the competitor was reserved".into());
    }
    let orphan = provider.orphan(&reservation)?;
    std::fs::write(root.join(RESERVATION_FILE), serde_json::to_vec(&reservation)?)?;
    println!(
        "abandoned-orphan head_generation={generation} external_ordinal={} external_revision={}",
        reservation.claim.generation, orphan.snapshot.revision
    );
    Ok(())
}

/// Explicit operator resolution; the retained external high-water is never erased.
pub fn resolve(root: &std::path::Path) -> super::Result<()> {
    let reservation: molten::world_witness::provider::Reservation =
        serde_json::from_slice(&std::fs::read(root.join(RESERVATION_FILE))?)?;
    let mut provider = super::index::open(root)?;
    let resolved = provider.resolve_orphan(&reservation)?;
    let highest = resolved.snapshot.highest.ok_or("resolution erased the external high-water")?;
    let committed = resolved.snapshot.committed.ok_or("resolution lacks the committed predecessor")?;
    if highest != reservation.claim {
        return Err("resolution changed the retained external high-water".into());
    }
    println!(
        "resolved-orphan external_high_water={} external_committed={} external_revision={}",
        highest.generation, committed.generation, resolved.snapshot.revision
    );
    Ok(())
}
