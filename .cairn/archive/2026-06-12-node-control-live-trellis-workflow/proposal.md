# Change: Node Control Live Trellis Workflow Gate

## Motivation

Live workflow bundles now carry sender-side handoff, gate, apply, receiver reconcile, and ack evidence, but operators still need one finite-protocol gate that proves the completed workflow followed the intended sender/receiver order. Hashes of the bundle receipts are not enough: the handoff must be replayed as a Trellis-shaped lifecycle before it is archived as completed remote-control evidence.

## Proposed Change

Add `molten node live-workflow-bundle-protocol-gate`. The command reads a workflow bundle, bundle gate receipt, apply receipt, reconcile receipt, and ack artifact; projects them through a finite sender/receiver Trellis protocol (`bundle-handoff`, `apply-evidence`, `ack-evidence`); and emits a `protocol-session-gate-receipt-v1`. The gate also denies when workflow evidence is missing, malformed, denying, or mismatched even if the protocol shape itself replays.

## Non-Goals

- Protocol gate receipts do not grant node-control authority, peer bootstrap, policy/resource rights, provenance, or transport trust.
- The command does not import bundle members, send live Iroh messages, dispatch receiver control requests, or mutate node state.
- The Trellis gate does not replace existing bundle verify/gate/apply/reconcile/ack checks; it only gates completed workflow evidence for review/archive.
