## Why

Molten already models receiver-driven federation, signed announcements, inventories, missing sets, local admission, and hint-only discovery, but the current path uses local fixture signatures and filesystem loopback. Federation should be an optional long-lived system extension rather than transport behavior or a global runtime mode.

The extension needs production cryptographic identity, bounded anti-entropy sessions, content/DAG synchronization, local policy admission, and explicit rate/resource controls.

## What Changes

- Add a canonical federated-pull system-extension manifest and supervised anti-entropy lifecycle.
- Sign and verify canonical announcements, inventories, delegates, requests, and responses through admitted cryptographic identity adapters.
- Treat static peers, endpoint observations, trackers, pkarr-style pointers, gossip, and catalog records as candidate-location hints only.
- Compute receiver-owned missing sets and fetch plans, then use DAG/content synchronization before local verification and admission.
- Enforce per-peer and per-resource query, byte, concurrency, retry, and freshness bounds through fabric time and resource ports.
- Expose bounded local status assertions and conflict diagnostics without push import, global consensus, or automatic merge authority.

## Impact

- **Files**: federation extension runtime, crypto/content/DAG/transport bindings, peer configuration, rate-limit state, status assertions, operator workflows, fixtures, and `cairn/specs/federated-pull-sync/spec.md`.
- **Testing**: signed pull success, restart/resume, stale inventory, wrong key/purpose, revoked delegate, resource exhaustion, malicious peer, unsolicited content, conflict, partition, live/sim parity, and no-push-import tests.
- **Safety**: successful transport, discovery, signature, or fetch is not local trust; import still requires content verification, capability, policy, resource, provenance, retention, and schema admission.
- **Licensing**: Aspen `main` federation behavior is reference material only unless code has an explicit compatible license or relicense grant.
