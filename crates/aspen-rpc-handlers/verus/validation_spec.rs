use vstd::prelude::*;

verus! {

/// Pure model of the runtime `LockOwnershipError` variants without storing
/// string payloads. Payload construction is runtime-only; the verified decision
/// is which mismatch class is selected.
#[derive(PartialEq, Eq)]
pub enum LockOwnershipDecision {
    Valid,
    HolderIdMismatch,
    FencingTokenMismatch,
    BothMismatch,
}

pub open spec fn is_lock_owner_spec(holder_matches: bool, token_matches: bool) -> bool {
    holder_matches && token_matches
}

pub open spec fn lock_ownership_decision_spec(
    holder_matches: bool,
    token_matches: bool,
) -> LockOwnershipDecision {
    if holder_matches && token_matches {
        LockOwnershipDecision::Valid
    } else if !holder_matches && !token_matches {
        LockOwnershipDecision::BothMismatch
    } else if !holder_matches {
        LockOwnershipDecision::HolderIdMismatch
    } else {
        LockOwnershipDecision::FencingTokenMismatch
    }
}

pub fn is_lock_owner_from_matches(holder_matches: bool, token_matches: bool) -> (result: bool)
    ensures
        result == is_lock_owner_spec(holder_matches, token_matches),
        result <==> holder_matches && token_matches,
        result ==> holder_matches,
        result ==> token_matches,
{
    holder_matches && token_matches
}

pub fn validate_lock_ownership_from_matches(
    holder_matches: bool,
    token_matches: bool,
) -> (result: LockOwnershipDecision)
    ensures
        result == lock_ownership_decision_spec(holder_matches, token_matches),
        result == LockOwnershipDecision::Valid <==> holder_matches && token_matches,
        result == LockOwnershipDecision::HolderIdMismatch <==> !holder_matches && token_matches,
        result == LockOwnershipDecision::FencingTokenMismatch <==> holder_matches && !token_matches,
        result == LockOwnershipDecision::BothMismatch <==> !holder_matches && !token_matches,
{
    match (holder_matches, token_matches) {
        (true, true) => LockOwnershipDecision::Valid,
        (false, false) => LockOwnershipDecision::BothMismatch,
        (false, true) => LockOwnershipDecision::HolderIdMismatch,
        (true, false) => LockOwnershipDecision::FencingTokenMismatch,
    }
}

pub proof fn valid_ownership_requires_both_matches(holder_matches: bool, token_matches: bool)
    ensures
        lock_ownership_decision_spec(holder_matches, token_matches) == LockOwnershipDecision::Valid
            <==> holder_matches && token_matches,
{
}

pub proof fn holder_mismatch_selected_only_for_holder_failure(
    holder_matches: bool,
    token_matches: bool,
)
    ensures
        lock_ownership_decision_spec(holder_matches, token_matches)
            == LockOwnershipDecision::HolderIdMismatch <==> !holder_matches && token_matches,
{
}

pub proof fn token_mismatch_selected_only_for_token_failure(
    holder_matches: bool,
    token_matches: bool,
)
    ensures
        lock_ownership_decision_spec(holder_matches, token_matches)
            == LockOwnershipDecision::FencingTokenMismatch <==> holder_matches && !token_matches,
{
}

pub proof fn both_mismatch_selected_only_for_dual_failure(
    holder_matches: bool,
    token_matches: bool,
)
    ensures
        lock_ownership_decision_spec(holder_matches, token_matches)
            == LockOwnershipDecision::BothMismatch <==> !holder_matches && !token_matches,
{
}

} // verus!
