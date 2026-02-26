//! Timestamp Validation Specification
//!
//! Formal specification for timestamp-based security properties of signed tickets.
//!
//! # Security Properties
//!
//! 1. **SIGNED-2a: Not Expired**: Current time < expires_at
//! 2. **SIGNED-2b: Not Future Issued**: issued_at <= now + clock_skew
//! 3. **SIGNED-2c: Valid Duration**: issued_at <= expires_at
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-ticket/verus/timestamp_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Clock skew tolerance in seconds (5 minutes)
    pub const CLOCK_SKEW_TOLERANCE_SECS: u64 = 300;

    /// Default ticket validity in seconds (24 hours)
    pub const DEFAULT_TICKET_VALIDITY_SECS: u64 = 86400;

    /// Minimum ticket validity in seconds (1 minute)
    pub const MIN_TICKET_VALIDITY_SECS: u64 = 60;

    /// Maximum ticket validity in seconds (30 days)
    pub const MAX_TICKET_VALIDITY_SECS: u64 = 30 * 86400;

    // ========================================================================
    // Timestamp State
    // ========================================================================

    /// Abstract timestamp state for verification
    pub struct TimestampState {
        /// Unix timestamp when ticket was issued
        pub issued_at_secs: u64,
        /// Unix timestamp when ticket expires
        pub expires_at_secs: u64,
    }

    // ========================================================================
    // Invariant: Timestamp Validity
    // ========================================================================

    /// SIGNED-2a: Ticket is not expired
    pub open spec fn not_expired(
        ticket: TimestampState,
        current_time_secs: u64,
    ) -> bool {
        ticket.expires_at_secs >= current_time_secs
    }

    /// SIGNED-2b: Ticket was not issued in the future
    ///
    /// Allows for clock skew tolerance.
    pub open spec fn not_issued_in_future(
        ticket: TimestampState,
        current_time_secs: u64,
        tolerance_secs: u64,
    ) -> bool {
        ticket.issued_at_secs <= current_time_secs + tolerance_secs
    }

    /// Convenience: within clock skew tolerance
    pub open spec fn within_clock_skew(
        ticket: TimestampState,
        current_time_secs: u64,
    ) -> bool {
        not_issued_in_future(ticket, current_time_secs, CLOCK_SKEW_TOLERANCE_SECS)
    }

    /// SIGNED-2c: Valid duration (issued <= expires)
    pub open spec fn valid_duration(ticket: TimestampState) -> bool {
        ticket.issued_at_secs <= ticket.expires_at_secs
    }

    /// Combined timestamp validity
    pub open spec fn timestamp_validity(
        ticket: TimestampState,
        current_time_secs: u64,
    ) -> bool {
        not_expired(ticket, current_time_secs) &&
        within_clock_skew(ticket, current_time_secs) &&
        valid_duration(ticket)
    }

    // ========================================================================
    // Validity Window
    // ========================================================================

    /// Ticket validity window (time range where ticket is valid)
    pub open spec fn validity_window_start(
        ticket: TimestampState,
    ) -> u64 {
        // Valid from issued_at (minus clock skew tolerance on verifier side)
        if ticket.issued_at_secs >= CLOCK_SKEW_TOLERANCE_SECS {
            ticket.issued_at_secs - CLOCK_SKEW_TOLERANCE_SECS
        } else {
            0
        }
    }

    pub open spec fn validity_window_end(ticket: TimestampState) -> u64 {
        ticket.expires_at_secs
    }

    /// Check if current time is within validity window
    pub open spec fn in_validity_window(
        ticket: TimestampState,
        current_time_secs: u64,
    ) -> bool {
        current_time_secs >= validity_window_start(ticket) &&
        current_time_secs <= validity_window_end(ticket)
    }

    // ========================================================================
    // Ticket Creation
    // ========================================================================

    /// Create timestamp state for a new ticket
    pub open spec fn create_timestamp(
        current_time_secs: u64,
        validity_secs: u64,
    ) -> TimestampState {
        TimestampState {
            issued_at_secs: current_time_secs,
            expires_at_secs: current_time_secs + validity_secs,
        }
    }

    /// Proof: Fresh tickets are immediately valid
    pub proof fn fresh_ticket_is_valid(
        current_time_secs: u64,
        validity_secs: u64,
    )
        requires validity_secs > 0
        ensures {
            let ticket = create_timestamp(current_time_secs, validity_secs);
            timestamp_validity(ticket, current_time_secs)
        }
    {
        let ticket = create_timestamp(current_time_secs, validity_secs);
        // not_expired: expires_at = current + validity >= current
        assert(ticket.expires_at_secs >= current_time_secs);
        // within_clock_skew: issued_at = current <= current + tolerance
        assert(ticket.issued_at_secs <= current_time_secs + CLOCK_SKEW_TOLERANCE_SECS);
        // valid_duration: issued_at = current <= current + validity = expires_at
        assert(ticket.issued_at_secs <= ticket.expires_at_secs);
    }

    /// Proof: Tickets remain valid during their window
    pub proof fn ticket_valid_during_window(
        ticket: TimestampState,
        check_time: u64,
    )
        requires
            valid_duration(ticket),
            check_time >= ticket.issued_at_secs,
            check_time <= ticket.expires_at_secs,
        ensures
            not_expired(ticket, check_time),
            within_clock_skew(ticket, check_time),
    {
        // not_expired: check_time <= expires_at
        assert(ticket.expires_at_secs >= check_time);
        // within_clock_skew: issued_at <= check_time <= check_time + tolerance
        assert(ticket.issued_at_secs <= check_time + CLOCK_SKEW_TOLERANCE_SECS);
    }

    // ========================================================================
    // Expiration Behavior
    // ========================================================================

    /// Ticket expires at the instant after expires_at
    pub open spec fn is_expired(
        ticket: TimestampState,
        current_time_secs: u64,
    ) -> bool {
        current_time_secs > ticket.expires_at_secs
    }

    /// Proof: Expired tickets fail validation
    pub proof fn expired_tickets_fail(
        ticket: TimestampState,
        current_time_secs: u64,
    )
        requires is_expired(ticket, current_time_secs)
        ensures !not_expired(ticket, current_time_secs)
    {
        // current > expires_at means expires_at < current
        // not_expired requires expires_at >= current
    }

    /// Proof: Expiration is monotonic
    ///
    /// Once expired, a ticket stays expired.
    pub proof fn expiration_is_monotonic(
        ticket: TimestampState,
        time1: u64,
        time2: u64,
    )
        requires
            time1 <= time2,
            is_expired(ticket, time1),
        ensures is_expired(ticket, time2)
    {
        // time1 > expires_at and time2 >= time1
        // therefore time2 > expires_at
    }

    // ========================================================================
    // Clock Skew Handling
    // ========================================================================

    /// Future-issued tickets (beyond tolerance) are rejected
    pub open spec fn is_future_issued(
        ticket: TimestampState,
        current_time_secs: u64,
        tolerance_secs: u64,
    ) -> bool {
        ticket.issued_at_secs > current_time_secs + tolerance_secs
    }

    /// Proof: Future tickets fail validation
    pub proof fn future_tickets_fail(
        ticket: TimestampState,
        current_time_secs: u64,
    )
        requires is_future_issued(ticket, current_time_secs, CLOCK_SKEW_TOLERANCE_SECS)
        ensures !within_clock_skew(ticket, current_time_secs)
    {
        // issued_at > current + tolerance means
        // not (issued_at <= current + tolerance)
    }

    /// Proof: Reasonable clock skew is tolerated
    pub proof fn clock_skew_tolerated(
        ticket: TimestampState,
        issuer_time: u64,
        verifier_time: u64,
        skew_secs: u64,
    )
        requires
            skew_secs <= CLOCK_SKEW_TOLERANCE_SECS,
            ticket.issued_at_secs == issuer_time,
            // Issuer is ahead of verifier by skew_secs
            issuer_time == verifier_time + skew_secs,
        ensures within_clock_skew(ticket, verifier_time)
    {
        // issued_at = verifier_time + skew <= verifier_time + tolerance
    }

    // ========================================================================
    // Validity Duration Bounds
    // ========================================================================

    /// Ticket validity duration in seconds
    pub open spec fn validity_duration(ticket: TimestampState) -> u64 {
        if ticket.expires_at_secs >= ticket.issued_at_secs {
            ticket.expires_at_secs - ticket.issued_at_secs
        } else {
            0  // Invalid ticket has 0 duration
        }
    }

    /// Reasonable validity duration (not too short, not too long)
    pub open spec fn reasonable_validity(ticket: TimestampState) -> bool {
        let duration = validity_duration(ticket);
        duration >= MIN_TICKET_VALIDITY_SECS &&
        duration <= MAX_TICKET_VALIDITY_SECS
    }

    /// Proof: Default validity is reasonable
    pub proof fn default_validity_is_reasonable(current_time_secs: u64)
        ensures {
            let ticket = create_timestamp(current_time_secs, DEFAULT_TICKET_VALIDITY_SECS);
            reasonable_validity(ticket)
        }
    {
        // DEFAULT_TICKET_VALIDITY_SECS = 86400 (24 hours)
        // MIN = 60, MAX = 30 * 86400 = 2592000
        // 60 <= 86400 <= 2592000
    }
}
