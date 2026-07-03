# Design: protocol session transition proof

## Scope

This change proves the projected protocol endpoint state machine. It covers manifest lowering, projectability evidence, local endpoint actions, send/receive/branch/offer operation receipts, session lifecycle gate receipts, and replay against terminal states.

## Proof checklist

- **Proof claim**: endpoint operation receipts are accepted only when they match the projected local state and advance to the expected next state; lifecycle gates accept only replayable, complete, ordered session evidence.
- **Out of scope**: network transport reliability, carrier-specific delivery, and distributed scheduling.
- **Trusted assumptions**: Trellis projectability/projection results are deterministic for the lowered global choreography.
- **Positive evidence**: valid request/response and branching sessions replay from initial endpoint states to terminal states with passing gate receipts.
- **Negative evidence**: wrong role, wrong label, wrong peer, missing message evidence, ambiguous branch, stale prior state, and missing terminal evidence deny.
- **Canonical refs**: proof traces bind install receipts, operation receipts, protocol ids, session ids, message refs, state refs, and diagnostics.
- **Regeneration command**: `cargo test protocol`.

## Functional core

Transition matching should be expressed as pure checks over projected local state, prior state, operation evidence, and next state. Transport adapters should only supply canonical message evidence after the transition gate accepts it.

## Non-goals

- No claim that a carrier delivers messages exactly once.
- No expansion of supported nested protocol forms unless a later change extends projection semantics.
