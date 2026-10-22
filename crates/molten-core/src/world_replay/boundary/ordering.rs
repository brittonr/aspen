use super::*;

/// The replay order of recorded observations: by transition step, then by
/// explicit logical position. Wall-clock arrival, thread wake order, and
/// adapter iteration order play no part.
// r[impl molten.world_replay_boundary.ordering]
pub fn replay_dispatch_order(observations: &[RecordedReplayObservation]) -> Vec<&RecordedReplayObservation> {
    let mut ordered = observations.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|observation| observation.slot.order_key());
    ordered
}

/// The first order key that does not strictly follow its predecessor.
pub fn first_order_violation(keys: impl IntoIterator<Item = (u64, u64)>) -> Option<(u64, u64)> {
    let mut previous = None;
    for key in keys {
        if previous.is_some_and(|earlier| key <= earlier) {
            return Some(key);
        }
        previous = Some(key);
    }
    None
}

/// Compare the recorded observations with the slots, slot by slot in replay
/// order, and return the earliest divergence. An observation at a slot's
/// position that answers another slot's request is an ordering divergence; any
/// other answer is a wrong source or a wrong request. A position with no
/// observation is left to the completeness check.
// r[impl molten.world_replay_boundary.ordering]
pub fn first_slot_divergence(
    slots: &[ReplayObservationSlot],
    observations: &[RecordedReplayObservation],
) -> Option<ReplayBoundaryBlocker> {
    let by_position = observations
        .iter()
        .map(|observation| (observation.slot.order_key(), &observation.slot))
        .collect::<std::collections::BTreeMap<_, _>>();
    for slot in slots {
        let Some(observed) = by_position.get(&slot.order_key()) else {
            continue;
        };
        if slot.binds_same_request(observed) {
            continue;
        }
        let (step_position, logical_position) = slot.order_key();
        let is_reordered = slots
            .iter()
            .any(|other| other.order_key() != slot.order_key() && other.binds_same_request(observed));
        return Some(if is_reordered {
            ReplayBoundaryBlocker::OrderingDivergence {
                step_position,
                logical_position,
            }
        } else if slot.source_id != observed.source_id {
            ReplayBoundaryBlocker::WrongSource {
                step_position,
                logical_position,
            }
        } else {
            ReplayBoundaryBlocker::WrongRequest {
                step_position,
                logical_position,
            }
        });
    }
    None
}
