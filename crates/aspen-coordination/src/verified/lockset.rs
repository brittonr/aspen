//! Pure lock-set helpers.
//!
//! Deterministic helpers for canonical ordering, duplicate detection,
//! per-member fencing token computation, and set-wide retry / renewal logic.

use crate::types::LockEntry;
use crate::types::LockSetMemberToken;
use crate::verified::compute_next_fencing_token;
use crate::verified::is_lock_expired;
use crate::verified::remaining_ttl_ms;

/// Validation errors for lock-set member normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockSetValidationError {
    /// Empty lock sets are invalid.
    EmptySet,
    /// Lock set exceeds configured bound.
    TooManyMembers { count: u32, max: u32 },
    /// Canonicalized member list contains a duplicate.
    DuplicateMember { member: String },
}

/// Canonicalize and validate a lock-set member list.
#[inline]
pub fn normalize_lockset_members(
    mut members: Vec<String>,
    max_members: u32,
) -> Result<Vec<String>, LockSetValidationError> {
    validate_lockset_count(members.len(), max_members)?;
    members.sort();
    if let Some(member) = find_duplicate_member(&members) {
        return Err(LockSetValidationError::DuplicateMember { member });
    }
    Ok(members)
}

/// Canonicalize and validate a lock-set member-token list.
#[inline]
pub fn normalize_lockset_member_tokens(
    mut member_tokens: Vec<LockSetMemberToken>,
    max_members: u32,
) -> Result<Vec<LockSetMemberToken>, LockSetValidationError> {
    validate_lockset_count(member_tokens.len(), max_members)?;
    member_tokens.sort_by(|left, right| left.member.cmp(&right.member));
    if let Some(member) = find_duplicate_member_tokens(&member_tokens) {
        return Err(LockSetValidationError::DuplicateMember { member });
    }
    Ok(member_tokens)
}

#[inline]
fn validate_lockset_count(member_count: usize, max_members: u32) -> Result<(), LockSetValidationError> {
    let count = match u32::try_from(member_count) {
        Ok(count) => count,
        Err(_) => {
            return Err(LockSetValidationError::TooManyMembers {
                count: max_members.saturating_add(1),
                max: max_members,
            });
        }
    };
    if count == 0 {
        return Err(LockSetValidationError::EmptySet);
    }
    if count > max_members {
        return Err(LockSetValidationError::TooManyMembers {
            count,
            max: max_members,
        });
    }
    Ok(())
}

/// Find the first duplicate member in an already-sorted member list.
#[inline]
pub fn find_duplicate_member(sorted_members: &[String]) -> Option<String> {
    sorted_members.windows(2).find(|pair| pair[0] == pair[1]).map(|pair| pair[0].clone())
}

/// Find the first duplicate member in an already-sorted member-token list.
#[inline]
pub fn find_duplicate_member_tokens(sorted_member_tokens: &[LockSetMemberToken]) -> Option<String> {
    sorted_member_tokens
        .windows(2)
        .find(|pair| pair[0].member == pair[1].member)
        .map(|pair| pair[0].member.clone())
}

/// Compute the next fencing token for one lock-set member.
#[inline]
pub fn compute_lockset_member_token(current_entry: Option<&LockEntry>) -> u64 {
    compute_next_fencing_token(current_entry)
}

/// Count how many members are taken over from expired holders.
#[inline]
pub fn compute_lockset_takeover_count(entries: &[Option<LockEntry>], now_ms: u64) -> u64 {
    entries.iter().fold(0_u64, |takeover_count, entry| {
        if entry
            .as_ref()
            .is_some_and(|member| member.deadline_ms != 0 && is_lock_expired(member.deadline_ms, now_ms))
        {
            takeover_count.saturating_add(1)
        } else {
            takeover_count
        }
    })
}

/// Decide whether a retrying acquire should continue.
#[inline]
pub fn should_retry_lockset_acquire(now_ms: u64, acquire_deadline_ms: u64) -> bool {
    now_ms < acquire_deadline_ms
}

/// Decide whether a lock set should be renewed now.
#[inline]
pub fn should_renew_lockset(deadline_ms: u64, now_ms: u64, renew_window_ms: u64) -> bool {
    if deadline_ms == 0 || now_ms > deadline_ms {
        return false;
    }
    remaining_ttl_ms(deadline_ms, now_ms) <= renew_window_ms
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::types::FencingToken;

    #[test]
    fn test_normalize_lockset_members_sorts() {
        let normalized = normalize_lockset_members(
            vec![
                "pipeline:42".to_string(),
                "repo:a".to_string(),
                "deploy:prod".to_string(),
            ],
            8,
        )
        .unwrap();
        assert_eq!(normalized, vec!["deploy:prod", "pipeline:42", "repo:a"]);
    }

    #[test]
    fn test_normalize_lockset_members_rejects_duplicate() {
        let error = normalize_lockset_members(vec!["repo:a".to_string(), "repo:a".to_string()], 8).unwrap_err();
        assert_eq!(error, LockSetValidationError::DuplicateMember {
            member: "repo:a".to_string(),
        });
    }

    #[test]
    fn test_normalize_lockset_member_tokens_sorts() {
        let normalized = normalize_lockset_member_tokens(
            vec![
                LockSetMemberToken::new("pipeline:42", FencingToken::new(2)),
                LockSetMemberToken::new("repo:a", FencingToken::new(1)),
            ],
            8,
        )
        .unwrap();
        assert_eq!(normalized[0].member, "pipeline:42");
        assert_eq!(normalized[1].member, "repo:a");
    }

    #[test]
    fn test_should_renew_lockset_inside_window() {
        assert!(should_renew_lockset(10_000, 9_500, 600));
        assert!(!should_renew_lockset(10_000, 8_000, 600));
        assert!(!should_renew_lockset(0, 8_000, 600));
    }

    proptest! {
        #[test]
        fn prop_normalized_members_are_sorted(mut members in prop::collection::vec("[a-z]{1,6}", 1..8)) {
            members.sort();
            members.dedup();
            let normalized = normalize_lockset_members(members.clone(), 8).unwrap();
            prop_assert_eq!(normalized, members);
        }

        #[test]
        fn prop_lockset_member_token_is_monotonic(token in any::<u64>()) {
            let entry = LockEntry {
                holder_id: "holder".to_string(),
                fencing_token: token,
                acquired_at_ms: 1,
                ttl_ms: 10,
                deadline_ms: 11,
            };
            let next = compute_lockset_member_token(Some(&entry));
            prop_assert!(next >= token);
        }

        #[test]
        fn prop_takeover_count_is_bounded(entries in prop::collection::vec(any::<(bool, u64, u64)>(), 0..8)) {
            let members: Vec<Option<LockEntry>> = entries
                .into_iter()
                .map(|(expired, token, deadline_ms)| {
                    let deadline_ms = if expired { 1 } else { deadline_ms.saturating_add(2) };
                    Some(LockEntry {
                        holder_id: "holder".to_string(),
                        fencing_token: token,
                        acquired_at_ms: 0,
                        ttl_ms: 0,
                        deadline_ms,
                    })
                })
                .collect();
            let count = compute_lockset_takeover_count(&members, 2);
            prop_assert!(usize::try_from(count).unwrap_or(usize::MAX) <= members.len());
        }
    }
}
