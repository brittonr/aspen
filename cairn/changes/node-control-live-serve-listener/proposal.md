# Proposal: Node Control Live Serve Listener

## Summary
Begin wiring live Iroh node-control transport into the supervised `serve` path with bounded listener receipts and a local two-node listener loopback.

## Motivation
Molten can publish and receive live Iroh node-control envelopes in a loopback helper, but the operator-facing serve path still only scans local ingress directories. The next step is a bounded listener that accepts live gossip events, stores them through the same ingress boundary, then drains through the supervised control loop.

## Scope
- Canonical live listener receipt artifacts.
- `molten node serve --live-iroh` bounded listener mode.
- Live gossip event processing into `receive_node_control_live_ingress_event` before normal service drain.
- Neighbor/session diagnostics in listener receipts.
- Local two-endpoint loopback coverage where one endpoint publishes and the listener endpoint enqueues and dispatches through serve.

## Out of Scope
- Public daemon socket management or external relay bootstrap UX.
- Unbounded always-on listener process.
- New node-control operations or dispatch bypass.
