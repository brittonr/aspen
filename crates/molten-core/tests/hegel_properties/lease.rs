//! Object-store compare-and-swap leases: fencing epochs and exact compare-and-swap admission.

const OWNERS: [&str; 4] = ["node-a", "node-b", "node-c", " "];
const MAX_ATTEMPTS: usize = 24;
const MAX_EPOCH_STEP: u64 = 3;

fn draw_owner(tc: &hegel::TestCase) -> String {
    let index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(OWNERS.len() - 1));
    OWNERS[index].to_string()
}

fn is_valid_owner(owner: &str) -> bool {
    !owner.trim().is_empty()
}

fn draw_attempt(
    tc: &hegel::TestCase,
    current: &molten_core::fabric_durability::ownership::CasLease,
) -> molten_core::fabric_durability::ownership::CasLeaseDecisionInput {
    // Contenders read either the true current snapshot or a stale/forged one.
    let expected = if tc.draw(hegel::generators::booleans()) {
        current.clone()
    } else {
        molten_core::fabric_durability::ownership::CasLease {
            owner: draw_owner(tc),
            epoch: current
                .epoch
                .saturating_sub(tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(MAX_EPOCH_STEP))),
        }
    };
    let base_epoch = current.epoch.saturating_sub(1);
    let proposed = molten_core::fabric_durability::ownership::CasLease {
        owner: draw_owner(tc),
        epoch: base_epoch + tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(MAX_EPOCH_STEP)),
    };
    let membership = if tc.draw(hegel::generators::integers::<u8>().min_value(0).max_value(7)) == 0 {
        molten_core::fabric_durability::ownership::MembershipPosture::FixedMembership
    } else {
        molten_core::fabric_durability::ownership::MembershipPosture::ReplaceableNodes
    };
    molten_core::fabric_durability::ownership::CasLeaseDecisionInput {
        current: current.clone(),
        expected,
        proposed,
        membership,
    }
}

// Across any contention history, the lease changes only when the compare matches exactly and the
// proposed fencing epoch strictly advances; epochs never regress and a rejection mutates nothing.
#[hegel::test(test_cases = 150, derandomize = true, database = None)]
fn hegel_cas_lease_epochs_advance_only_on_exact_compare(tc: hegel::TestCase) {
    let mut current = molten_core::fabric_durability::ownership::CasLease {
        owner: "node-a".to_string(),
        epoch: tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(1_000)),
    };
    let attempts = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(MAX_ATTEMPTS));
    for _ in 0..attempts {
        let input = draw_attempt(&tc, &current);
        let decision = molten_core::fabric_durability::ownership::decide_cas_lease(&input);
        let has_valid_owners =
            [&input.current, &input.expected, &input.proposed].iter().all(|lease| is_valid_owner(&lease.owner));
        let is_admissible = has_valid_owners
            && input.membership == molten_core::fabric_durability::ownership::MembershipPosture::ReplaceableNodes
            && input.expected == input.current
            && input.proposed.epoch > input.current.epoch;

        assert_eq!(decision.before, current);
        assert_eq!(
            decision.disposition == molten_core::fabric_durability::ownership::CasLeaseDisposition::Acquire,
            is_admissible,
            "input {input:?}"
        );
        assert_eq!(decision.rejection.is_none(), is_admissible);
        if !has_valid_owners {
            assert_eq!(
                decision.rejection,
                Some(molten_core::fabric_durability::ownership::CasLeaseRejection::InvalidOwner)
            );
        }
        if is_admissible {
            assert_eq!(decision.after, input.proposed);
            assert!(decision.after.epoch > current.epoch, "an acquisition must advance the fencing epoch");
        } else {
            assert_eq!(decision.after, current, "a rejection must leave the lease untouched");
        }
        assert!(decision.after.epoch >= current.epoch, "fencing epochs never regress");
        current = decision.after;
    }
}

// Decisions are canonical: identical inputs share one decision ref, and changing any compared field
// changes the ref.
#[hegel::test(test_cases = 100, derandomize = true, database = None)]
fn hegel_cas_lease_decision_refs_are_canonical(tc: hegel::TestCase) {
    let current = molten_core::fabric_durability::ownership::CasLease {
        owner: draw_owner(&tc),
        epoch: tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000)),
    };
    let input = draw_attempt(&tc, &current);
    let decision = molten_core::fabric_durability::ownership::decide_cas_lease(&input);
    assert!(molten_core::protocol_oracle::is_content_ref(&decision.decision_ref));
    assert_eq!(
        molten_core::fabric_durability::ownership::decide_cas_lease(&input),
        decision,
        "deciding the same input twice must agree"
    );

    let mut advanced = input.clone();
    advanced.proposed.epoch += 1;
    assert_ne!(
        molten_core::fabric_durability::ownership::decide_cas_lease(&advanced).decision_ref,
        decision.decision_ref
    );
    let mut reposted = input;
    reposted.membership = match reposted.membership {
        molten_core::fabric_durability::ownership::MembershipPosture::ReplaceableNodes => {
            molten_core::fabric_durability::ownership::MembershipPosture::FixedMembership
        }
        molten_core::fabric_durability::ownership::MembershipPosture::FixedMembership => {
            molten_core::fabric_durability::ownership::MembershipPosture::ReplaceableNodes
        }
    };
    assert_ne!(
        molten_core::fabric_durability::ownership::decide_cas_lease(&reposted).decision_ref,
        decision.decision_ref
    );
}
