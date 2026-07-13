## Phase 1: Federation extension and signed models

- [ ] [serial] Define the federated-pull system-extension manifest, peer configuration, protocol/session, announcement, inventory, delegate, request/response, conflict, status, and evidence records. r[molten.federated_pull_runtime.manifest]
- [ ] [serial] Replace production federation fixture signatures with purpose-separated cryptographic adapter operations over canonical payload refs while retaining fixture profiles for simulation only. r[molten.federated_pull_runtime.crypto]
- [ ] [parallel] Add positive and negative signature tests for current key, wrong key, wrong purpose/domain, stale generation, revocation, malformed records, and redaction. r[molten.federated_pull_runtime.crypto]

## Phase 2: Discovery and receiver-owned pull

- [ ] [serial] Implement static configured peers as the initial discovery profile and represent optional endpoint, gossip, tracker, DHT/pkarr-style, probe, and catalog inputs as hint-only candidate locators. r[molten.federated_pull_runtime.discovery_hints]
- [ ] [serial] Implement pure peer selection, inventory diff, missing-set, DAG/content strategy, freshness, rate, resource, conflict, and local-admission prerequisite planning. r[molten.federated_pull_runtime.receiver_pull] r[molten.federated_pull_runtime.bounds_freshness] r[molten.federated_pull_runtime.conflict_boundary]
- [ ] [parallel] Reject push import, unsolicited content, oversized inventory, stale signatures, unavailable peers, and unadmitted domain merges before trusted local mutation. r[molten.federated_pull_runtime.discovery_hints] r[molten.federated_pull_runtime.receiver_pull] r[molten.federated_pull_runtime.conflict_boundary]

## Phase 3: Executable anti-entropy service

- [ ] [serial] Host bounded anti-entropy sessions as a supervised system extension using crypto, transport, DAG, content, time, resource, and observability bindings. r[molten.federated_pull_runtime.manifest] r[molten.federated_pull_runtime.bounds_freshness]
- [ ] [serial] Fetch through DAG/content extensions, verify canonical identity, and invoke local schema, policy, capability, provenance, resource, retention, and import admission before registry mutation. r[molten.federated_pull_runtime.receiver_pull]
- [ ] [parallel] Add local status assertions and bounded operator readback for peers, freshness, sessions, inventories, missing sets, fetches, verification, admission, denial, conflicts, resources, and evidence. r[molten.federated_pull_runtime.status_evidence]

## Phase 4: Simulation, live tests, and validation

- [ ] [parallel] Run the same federation core under deterministic simulation with malicious-peer, stale inventory, partition, restart, resource exhaustion, signature, corruption, conflict, and unsolicited-push faults. r[molten.federated_pull_runtime.final_validation]
- [ ] [parallel] Add live-loopback and local multiprocess signed pull/import fixtures with offline-verifiable receipts. r[molten.federated_pull_runtime.final_validation]
- [ ] [serial] Run focused federation, crypto, transport, DAG/content, admission, rate/resource, simulation/live parity, no-push-import, and conflict-boundary tests. r[molten.federated_pull_runtime.final_validation]
- [ ] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.federated_pull_runtime.final_validation]

## Blocker

This package depends on both `dag-sync-system-extension` and
`fabric-whole-system-simulation`, which are blocked transitively by the missing
live consistency transport shell. The anti-entropy service cannot produce its
required same-core malicious-peer, restart, partition, and multiprocess evidence
until those dependencies are completed. Resume afterward; do not bypass DAG
admission or use filesystem loopback as live federation evidence.
