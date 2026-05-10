//! Queue delivery helper specifications.
//!
//! Formal specs for the pure delivery-attempt helpers in
//! `src/verified/queue/delivery.rs`.

use vstd::prelude::*;

verus! {
    pub enum RequeuePrioritySpec {
        Normal,
        Elevated,
        High,
    }

    pub open spec fn has_exceeded_max_delivery_attempts_spec(
        delivery_attempts: u32,
        max_delivery_attempts: u32,
    ) -> bool {
        max_delivery_attempts > 0 && delivery_attempts >= max_delivery_attempts
    }

    pub fn has_exceeded_max_delivery_attempts(
        delivery_attempts: u32,
        max_delivery_attempts: u32,
    ) -> (exceeded: bool)
        ensures
            exceeded == has_exceeded_max_delivery_attempts_spec(delivery_attempts, max_delivery_attempts),
            max_delivery_attempts == 0 ==> !exceeded,
            max_delivery_attempts > 0 && delivery_attempts >= max_delivery_attempts ==> exceeded,
            max_delivery_attempts > 0 && delivery_attempts < max_delivery_attempts ==> !exceeded,
    {
        max_delivery_attempts > 0 && delivery_attempts >= max_delivery_attempts
    }

    pub open spec fn increment_delivery_count_spec(current_count: u32) -> u32 {
        if current_count == 0xFFFF_FFFFu32 {
            0xFFFF_FFFFu32
        } else {
            (current_count + 1) as u32
        }
    }

    pub fn increment_delivery_count(current_count: u32) -> (next: u32)
        ensures
            next == increment_delivery_count_spec(current_count),
            next >= current_count,
            current_count < 0xFFFF_FFFFu32 ==> next == current_count + 1,
            current_count == 0xFFFF_FFFFu32 ==> next == current_count,
    {
        if current_count == 0xFFFF_FFFFu32 {
            0xFFFF_FFFFu32
        } else {
            (current_count + 1) as u32
        }
    }

    pub fn increment_delivery_count_for_dequeue(current_count: u32) -> (next: u32)
        ensures
            next == increment_delivery_count_spec(current_count),
            next >= current_count,
    {
        increment_delivery_count(current_count)
    }

    pub open spec fn can_increment_delivery_count_spec(delivery_count: u32) -> bool {
        delivery_count < 0xFFFF_FFFFu32
    }

    pub fn can_increment_delivery_count(delivery_count: u32) -> (can_increment: bool)
        ensures
            can_increment == can_increment_delivery_count_spec(delivery_count),
            can_increment ==> increment_delivery_count_spec(delivery_count) == delivery_count + 1,
            !can_increment ==> increment_delivery_count_spec(delivery_count) == delivery_count,
    {
        delivery_count < 0xFFFF_FFFFu32
    }

    pub open spec fn decrement_delivery_count_for_release_spec(delivery_count: u32) -> u32 {
        if delivery_count == 0 {
            0u32
        } else {
            (delivery_count - 1) as u32
        }
    }

    pub fn decrement_delivery_count_for_release(delivery_count: u32) -> (next: u32)
        ensures
            next == decrement_delivery_count_for_release_spec(delivery_count),
            next <= delivery_count,
            delivery_count > 0 ==> next == delivery_count - 1,
            delivery_count == 0 ==> next == 0,
    {
        if delivery_count == 0 {
            0u32
        } else {
            (delivery_count - 1) as u32
        }
    }

    pub open spec fn compute_requeue_delivery_attempts_spec(
        current_attempts: u32,
        increment: bool,
    ) -> u32 {
        if increment {
            increment_delivery_count_spec(current_attempts)
        } else {
            decrement_delivery_count_for_release_spec(current_attempts)
        }
    }

    pub fn compute_requeue_delivery_attempts(current_attempts: u32, increment: bool) -> (next: u32)
        ensures
            next == compute_requeue_delivery_attempts_spec(current_attempts, increment),
            increment ==> next >= current_attempts,
            !increment ==> next <= current_attempts,
            increment && current_attempts < 0xFFFF_FFFFu32 ==> next == current_attempts + 1,
            increment && current_attempts == 0xFFFF_FFFFu32 ==> next == current_attempts,
            !increment && current_attempts > 0 ==> next == current_attempts - 1,
            !increment && current_attempts == 0 ==> next == 0,
    {
        if increment {
            increment_delivery_count(current_attempts)
        } else {
            decrement_delivery_count_for_release(current_attempts)
        }
    }

    pub open spec fn compute_requeue_priority_spec(
        delivery_attempts: u32,
        max_delivery_attempts: u32,
    ) -> RequeuePrioritySpec {
        if max_delivery_attempts == 0 {
            RequeuePrioritySpec::Normal
        } else {
            let remaining = if delivery_attempts >= max_delivery_attempts {
                0u32
            } else {
                (max_delivery_attempts - delivery_attempts) as u32
            };
            if remaining <= 1 {
                RequeuePrioritySpec::High
            } else if remaining <= max_delivery_attempts / 2 {
                RequeuePrioritySpec::Elevated
            } else {
                RequeuePrioritySpec::Normal
            }
        }
    }

    pub fn compute_requeue_priority(
        delivery_attempts: u32,
        max_delivery_attempts: u32,
    ) -> (priority: RequeuePrioritySpec)
        ensures
            priority == compute_requeue_priority_spec(delivery_attempts, max_delivery_attempts),
            max_delivery_attempts == 0 ==> priority is Normal,
            max_delivery_attempts > 0 && delivery_attempts >= max_delivery_attempts ==> priority is High,
            max_delivery_attempts > 0 && max_delivery_attempts > delivery_attempts &&
                max_delivery_attempts - delivery_attempts <= 1 ==> priority is High,
            max_delivery_attempts > 0 && max_delivery_attempts - delivery_attempts > 1 &&
                max_delivery_attempts - delivery_attempts <= max_delivery_attempts / 2 ==> priority is Elevated,
    {
        if max_delivery_attempts == 0 {
            return RequeuePrioritySpec::Normal;
        }

        let remaining = if delivery_attempts >= max_delivery_attempts {
            0u32
        } else {
            (max_delivery_attempts - delivery_attempts) as u32
        };

        if remaining <= 1 {
            return RequeuePrioritySpec::High;
        }

        if remaining <= max_delivery_attempts / 2 {
            return RequeuePrioritySpec::Elevated;
        }

        RequeuePrioritySpec::Normal
    }

    pub proof fn no_limit_never_exceeds(delivery_attempts: u32)
        ensures
            !has_exceeded_max_delivery_attempts_spec(delivery_attempts, 0u32),
            compute_requeue_priority_spec(delivery_attempts, 0u32) is Normal,
    {
    }

    pub proof fn saturated_increment_is_idempotent()
        ensures
            increment_delivery_count_spec(0xFFFF_FFFFu32) == 0xFFFF_FFFFu32,
            compute_requeue_delivery_attempts_spec(0xFFFF_FFFFu32, true) == 0xFFFF_FFFFu32,
    {
    }

    pub proof fn zero_decrement_is_idempotent()
        ensures
            decrement_delivery_count_for_release_spec(0u32) == 0u32,
            compute_requeue_delivery_attempts_spec(0u32, false) == 0u32,
    {
    }
}
