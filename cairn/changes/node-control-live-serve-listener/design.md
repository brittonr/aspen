# Design: Node Control Live Serve Listener

## Overview
`serve --live-iroh` runs a bounded live-listener prelude before the existing supervised drain. The listener subscribes to the node-control gossip topic, records neighbor/session observations, receives up to the configured event bound, and passes received messages to `receive_node_control_live_ingress_event`. After the listener bound or timeout, it calls the existing `serve_node_control` drain so dispatch still flows through the durable inbox and control loop.

## Receipt
`node-control-live-listener-receipt-v1` binds decision, startup, node, logical node endpoint id, bound Iroh endpoint id, topic, event limit, observed event count, transport receipt refs, neighbor events, service run receipt ref, diagnostics, and checks.

## Loopback
A local loopback helper creates two Iroh endpoints with a shared memory address lookup. The receiver runs the live listener on a gossip topic while the sender broadcasts a live node-control ingress envelope. The listener stores and delivers the envelope, then the supervised drain dispatches the queued request.

## Safety
The listener never dispatches directly. It records transport evidence, stores canonical bytes through the live receive path, and delegates enqueue decisions to the existing ingress gates. Authority, resource, policy, delivery idempotency, provenance, source gates, and shutdown semantics remain unchanged.
