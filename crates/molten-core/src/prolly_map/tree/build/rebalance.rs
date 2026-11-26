//! Underfull last-group rebalancing for internal Prolly levels.

use super::*;

#[cfg(test)]
mod tests;

// r[impl molten.prolly.rebalance_enforces_max_node_bytes]
/// Top up an underfull last internal group from its left sibling, or merge
/// the two, without ever producing a group beyond `max_fanout` or
/// `max_node_bytes`.
///
/// A group split for bytes may be unable to take a borrowed child or merge
/// with its sibling. The last group then stays below `min_fanout`: every group
/// still satisfies both upper bounds, and readers do not require a minimum
/// fanout, so the build never produces a node it cannot encode.
#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "deterministic rebalancing must pop and merge the bounded group vector"
)]
pub(super) fn rebalance_last_group(
    profile: &ProllyProfile,
    groups: &mut Vec<Vec<ChildRange>>,
) -> Result<(), ProllyIssue> {
    if groups.len() < 2 {
        return Ok(());
    }
    let minimum = usize::from(profile.min_fanout);
    let maximum = usize::from(profile.max_fanout);
    let last_index = groups.len() - 1;
    while groups[last_index].len() < minimum && groups[last_index - 1].len() > minimum {
        let moved = groups[last_index - 1].last().ok_or(ProllyIssue::InternalFanoutExceeded)?;
        let mut candidate = Vec::with_capacity(groups[last_index].len() + 1);
        candidate.push(moved.clone());
        candidate.extend(groups[last_index].iter().cloned());
        if internal_encoded_len(profile, &candidate)? > profile.max_node_bytes {
            break;
        }
        groups[last_index - 1].pop();
        groups[last_index] = candidate;
    }
    if groups[last_index].len() < minimum {
        let combined = groups[last_index - 1]
            .len()
            .checked_add(groups[last_index].len())
            .ok_or(ProllyIssue::InternalFanoutExceeded)?;
        if combined > maximum {
            return Ok(());
        }
        let mut merged = Vec::with_capacity(combined);
        merged.extend(groups[last_index - 1].iter().cloned());
        merged.extend(groups[last_index].iter().cloned());
        if internal_encoded_len(profile, &merged)? > profile.max_node_bytes {
            return Ok(());
        }
        groups.pop();
        groups[last_index - 1] = merged;
    }
    Ok(())
}
