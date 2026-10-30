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
    std::fs::write(root.join("consumer").join(format!("operation-{generation}")), stage.operation_ref)?;
    let local = store.current(&super::domain::branch()?)?.ok_or("missing local head after commit")?;
    if local.generation != generation {
        return Err("pilot head generation differs after commit".into());
    }
    println!("committed generation={generation} receipt={receipt_ref}");
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
    input.operation_id = operation;
    let mut provider = super::index::open(root)?;
    // The actual public consumer read must challenge the independent current head,
    // rather than trust either locally stored generation or receipt.
    let admission =
        match molten::world_witness::observe_current_head(&store, &mut provider, molten::world_witness::ReadInput {
            claim: &canonical,
            input,
        }) {
            Ok(admission) => admission,
            Err(denial) => {
                let external = provider.query(0).map_err(|error| error.to_string())?;
                let committed = external.snapshot.committed.ok_or("external index has no committed head")?;
                if committed.generation <= local.generation {
                    return Err(denial.into());
                }
                println!(
                    "rollback-detected local_generation={} external_generation={} denial={denial}",
                    local.generation, committed.generation
                );
                return Ok(false);
            }
        };
    let highest = admission.highest.ok_or("independent read lacks highest")?;
    println!("current generation={} external_revision={}", local.generation, highest.checkpoint.revision);
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
