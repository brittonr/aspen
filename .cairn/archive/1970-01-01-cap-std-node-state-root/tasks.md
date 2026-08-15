## Phase 1: Node-state authority model

- [x] [serial] Add `NodeStateRoot`, typed namespace views, and pure fixed/validated relative locator constructors for identity, ledger, control, ingress, service, receipts, and secrets. r[molten.node.cap_std_state_root] r[molten.node.cap_std_namespaces]
- [x] [serial] Open or create the node state root once in the outer CLI/daemon shell and thread authority through synchronous and asynchronous node operations. r[molten.node.cap_std_state_root] r[molten.node.cap_std_async_authority]

## Phase 2: State, queue, and lock cutover

- [x] [parallel] Convert state-layout creation, Preserves reads/writes, ledger imports, delivery-idempotency state, and receipt storage to capability-relative namespace operations. r[molten.node.cap_std_namespaces]
- [x] [serial] Replace inbox scans and raw request paths with bounded relative entry identities; archive and remove requests through the inbox capability without lexical containment checks. r[molten.node.cap_std_request_lifecycle]
- [x] [parallel] Convert startup, service, shutdown, heartbeat, stale-lock recovery, and duplicate-runner lock effects to capability-relative operations. r[molten.node.cap_std_request_lifecycle] r[molten.node.cap_std_namespaces]

## Phase 3: Endpoint identity and secret hardening

- [x] [serial] Convert endpoint-id and persisted-secret create, load, permission inspection, rotation, and receipt effects to the identity capability while preserving platform-specific restriction diagnostics. r[molten.node.cap_std_identity_secret]
- [x] [parallel] Add positive tests for first boot, persisted identity reuse, admitted rotation, queue dispatch, service restart, and nested ledger/store operations under one node capability. r[molten.node.cap_std_validation]
- [x] [parallel] Add negative tests for symlinked secret and state leaves, non-regular secret files, unsafe permissions, root replacement, wrong-root handles, stale queue entries, request substitution, and lock replacement. r[molten.node.cap_std_validation]

## Phase 4: Enforcement and documentation

- [x] [serial] Add node-state ast-grep fixtures and a blocking rule that rejects ambient descendant I/O while permitting explicit node-root bootstrap and separate CLI input reads. r[molten.node.cap_std_regression_gate]
- [x] [parallel] Document node-state authority flow, namespace views, process-path bridges, platform permission limits, receipt path redaction, and non-claims. r[molten.node.cap_std_state_root] r[molten.node.cap_std_identity_secret]

## Phase 5: Validation

- [x] [serial] Run focused node daemon, control queue, service supervision, ledger, endpoint identity, live-ingress, and structural-authority positive and negative tests. r[molten.node.cap_std_validation] r[molten.node.cap_std_regression_gate]
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and relevant node and Nix checks before sync and archive. r[molten.node.cap_std_validation]
