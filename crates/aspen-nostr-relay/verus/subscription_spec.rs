//! Verus specs for Nostr subscription admission and per-connection state shape.
//!
//! Production `src/subscriptions.rs` stores subscriptions in a `HashMap` behind
//! an async `RwLock`. This module verifies the deterministic scalar contract
//! around that shell: per-connection limit admission, replacement without limit
//! expansion, on-the-fly connection creation, unsubscribe/remove count effects,
//! and HashMap-style deduplicated subscription identifiers.

use vstd::prelude::*;

verus! {

pub const DEFAULT_MAX_SUBSCRIPTIONS_PER_CONNECTION: u32 = 16;
pub const BROADCAST_CHANNEL_CAPACITY: u32 = 4096;

pub enum SubscribeAction {
    InsertNew,
    ReplaceExisting,
    RejectTooMany,
}

pub enum ConnectionLookupAction {
    UseExisting,
    CreateMissing,
}

pub open spec fn limit_positive(max: u32) -> bool {
    max > 0
}

pub open spec fn broadcast_capacity_positive(capacity: u32) -> bool {
    capacity > 0
}

pub open spec fn subscribe_action(current: u32, max: u32, is_replacement: bool) -> SubscribeAction {
    if is_replacement {
        SubscribeAction::ReplaceExisting
    } else if current >= max {
        SubscribeAction::RejectTooMany
    } else {
        SubscribeAction::InsertNew
    }
}

pub open spec fn subscribe_succeeds(current: u32, max: u32, is_replacement: bool) -> bool {
    subscribe_action(current, max, is_replacement) != SubscribeAction::RejectTooMany
}

pub open spec fn subscribe_result_count(current: u32, max: u32, is_replacement: bool) -> u32 {
    match subscribe_action(current, max, is_replacement) {
        SubscribeAction::InsertNew => (current + 1) as u32,
        SubscribeAction::ReplaceExisting => current,
        SubscribeAction::RejectTooMany => current,
    }
}

pub open spec fn too_many_error_current(current: u32, max: u32, is_replacement: bool) -> Option<u32> {
    if subscribe_action(current, max, is_replacement) == SubscribeAction::RejectTooMany {
        Some(current)
    } else {
        None::<u32>
    }
}

pub open spec fn connection_lookup_action(connection_exists: bool) -> ConnectionLookupAction {
    if connection_exists { ConnectionLookupAction::UseExisting } else { ConnectionLookupAction::CreateMissing }
}

pub open spec fn connection_count_after_add(existing_connections: u32, connection_exists: bool) -> u32 {
    if connection_exists { existing_connections } else { (existing_connections + 1) as u32 }
}

pub open spec fn connection_count_after_remove(existing_connections: u32, connection_exists: bool) -> u32 {
    if connection_exists { (existing_connections - 1) as u32 } else { existing_connections }
}

pub open spec fn subscriptions_after_remove_connection(connection_exists: bool) -> u32 {
    0
}

pub open spec fn unsubscribe_result_count(current: u32, sub_exists: bool) -> u32 {
    if sub_exists { (current - 1) as u32 } else { current }
}

pub open spec fn deduped_subscription_count(raw_ids: u32, duplicate_ids: u32) -> u32 {
    (raw_ids - duplicate_ids) as u32
}

pub open spec fn dedup_count_valid(raw_ids: u32, duplicate_ids: u32) -> bool {
    duplicate_ids <= raw_ids
}

pub fn subscribe_action_exec(current: u32, max: u32, is_replacement: bool) -> (action: SubscribeAction)
    ensures action == subscribe_action(current, max, is_replacement)
{
    if is_replacement {
        SubscribeAction::ReplaceExisting
    } else if current >= max {
        SubscribeAction::RejectTooMany
    } else {
        SubscribeAction::InsertNew
    }
}

pub fn subscribe_result_count_exec(current: u32, max: u32, is_replacement: bool) -> (next: u32)
    requires current < u32::MAX
    ensures next == subscribe_result_count(current, max, is_replacement)
{
    if is_replacement {
        current
    } else if current >= max {
        current
    } else {
        current + 1
    }
}

pub fn unsubscribe_result_count_exec(current: u32, sub_exists: bool) -> (next: u32)
    requires sub_exists ==> current > 0
    ensures next == unsubscribe_result_count(current, sub_exists)
{
    if sub_exists { current - 1 } else { current }
}

pub proof fn default_subscription_limit_is_positive()
    ensures limit_positive(DEFAULT_MAX_SUBSCRIPTIONS_PER_CONNECTION)
{
}

pub proof fn broadcast_capacity_is_positive()
    ensures broadcast_capacity_positive(BROADCAST_CHANNEL_CAPACITY)
{
}

pub proof fn new_subscription_below_limit_is_inserted(current: u32, max: u32)
    requires current < max
    ensures
        subscribe_action(current, max, false) == SubscribeAction::InsertNew,
        subscribe_succeeds(current, max, false),
        subscribe_result_count(current, max, false) == current + 1,
{
}

pub proof fn new_subscription_at_limit_is_rejected(current: u32, max: u32)
    requires current >= max
    ensures
        subscribe_action(current, max, false) == SubscribeAction::RejectTooMany,
        !subscribe_succeeds(current, max, false),
        subscribe_result_count(current, max, false) == current,
        too_many_error_current(current, max, false) == Some(current),
{
}

pub proof fn replacement_bypasses_limit_and_preserves_count(current: u32, max: u32)
    ensures
        subscribe_action(current, max, true) == SubscribeAction::ReplaceExisting,
        subscribe_succeeds(current, max, true),
        subscribe_result_count(current, max, true) == current,
        too_many_error_current(current, max, true) == None::<u32>,
{
}

pub proof fn successful_new_subscription_stays_within_limit(current: u32, max: u32)
    requires current < max
    ensures subscribe_result_count(current, max, false) <= max
{
}

pub proof fn rejected_subscription_does_not_expand_count(current: u32, max: u32)
    requires current >= max
    ensures subscribe_result_count(current, max, false) == current
{
}

pub proof fn missing_connection_is_created_on_subscribe(existing_connections: u32)
    requires existing_connections < u32::MAX
    ensures
        connection_lookup_action(false) == ConnectionLookupAction::CreateMissing,
        connection_count_after_add(existing_connections, false) == existing_connections + 1,
{
}

pub proof fn existing_connection_is_reused_on_subscribe(existing_connections: u32)
    ensures
        connection_lookup_action(true) == ConnectionLookupAction::UseExisting,
        connection_count_after_add(existing_connections, true) == existing_connections,
{
}

pub proof fn remove_existing_connection_clears_subscriptions(existing_connections: u32)
    requires existing_connections > 0
    ensures
        connection_count_after_remove(existing_connections, true) == existing_connections - 1,
        subscriptions_after_remove_connection(true) == 0,
{
}

pub proof fn remove_missing_connection_is_noop(existing_connections: u32)
    ensures
        connection_count_after_remove(existing_connections, false) == existing_connections,
        subscriptions_after_remove_connection(false) == 0,
{
}

pub proof fn unsubscribe_existing_decrements_count(current: u32)
    requires current > 0
    ensures unsubscribe_result_count(current, true) == current - 1
{
}

pub proof fn unsubscribe_missing_preserves_count(current: u32)
    ensures unsubscribe_result_count(current, false) == current
{
}

pub proof fn duplicate_subscription_ids_do_not_increase_dedup_count(raw_ids: u32, duplicate_ids: u32)
    requires dedup_count_valid(raw_ids, duplicate_ids)
    ensures deduped_subscription_count(raw_ids, duplicate_ids) <= raw_ids
{
}

pub proof fn no_duplicate_subscription_ids_preserve_count(raw_ids: u32)
    ensures deduped_subscription_count(raw_ids, 0) == raw_ids
{
}

pub proof fn all_subscription_ids_duplicate_collapse_to_zero(raw_ids: u32)
    ensures deduped_subscription_count(raw_ids, raw_ids) == 0
{
}

} // verus!
