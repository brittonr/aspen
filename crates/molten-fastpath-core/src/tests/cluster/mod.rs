//! Deterministic multi-replica simulation of fast admission and session-keyed convergence.
//!
//! Two operations carry the same command from distinct client sessions. Their member and
//! non-member acknowledgements arrive in many seeded interleavings; each operation joins the
//! committed log when stable-view admission first fast-commits it. Every replica then receives
//! that log, duplicate re-deliveries, and a conflicting retry of one session operation, in its own
//! seeded order, and applies them through its own `ApplicationLedger`.

const REPLICAS: usize = 5;
const OUTSIDERS: usize = 3;
const INTERLEAVINGS: u64 = 48;
const LCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const LCG_INCREMENT: u64 = 1_442_695_040_888_963_407;
const LCG_SHIFT: u32 = 33;

fn session_operation(session_ref: &str, command_ref: &str) -> crate::quorate::OperationIdentity {
    crate::quorate::OperationIdentity {
        session_ref: session_ref.to_owned(),
        command_ref: command_ref.to_owned(),
        ..super::support::operation(command_ref)
    }
}

/// A deterministic Fisher-Yates shuffle driven by a 64-bit LCG.
fn shuffled<T>(mut items: Vec<T>, seed: u64) -> Vec<T> {
    let mut state = seed;
    for index in (1..items.len()).rev() {
        state = state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_INCREMENT);
        let draw = usize::try_from(state >> LCG_SHIFT).expect("31-bit draw fits usize");
        items.swap(index, draw.checked_rem(index + 1).expect("non-zero bound"));
    }
    items
}

/// Acknowledgement messages as (operation index, sender).
fn interleaving(seed: u64, operations: usize) -> Vec<(usize, alloc::string::String)> {
    let senders = (0..REPLICAS)
        .map(super::support::replica)
        .chain((0..OUTSIDERS).map(|index| format!("outsider-{index}")));
    let deliveries: Vec<(usize, alloc::string::String)> = (0..operations)
        .flat_map(|operation| senders.clone().map(move |sender| (operation, sender)))
        .collect();
    if seed == 0 {
        // Every non-member acknowledgement first, then members in index order: with three members
        // (the proposer among them) the non-members would complete the superquorum if they counted.
        let (mut outsiders, members): (Vec<_>, Vec<_>) =
            deliveries.into_iter().partition(|(_, sender)| sender.starts_with("outsider"));
        outsiders.extend(members);
        return outsiders;
    }
    shuffled(deliveries, seed)
}

/// Feed acknowledgements one at a time; record each operation when it first fast-commits.
fn commit_log(operations: &[crate::quorate::OperationIdentity], seed: u64) -> Vec<crate::quorate::OperationIdentity> {
    let quorums = super::support::quorums(REPLICAS);
    let members = super::support::strings(&["node-0", "node-1", "node-2", "node-3", "node-4"]);
    let mut received: Vec<Vec<crate::quorate::FastAcknowledgement>> = vec![Vec::new(); operations.len()];
    let mut log = Vec::with_capacity(operations.len());
    for (index, sender) in interleaving(seed, operations.len()) {
        received[index].push(super::support::acknowledgement(sender, &operations[index]));
        let mut attempt = super::support::attempt(REPLICAS);
        attempt.operation = operations[index].clone();
        attempt.original_operation = operations[index].clone();
        attempt.acknowledgements = received[index].clone();
        let decision = crate::quorate::evaluate_stable_view(&attempt, quorums);
        let member_acknowledgers: alloc::collections::BTreeSet<&str> = received[index]
            .iter()
            .map(|row| row.replica_id.as_str())
            .filter(|id| members.contains(*id))
            .collect();
        let is_admissible = member_acknowledgers.len() >= quorums.superquorum
            && member_acknowledgers.contains(super::support::PROPOSER);
        assert_eq!(
            decision == crate::quorate::StableViewDecision::FastCommitted,
            is_admissible,
            "seed {seed}: operation {index} after {} member acknowledgements and non-member ones",
            member_acknowledgers.len()
        );
        if is_admissible && !log.contains(&operations[index]) {
            log.push(operations[index].clone());
        }
    }
    log
}

/// One replica's deliveries: the log in order, a re-delivery of each entry and the conflicting
/// retry, each inserted at a seeded position after the entry it repeats.
fn replica_deliveries(
    log: &[crate::quorate::OperationIdentity],
    retry: &crate::quorate::OperationIdentity,
    seed: u64,
) -> Vec<crate::quorate::OperationIdentity> {
    let mut deliveries = log.to_vec();
    let extras = log.iter().cloned().chain([retry.clone()]);
    for (offset, extra) in (1_u64..).zip(extras) {
        let after = deliveries
            .iter()
            .position(|operation| operation.session_ref == extra.session_ref)
            .expect("repeated session operation is in the log");
        let span = deliveries.len() - after;
        let draw = shuffled((0..span).collect(), seed.wrapping_add(offset))[0];
        deliveries.insert(after + 1 + draw, extra);
    }
    deliveries
}

// r[verify molten.consensus.fast_path_model.fallback_identity]
// r[verify molten.consensus.fast_path_model.stable_view]
#[test]
fn simulated_replicas_converge_on_session_keyed_ledgers_and_ignore_non_member_acknowledgements() {
    let session_a = session_operation("session-a", "command-x");
    let session_b = session_operation("session-b", "command-x");
    let conflicting_retry = session_operation("session-a", "command-y");
    for seed in 0..INTERLEAVINGS {
        let log = commit_log(&[session_a.clone(), session_b.clone()], seed);
        assert_eq!(log.len(), 2, "seed {seed}: both equal-command session operations commit");
        let mut applied_sequences = Vec::with_capacity(REPLICAS);
        for replica in 0..REPLICAS {
            let mut ledger = crate::quorate::ApplicationLedger::new();
            let mut applied = Vec::with_capacity(log.len());
            let replica_seed = seed.wrapping_mul(LCG_MULTIPLIER).wrapping_add(u64::try_from(replica).expect("index"));
            for delivery in replica_deliveries(&log, &conflicting_retry, replica_seed) {
                match ledger.converge(&delivery) {
                    Ok(outcome) if outcome.is_applied => applied.push(delivery),
                    Ok(outcome) => assert!(outcome.is_duplicate_suppressed, "seed {seed} replica {replica}"),
                    Err(issue) => {
                        assert_eq!(issue, crate::quorate::LedgerIssue::ConflictingSessionRetry);
                        assert_eq!(
                            delivery, conflicting_retry,
                            "seed {seed} replica {replica}: only the retry is rejected"
                        );
                    }
                }
            }
            assert_eq!(applied, log, "seed {seed} replica {replica}: each operation applies once, in log order");
            assert!(!applied.contains(&conflicting_retry), "seed {seed} replica {replica}");
            applied_sequences.push(applied);
        }
        assert!(applied_sequences.windows(2).all(|pair| pair[0] == pair[1]), "seed {seed}: replicas diverged");
    }
}
