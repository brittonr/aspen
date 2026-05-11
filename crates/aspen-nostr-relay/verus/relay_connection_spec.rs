//! Verus specs for Nostr relay connection-count admission.
//!
//! Production `src/relay.rs` owns TCP accept, HTTP peeking, WebSocket
//! handshakes, spawned task lifetimes, and atomics. Production
//! `src/iroh_transport.rs` owns semaphore permits for QUIC/iroh sessions. This
//! module verifies the pure resource-accounting kernel around those shells:
//! max-connection admission, active-count increment/decrement balance, and
//! semaphore permit acquire/release bounds.

use vstd::prelude::*;

verus! {

pub const MAX_NOSTR_CONNECTIONS: u32 = 256;
pub const MAX_IROH_CONNECTIONS: u32 = 128;

pub enum TcpAdmissionSpec {
    Accept,
    RejectLimit,
}

pub enum TcpConnectionStepSpec {
    RejectedBeforeHandshake,
    AcceptedAwaitingHandshake,
    HandshakeFailedReleased,
    SpawnedConnection,
    ConnectionFinishedReleased,
}

pub enum IrohAdmissionSpec {
    PermitAcquired,
    RejectNoPermits,
}

pub open spec fn tcp_connection_admission(active: u32, max_connections: u32) -> TcpAdmissionSpec {
    if active < max_connections {
        TcpAdmissionSpec::Accept
    } else {
        TcpAdmissionSpec::RejectLimit
    }
}

pub open spec fn tcp_admitted(active: u32, max_connections: u32) -> bool {
    tcp_connection_admission(active, max_connections) == TcpAdmissionSpec::Accept
}

pub open spec fn active_after_tcp_accept(active: u32, max_connections: u32) -> int {
    if tcp_admitted(active, max_connections) {
        active as int + 1
    } else {
        active as int
    }
}

pub open spec fn active_after_release(active: u32) -> int {
    if active > 0 {
        active as int - 1
    } else {
        0
    }
}

pub open spec fn tcp_lifecycle_step(
    initial_active: u32,
    max_connections: u32,
    handshake_success: bool,
    connection_finished: bool,
) -> TcpConnectionStepSpec {
    if !tcp_admitted(initial_active, max_connections) {
        TcpConnectionStepSpec::RejectedBeforeHandshake
    } else if !handshake_success {
        TcpConnectionStepSpec::HandshakeFailedReleased
    } else if connection_finished {
        TcpConnectionStepSpec::ConnectionFinishedReleased
    } else {
        TcpConnectionStepSpec::SpawnedConnection
    }
}

pub open spec fn active_after_tcp_lifecycle(
    initial_active: u32,
    max_connections: u32,
    handshake_success: bool,
    connection_finished: bool,
) -> int {
    if !tcp_admitted(initial_active, max_connections) {
        initial_active as int
    } else if !handshake_success || connection_finished {
        initial_active as int
    } else {
        initial_active as int + 1
    }
}

pub open spec fn iroh_connection_admission(available_permits: u32) -> IrohAdmissionSpec {
    if available_permits > 0 {
        IrohAdmissionSpec::PermitAcquired
    } else {
        IrohAdmissionSpec::RejectNoPermits
    }
}

pub open spec fn iroh_admitted(available_permits: u32) -> bool {
    iroh_connection_admission(available_permits) == IrohAdmissionSpec::PermitAcquired
}

pub open spec fn permits_after_iroh_accept(available_permits: u32) -> int {
    if iroh_admitted(available_permits) {
        available_permits as int - 1
    } else {
        0
    }
}

pub open spec fn permits_after_iroh_release(available_permits_after_accept: u32, max_permits: u32) -> int {
    if available_permits_after_accept < max_permits {
        available_permits_after_accept as int + 1
    } else {
        available_permits_after_accept as int
    }
}

pub fn tcp_connection_admission_exec(active: u32, max_connections: u32) -> (admission: TcpAdmissionSpec)
    ensures admission == tcp_connection_admission(active, max_connections)
{
    if active < max_connections {
        TcpAdmissionSpec::Accept
    } else {
        TcpAdmissionSpec::RejectLimit
    }
}

pub fn active_after_accepted_tcp_exec(active: u32, max_connections: u32) -> (next_active: u32)
    requires active < max_connections
    ensures
        next_active as int == active as int + 1,
        next_active as int <= max_connections as int,
{
    active + 1
}

pub fn active_after_release_exec(active: u32) -> (next_active: u32)
    ensures next_active as int == active_after_release(active)
{
    if active > 0 {
        active - 1
    } else {
        0
    }
}

pub fn iroh_connection_admission_exec(available_permits: u32) -> (admission: IrohAdmissionSpec)
    ensures admission == iroh_connection_admission(available_permits)
{
    if available_permits > 0 {
        IrohAdmissionSpec::PermitAcquired
    } else {
        IrohAdmissionSpec::RejectNoPermits
    }
}

pub fn permits_after_iroh_accept_exec(available_permits: u32) -> (next_permits: u32)
    ensures next_permits as int == permits_after_iroh_accept(available_permits)
{
    if available_permits > 0 {
        available_permits - 1
    } else {
        0
    }
}

pub fn permits_after_iroh_release_exec(available_permits_after_accept: u32, max_permits: u32) -> (next_permits: u32)
    requires available_permits_after_accept <= max_permits
    ensures
        next_permits as int == permits_after_iroh_release(available_permits_after_accept, max_permits),
        next_permits as int <= max_permits as int,
{
    if available_permits_after_accept < max_permits {
        available_permits_after_accept + 1
    } else {
        available_permits_after_accept
    }
}

pub proof fn default_tcp_limit_is_positive()
    ensures MAX_NOSTR_CONNECTIONS > 0
{
}

pub proof fn default_iroh_limit_is_positive()
    ensures MAX_IROH_CONNECTIONS > 0
{
}

pub proof fn tcp_rejects_at_or_above_limit(active: u32, max_connections: u32)
    requires active >= max_connections
    ensures tcp_connection_admission(active, max_connections) == TcpAdmissionSpec::RejectLimit
{
}

pub proof fn tcp_accepts_below_limit(active: u32, max_connections: u32)
    requires active < max_connections
    ensures tcp_connection_admission(active, max_connections) == TcpAdmissionSpec::Accept
{
}

pub proof fn zero_tcp_limit_rejects_all(active: u32)
    ensures tcp_connection_admission(active, 0) == TcpAdmissionSpec::RejectLimit
{
}

pub proof fn admitted_tcp_increment_stays_within_limit(active: u32, max_connections: u32)
    requires active < max_connections
    ensures
        active_after_tcp_accept(active, max_connections) == active as int + 1,
        active_after_tcp_accept(active, max_connections) <= max_connections as int,
{
}

pub proof fn rejected_tcp_does_not_change_active(active: u32, max_connections: u32)
    requires active >= max_connections
    ensures active_after_tcp_accept(active, max_connections) == active as int
{
}

pub proof fn handshake_failure_releases_tcp_slot(active: u32, max_connections: u32, connection_finished: bool)
    requires active < max_connections
    ensures
        tcp_lifecycle_step(active, max_connections, false, connection_finished)
            == TcpConnectionStepSpec::HandshakeFailedReleased,
        active_after_tcp_lifecycle(active, max_connections, false, connection_finished) == active as int,
{
}

pub proof fn completed_tcp_connection_releases_slot(active: u32, max_connections: u32)
    requires active < max_connections
    ensures
        tcp_lifecycle_step(active, max_connections, true, true)
            == TcpConnectionStepSpec::ConnectionFinishedReleased,
        active_after_tcp_lifecycle(active, max_connections, true, true) == active as int,
{
}

pub proof fn live_tcp_connection_accounts_for_one_slot(active: u32, max_connections: u32)
    requires active < max_connections
    ensures
        tcp_lifecycle_step(active, max_connections, true, false) == TcpConnectionStepSpec::SpawnedConnection,
        active_after_tcp_lifecycle(active, max_connections, true, false) == active as int + 1,
        active_after_tcp_lifecycle(active, max_connections, true, false) <= max_connections as int,
{
}

pub proof fn tcp_rejected_before_handshake_does_not_change_active(
    active: u32,
    max_connections: u32,
    handshake_success: bool,
    connection_finished: bool,
)
    requires active >= max_connections
    ensures
        tcp_lifecycle_step(active, max_connections, handshake_success, connection_finished)
            == TcpConnectionStepSpec::RejectedBeforeHandshake,
        active_after_tcp_lifecycle(active, max_connections, handshake_success, connection_finished) == active as int,
{
}

pub proof fn release_never_underflows(active: u32)
    ensures active_after_release(active) >= 0
{
}

pub proof fn positive_release_decrements(active: u32)
    requires active > 0
    ensures active_after_release(active) == active as int - 1
{
}

pub proof fn zero_release_stays_zero()
    ensures active_after_release(0) == 0
{
}

pub proof fn iroh_rejects_when_no_permits()
    ensures iroh_connection_admission(0) == IrohAdmissionSpec::RejectNoPermits
{
}

pub proof fn iroh_accepts_when_permit_available(available_permits: u32)
    requires available_permits > 0
    ensures iroh_connection_admission(available_permits) == IrohAdmissionSpec::PermitAcquired
{
}

pub proof fn iroh_accept_consumes_exactly_one_permit(available_permits: u32)
    requires available_permits > 0
    ensures permits_after_iroh_accept(available_permits) == available_permits as int - 1
{
}

pub proof fn iroh_reject_consumes_no_permits()
    ensures permits_after_iroh_accept(0) == 0
{
}

pub proof fn iroh_release_restores_consumed_permit(available_before_accept: u32, max_permits: u32)
    requires
        available_before_accept > 0,
        available_before_accept <= max_permits,
    ensures permits_after_iroh_release((available_before_accept - 1) as u32, max_permits)
        == available_before_accept as int
{
}

pub proof fn iroh_permit_count_stays_within_max(available_after_accept: u32, max_permits: u32)
    requires available_after_accept <= max_permits
    ensures permits_after_iroh_release(available_after_accept, max_permits) <= max_permits as int
{
}

} // verus!
