//! Stable-view admission over acknowledger and active-proposer subsets: exhaustive for five
//! replicas, bounded Hegel draws for seven.

const SEVEN_NODES: usize = 7;

fn is_member(mask: u32, index: usize) -> bool {
    mask & (1 << index) != 0
}

fn members(node_count: usize, mask: u32) -> alloc::collections::BTreeSet<alloc::string::String> {
    (0..node_count).filter(|index| is_member(mask, *index)).map(super::support::replica).collect()
}

/// Every replica in `acknowledger_mask` acknowledges; every replica in `proposer_mask` is an
/// order-preserving active proposer with a compatible promise.
fn subset_attempt(node_count: usize, acknowledger_mask: u32, proposer_mask: u32) -> crate::quorate::StableViewAttempt {
    let mut attempt = super::support::attempt(node_count);
    let acknowledgers = members(node_count, acknowledger_mask);
    attempt
        .acknowledgements
        .retain(|acknowledgement| acknowledgers.contains(&acknowledgement.replica_id));
    let template = attempt.promises[0].clone();
    attempt.active_proposers = members(node_count, proposer_mask);
    attempt.promises = attempt
        .active_proposers
        .iter()
        .map(|proposer| crate::quorate::ProposerPromise {
            proposer_id: proposer.clone(),
            ..template.clone()
        })
        .collect();
    attempt
}

/// Fast admission holds exactly when a superquorum acknowledged and contains every active proposer.
fn expected(node_count: usize, acknowledger_mask: u32, proposer_mask: u32) -> crate::quorate::StableViewDecision {
    let acknowledgers = members(node_count, acknowledger_mask);
    if acknowledgers.len() < super::support::quorums(node_count).superquorum {
        crate::quorate::StableViewDecision::Fallback(crate::quorate::FallbackReason::InsufficientAcknowledgements)
    } else if members(node_count, proposer_mask).is_subset(&acknowledgers) {
        crate::quorate::StableViewDecision::FastCommitted
    } else {
        crate::quorate::StableViewDecision::Fallback(crate::quorate::FallbackReason::MissingProposerAcknowledgement)
    }
}

fn check(node_count: usize, acknowledger_mask: u32, proposer_mask: u32) {
    let attempt = subset_attempt(node_count, acknowledger_mask, proposer_mask);
    assert_eq!(
        crate::quorate::evaluate_stable_view(&attempt, super::support::quorums(node_count)),
        expected(node_count, acknowledger_mask, proposer_mask),
        "replicas {node_count} acknowledgers {acknowledger_mask:#b} proposers {proposer_mask:#b}"
    );
}

/// All 32 acknowledger subsets of five replicas against all 31 non-empty active-proposer sets,
/// which include the 160 single-proposer and the 320 two-proposer combinations.
// r[verify molten.consensus.fast_path_model.stable_view]
// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[test]
fn every_five_replica_acknowledger_and_proposer_subset_admits_exactly_when_proposers_acknowledged() {
    let subsets = 1_u32 << super::support::FIVE_NODES;
    let pairs = (0..subsets).flat_map(|acknowledgers| (1..subsets).map(move |proposers| (acknowledgers, proposers)));
    for (acknowledger_mask, proposer_mask) in pairs.clone() {
        check(super::support::FIVE_NODES, acknowledger_mask, proposer_mask);
    }
    let fast_committed = pairs
        .filter(|(acknowledgers, proposers)| {
            expected(super::support::FIVE_NODES, *acknowledgers, *proposers)
                == crate::quorate::StableViewDecision::FastCommitted
        })
        .count();
    // Superquorums: the full set (31 proposer sets) and five 4-of-5 sets (15 proposer sets each).
    assert_eq!(fast_committed, 31 + 5 * 15);
}

/// Two active proposers: both must acknowledge, even when a superquorum has formed.
// r[verify molten.consensus.fast_path_model.stable_view]
#[test]
fn two_active_proposers_both_must_be_among_the_acknowledgers() {
    let quorums = super::support::quorums(super::support::FIVE_NODES);
    let both = 0b00011;
    let without_node_one = 0b11101;
    let with_both = 0b01111;
    assert_eq!(
        crate::quorate::evaluate_stable_view(
            &subset_attempt(super::support::FIVE_NODES, without_node_one, both),
            quorums
        ),
        crate::quorate::StableViewDecision::Fallback(crate::quorate::FallbackReason::MissingProposerAcknowledgement)
    );
    assert_eq!(
        crate::quorate::evaluate_stable_view(&subset_attempt(super::support::FIVE_NODES, with_both, both), quorums),
        crate::quorate::StableViewDecision::FastCommitted
    );
}

/// Seven replicas (superquorum 6) have 128 x 127 subset pairs; Hegel samples them.
// r[verify molten.consensus.fast_path_model.stable_view]
#[hegel::test(test_cases = 64)]
fn hegel_seven_replica_admission_requires_every_proposer_in_the_superquorum(tc: hegel::TestCase) {
    let subsets = 1_u32 << SEVEN_NODES;
    let acknowledger_mask = tc.draw(hegel::generators::integers::<u32>().min_value(0).max_value(subsets - 1));
    let proposer_mask = tc.draw(hegel::generators::integers::<u32>().min_value(1).max_value(subsets - 1));
    check(SEVEN_NODES, acknowledger_mask, proposer_mask);
}
