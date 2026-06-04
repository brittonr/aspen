# Design: Node Control Live Iroh Transport

## Overview
The live transport reuses `node-control-ingress-envelope-v1` with transport profile `iroh-gossip`. A sender broadcasts the canonical envelope bytes on an Iroh gossip topic derived from the node-control topic. A receiver parses only canonical bytes, validates topic/target/transport binding, writes a live transport receipt, stores the envelope in the existing ingress directory, and invokes the existing ingress delivery function.

## Receipts
`node-control-live-transport-receipt-v1` binds operation (`publish` or `receive`), decision, live transport profile, topic, local node, delivered-from endpoint when available, envelope ref, optional ingress receipt ref, diagnostics, and checks for canonical envelope identity, live Iroh gossip, peer bootstrap evidence, transport-not-authority, and durable inbox boundary.

## Loopback harness
`molten node control-ingress-live-loopback` creates two in-process Iroh endpoints with a memory address lookup, joins a gossip topic, broadcasts a live ingress envelope, receives the gossip event, and feeds it into the durable ingress path. This exercises the real `iroh-gossip` API without requiring a long-running public listener.

## Safety
The live receiver does not dispatch. It stores the envelope and calls `deliver_node_control_ingress`; peer bootstrap, authority, policy, resource, delivery idempotency, provenance, and operation gates remain authoritative. Transport delivery is evidence, not trust.
