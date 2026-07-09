# Peer session transition relation

Peer sessions are transport and bootstrap state, not operation authority. Session changes now flow through a pure finite transition relation that takes a prior record, event, target state, observed topic, freshness tick, and guard evidence refs, then returns a pass/deny decision with next or preserved state.

The reviewed relation admits normal progression through invite, handshake, negotiation, admission, connection, expiry, revocation, quarantine, and evidence-backed recovery. Invalid skips, wrong-topic evidence, missing bootstrap admission, missing authority, stale expiry ticks, missing revocation evidence, terminal exits, and quarantine bypasses deny without advancing state.

Transition receipts bind prior state, event, target state, next or preserved state, before/after state refs, guard refs, diagnostics, and checks. ALPN/transport reachability, handoff observations, and connected-session facts remain evidence-only until capability, authority, policy, and resource gates pass independently.
