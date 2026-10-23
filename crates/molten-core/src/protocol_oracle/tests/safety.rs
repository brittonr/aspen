use super::*;

// r[verify molten.fabric_simulation.protocol_safety]
#[test]
fn replicas_that_diverge_and_later_converge_keep_the_first_pairwise_failure() {
    let profile = profile();
    let first_a = projection("node-a", 0, 0, &["a"]);
    let diverged_b = projection("node-b", 0, 1, &["z"]);
    let converged_a = projection("node-a", 1, 2, &["z", "c"]);
    let converged_b = projection("node-b", 1, 3, &["z", "c"]);
    let cohort =
        assemble_cohort(&profile, vec![first_a.clone(), diverged_b.clone(), converged_a, converged_b]).expect("cohort");
    let results = evaluate_safety(&profile, &cohort, &NoLocalRules);
    let OracleVerdict::Fail(witness) = verdict(&results, SafetyLevel::Pairwise) else {
        panic!("the earlier disagreement fails: {results:?}");
    };
    assert_eq!(witness.run_sequence, 1);
    assert_eq!(witness.projection_ref, projection_ref(&diverged_b));
    assert_eq!(witness.other_projection_ref, Some(projection_ref(&first_a)));
    assert_eq!(witness.scheduler_choice_ref, diverged_b.scheduler_choice_ref);
    assert!(matches!(verdict(&results, SafetyLevel::Cohort), OracleVerdict::Fail(_)));
}

// r[verify molten.fabric_simulation.protocol_safety]
#[test]
fn semantic_agreement_passes_across_physical_layouts_unless_the_profile_promises_one() {
    let mut first = projection("node-a", 0, 0, &["a"]);
    first.physical_ref = Some(content("layout-a"));
    let mut second = projection("node-b", 0, 1, &["a"]);
    second.physical_ref = Some(content("layout-b"));
    let semantic = profile();
    let cohort = assemble_cohort(&semantic, vec![first.clone(), second.clone()]).expect("cohort");
    assert_eq!(
        verdict(&evaluate_safety(&semantic, &cohort, &NoLocalRules), SafetyLevel::Pairwise),
        OracleVerdict::Pass
    );

    let mut physical = semantic.clone();
    physical.services[0].physical_layout = PhysicalLayout::CanonicalPhysical;
    let cohort = assemble_cohort(&physical, vec![first, second]).expect("cohort");
    assert!(matches!(
        verdict(&evaluate_safety(&physical, &cohort, &NoLocalRules), SafetyLevel::Pairwise),
        OracleVerdict::Fail(_)
    ));
}

// r[verify molten.fabric_simulation.protocol_safety]
#[test]
fn a_recovered_state_must_be_one_the_participant_held_at_that_position() {
    let profile = profile();
    let mut single = profile.clone();
    single.participants.truncate(1);
    let held = projection("node-a", 0, 0, &["a"]);
    let mut recovered = projection("node-a", 1, 1, &["a"]);
    recovered.durability = DurabilityFact::Recovered;
    recovered.transition_class = TransitionClass::Recovered;
    let cohort = assemble_cohort(&single, vec![held.clone(), recovered.clone()]).expect("cohort");
    assert_eq!(
        verdict(&evaluate_safety(&single, &cohort, &NoLocalRules), SafetyLevel::Durability),
        OracleVerdict::Pass
    );

    let mut corrupted = projection("node-a", 1, 1, &["x"]);
    corrupted.durability = DurabilityFact::Recovered;
    let cohort = assemble_cohort(&single, vec![held.clone(), corrupted]).expect("cohort");
    assert!(matches!(
        verdict(&evaluate_safety(&single, &cohort, &NoLocalRules), SafetyLevel::Durability),
        OracleVerdict::Fail(_)
    ));

    let mut unreferenced = projection("node-a", 1, 1, &["a", "b"]);
    unreferenced.durability = DurabilityFact::Recovered;
    let cohort = assemble_cohort(&single, vec![held.clone(), unreferenced]).expect("cohort");
    assert_eq!(
        verdict(&evaluate_safety(&single, &cohort, &NoLocalRules), SafetyLevel::Durability),
        OracleVerdict::Incomplete,
        "a recovery to a position never observed has no reference"
    );

    let cohort = assemble_cohort(&single, vec![held]).expect("cohort");
    let results = evaluate_safety(&single, &cohort, &NoLocalRules);
    assert_eq!(verdict(&results, SafetyLevel::Durability), OracleVerdict::NotEvaluated);
    assert_eq!(verdict(&results, SafetyLevel::Pairwise), OracleVerdict::NotEvaluated, "one replica has no pair");
}
