//! Lock-set state model and proof-linked runtime invariants.
//!
//! These checks document two core guarantees for distributed lock sets:
//! - all-or-nothing live ownership across the canonical member set
//! - per-resource fencing tokens never move backward across reacquisition

use crate::spec::lock_invariants::LockEntrySpec;

/// Abstract state for one lock-set member.
#[derive(Clone, Debug, Default)]
pub struct LockSetMemberStateSpec {
    /// Canonical member name.
    pub member: String,
    /// Current entry for that member.
    pub entry: Option<LockEntrySpec>,
}

/// Abstract state for a full lock set.
#[derive(Clone, Debug, Default)]
pub struct LockSetStateSpec {
    /// Canonical member states in sorted order.
    pub members: Vec<LockSetMemberStateSpec>,
    /// Current time for expiry checks.
    pub current_time_ms: u64,
}

/// All members are stored in canonical strictly increasing order.
pub fn canonical_member_ordering(state: &LockSetStateSpec) -> bool {
    state.members.windows(2).all(|pair| pair[0].member < pair[1].member)
}

/// Live members must all belong to the same holder.
pub fn all_or_nothing_live_ownership(state: &LockSetStateSpec) -> bool {
    let mut live_holder: Option<&str> = None;
    for member in &state.members {
        let Some(entry) = &member.entry else {
            continue;
        };
        if entry.deadline_ms == 0 || state.current_time_ms > entry.deadline_ms {
            continue;
        }
        match live_holder {
            None => live_holder = Some(entry.holder_id.as_str()),
            Some(holder) if holder == entry.holder_id => {}
            Some(_) => return false,
        }
    }
    true
}

/// Per-resource tokens must not decrease across transitions.
pub fn member_tokens_monotonic(pre: &LockSetStateSpec, post: &LockSetStateSpec) -> bool {
    pre.members.iter().zip(post.members.iter()).all(|(before, after)| {
        before.member == after.member
            && after.entry.as_ref().map(|entry| entry.fencing_token).unwrap_or(0)
                >= before.entry.as_ref().map(|entry| entry.fencing_token).unwrap_or(0)
    })
}

/// Combined lock-set invariant used by runtime checks and tests.
pub fn lockset_invariant(state: &LockSetStateSpec) -> bool {
    canonical_member_ordering(state) && all_or_nothing_live_ownership(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live_entry(member: &str, holder: &str, token: u64) -> LockSetMemberStateSpec {
        LockSetMemberStateSpec {
            member: member.to_string(),
            entry: Some(LockEntrySpec {
                holder_id: holder.to_string(),
                fencing_token: token,
                acquired_at_ms: 10,
                ttl_ms: 100,
                deadline_ms: 110,
            }),
        }
    }

    #[test]
    fn test_lockset_invariant_accepts_single_holder() {
        let state = LockSetStateSpec {
            members: vec![
                live_entry("deploy:prod", "holder-a", 1),
                live_entry("pipeline:42", "holder-a", 4),
                live_entry("repo:a", "holder-a", 2),
            ],
            current_time_ms: 50,
        };
        assert!(lockset_invariant(&state));
    }

    #[test]
    fn test_lockset_invariant_rejects_split_live_holders() {
        let state = LockSetStateSpec {
            members: vec![
                live_entry("deploy:prod", "holder-a", 1),
                live_entry("pipeline:42", "holder-b", 4),
            ],
            current_time_ms: 50,
        };
        assert!(!all_or_nothing_live_ownership(&state));
    }

    #[test]
    fn test_member_tokens_monotonic() {
        let pre = LockSetStateSpec {
            members: vec![live_entry("repo:a", "holder-a", 2)],
            current_time_ms: 50,
        };
        let post = LockSetStateSpec {
            members: vec![live_entry("repo:a", "holder-b", 3)],
            current_time_ms: 60,
        };
        assert!(member_tokens_monotonic(&pre, &post));
    }
}
