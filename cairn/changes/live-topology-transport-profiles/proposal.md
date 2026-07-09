## Why

Molten's live Iroh/node-control workflows are explicit and receipt-backed, but operators still configure topology details by repeating node IDs, topics, peer expectations, endpoint guards, tickets, and timeout flags across commands. As deployments move beyond local loopback fixtures, live topology should be described by reviewed profiles rather than command transcripts alone.

A live topology/transport profile can make intended peers, topics, endpoint constraints, relay/direct preferences, ALPN surfaces, and retry policies reviewable while preserving the rule that transport identity never grants authority.

## What Changes

- Define live topology profiles for nodes, peers, topics, endpoint expectations, allowed ALPNs, and ticket/peer-admission requirements.
- Define transport policy profiles for retry attempts, join/publish timeouts, relay/direct preferences, and diagnostic expectations under runtime hard caps.
- Add command support to use topology/transport profiles for live-ticket export/import, live-send, live-listener, and workflow-bundle apply paths.
- Bind selected topology and transport profile refs into live receipts and effective-config readbacks.
- Add positive and negative tests for expected topology, wrong peer/topic/endpoint, unsupported ALPN, stale ticket, and transport-as-authority attempts.

## Impact

- **Files**: peer/bootstrap specs, node live transport inputs, Iroh router/readback surfaces, CLI node live commands, docs, and tests.
- **Testing**: pure topology admission tests plus CLI/live-loopback tests with positive and negative profile fixtures.
- **Safety**: topology and transport profiles are evidence and constraints only. They do not grant authority, policy, resource, provenance, source-gate trust, retention clearance, or execution permission.
