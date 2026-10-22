use molten_core::prolly_map::*;

use super::*;

// r[verify molten.prolly.publication.classification]
#[test]
fn a_staging_commit_failure_after_possible_durability_does_not_mark_the_publication_unknown() {
    let plan = initial_plan();
    let expected = ExpectedProllyRoot {
        root_ref: None,
        generation: INITIAL_GENERATION,
    };
    let mut port = MemoryPort::new(UnknownMode::StagingCommitUnknown);
    let receipt = publish_prolly_edit(&mut port, MAP_ID, &expected, &plan).expect("re-staged publication");
    assert_eq!(receipt.status, ProllyPublicationStatus::Applied);
    assert!(port.staging_attempted, "the staging commit failed once and was re-staged");
    assert!(port.blocks.contains_key(plan.staged_blocks[0].node_ref.as_str()));
}

// r[verify molten.prolly.publication.reconcile]
// r[verify molten.prolly.publication.tests]
#[test]
fn a_corrupt_readback_keeps_the_publication_unknown() {
    let plan = initial_plan();
    let expected = ExpectedProllyRoot {
        root_ref: None,
        generation: INITIAL_GENERATION,
    };
    let mut port = MemoryPort::new(UnknownMode::CorruptReadback);
    let receipt = publish_prolly_edit(&mut port, MAP_ID, &expected, &plan).expect("unknown publication");
    assert_eq!(receipt.status, ProllyPublicationStatus::Unknown);
}

/// Build a garbage collection plan with unreachable candidates.
fn plan_with_candidates() -> (MapSnapshot, GcPlan) {
    let profile = profile();
    let first = build_map(&profile, &entries()).expect("first build");
    let update = update_plan(&first.snapshot);
    let second = update.next.snapshot.clone();
    let all_nodes = first
        .snapshot
        .blocks
        .iter()
        .chain(second.blocks.iter())
        .map(|block| block.node_ref.clone())
        .collect::<Vec<_>>();
    let facts = local::merged_facts(&profile, &[&first.snapshot, &second]);
    let plan =
        plan_gc(&profile, &all_nodes, core::slice::from_ref(&second.root.top_node_ref), &[], &facts).expect("gc plan");
    assert!(!plan.candidate_unreachable.is_empty());
    (second, plan)
}

// r[verify molten.prolly.gc.currentness]
// r[verify molten.prolly.publication.tests]
#[test]
fn a_stale_admission_against_an_advanced_head_refuses_the_deletion() {
    let (second, plan) = plan_with_candidates();
    let mut port = MemoryPort::new(UnknownMode::None);
    let head = PublishedProllyRoot {
        root: second.root.clone(),
        generation: FIRST_GENERATION,
    };
    port.head = Some(head);
    for candidate in &plan.candidate_unreachable {
        port.blocks.insert(candidate.as_str().to_string(), b"staged".to_vec());
    }
    let inventory = port.retention_inventory().expect("inventory");
    assert_eq!(inventory.roots, plan.roots);

    // A later publication advances the head after the admission was built.
    port.head = Some(PublishedProllyRoot {
        root: ProllyRoot {
            schema: second.root.schema.clone(),
            profile_ref: second.root.profile_ref.clone(),
            top_node_ref: NodeRef::new("blake3:stale-admission-advanced-head".to_string()),
            height: second.root.height,
            entry_count: second.root.entry_count,
            root_ref: RootRef::new("blake3:stale-admission-advanced-root".to_string()),
        },
        generation: FIRST_GENERATION + 1,
    });
    port.retention_generation += 1;
    let error = execute_prolly_gc(&mut port, &plan, &ProllyDeletionAdmission {
        roots: plan.roots.clone(),
        pins: plan.pins.clone(),
        candidate_unreachable: plan.candidate_unreachable.clone(),
        generation_current: true,
        retention_policy_allows: true,
        deletion_authority_present: true,
        retention_generation: inventory.retention_generation,
    })
    .expect_err("a stale admission refuses the deletion");
    assert_eq!(error, ProllyServiceError::GcInventoryMismatch("published-roots-changed"));
    assert!(
        port.blocks.contains_key(plan.candidate_unreachable[0].as_str()),
        "the refused deletion left the candidate in place"
    );
}

// r[verify molten.prolly.gc.currentness]
#[test]
fn the_blocks_staged_for_an_in_progress_publication_are_protected() {
    let (second, plan) = plan_with_candidates();
    let mut port = MemoryPort::new(UnknownMode::None);
    port.head = Some(PublishedProllyRoot {
        root: second.root.clone(),
        generation: FIRST_GENERATION,
    });
    for candidate in &plan.candidate_unreachable {
        port.blocks.insert(candidate.as_str().to_string(), b"staged".to_vec());
    }
    let staged = plan.candidate_unreachable[0].clone();
    port.pin_blocks(std::slice::from_ref(&staged)).expect("pin the staged block");
    let inventory = port.retention_inventory().expect("inventory");
    let error = execute_prolly_gc(&mut port, &plan, &ProllyDeletionAdmission {
        roots: plan.roots.clone(),
        pins: plan.pins.clone(),
        candidate_unreachable: plan.candidate_unreachable.clone(),
        generation_current: true,
        retention_policy_allows: true,
        deletion_authority_present: true,
        retention_generation: inventory.retention_generation,
    })
    .expect_err("a pinned block is protected");
    assert_eq!(error, ProllyServiceError::GcInventoryMismatch("candidate-is-pinned"));
    assert!(port.blocks.contains_key(staged.as_str()), "the protected block survives the deletion");
}
