use super::shared::BoundaryIssue;
use super::shared::EvidenceProfile;

// r[verify molten.testing.chaoscontrol_smr_claim_boundary]
#[test]
fn both_profiles_share_one_corpus_and_invariant_set() {
    let simulation = super::shared::claim(EvidenceProfile::WholeSystemSimulation).expect("simulation claim");
    let kvm = super::shared::claim(EvidenceProfile::ChaosControlKvm).expect("kvm claim");
    assert_eq!(super::shared::compare(&simulation, &kvm), Ok(()));
    assert_eq!(super::shared::compare(&kvm, &simulation), Ok(()));
    assert_eq!(simulation.operation_corpus_ref, kvm.operation_corpus_ref);
    assert_eq!(simulation.invariant_ids, kvm.invariant_ids);
    assert_ne!(simulation.profile.as_str(), kvm.profile.as_str());
}

#[test]
fn one_profile_cannot_substitute_for_the_other() {
    let simulation = super::shared::claim(EvidenceProfile::WholeSystemSimulation).expect("simulation claim");
    let other = super::shared::claim(EvidenceProfile::WholeSystemSimulation).expect("simulation claim");
    assert_eq!(super::shared::compare(&simulation, &other), Err(BoundaryIssue::ProfileSubstitution));
}

#[test]
fn corpus_and_invariant_drift_are_denied() {
    let simulation = super::shared::claim(EvidenceProfile::WholeSystemSimulation).expect("simulation claim");
    let mut kvm = super::shared::claim(EvidenceProfile::ChaosControlKvm).expect("kvm claim");
    kvm.operation_corpus_ref = super::testfixtures::id("foreign-corpus");
    assert_eq!(super::shared::compare(&simulation, &kvm), Err(BoundaryIssue::Corpus));

    let mut kvm = super::shared::claim(EvidenceProfile::ChaosControlKvm).expect("kvm claim");
    kvm.invariant_ids.pop();
    assert_eq!(super::shared::compare(&simulation, &kvm), Err(BoundaryIssue::Invariants));
}

#[test]
fn arbitrary_or_reordered_invariants_do_not_define_a_shared_claim() {
    let simulation = super::shared::claim(EvidenceProfile::WholeSystemSimulation).expect("simulation claim");
    let mut kvm = super::shared::claim(EvidenceProfile::ChaosControlKvm).expect("kvm claim");
    kvm.invariant_ids.push("universal-consensus-correctness".into());
    assert_eq!(super::shared::compare(&simulation, &kvm), Err(BoundaryIssue::Invariants));

    let mut reordered = super::shared::claim(EvidenceProfile::ChaosControlKvm).expect("kvm claim");
    reordered.invariant_ids.reverse();
    assert_eq!(super::shared::compare(&simulation, &reordered), Ok(()));
}

#[test]
fn corpus_identity_is_canonical_and_profile_bounded() {
    let first = super::shared::corpus_ref().expect("corpus ref");
    let second = super::shared::corpus_ref().expect("corpus ref");
    assert_eq!(first, second);
    assert!(first.starts_with("blake3:"));
    assert_eq!(first.len(), "blake3:".len() + 64);
}
