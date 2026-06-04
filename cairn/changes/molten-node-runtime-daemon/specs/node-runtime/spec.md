# Node Runtime Delta: Durable Molten Node Process

### Requirement: Node startup evidence is canonical
r[molten.node_runtime_daemon.spec.startup_evidence] A Molten node MUST start only from canonical config evidence that binds identity, state roots, adapter profiles, policy refs, capability refs, resource refs, effect profile refs, and runtime version refs.

#### Scenario: Explicit config starts
- GIVEN a `node-config-v1` with explicit state roots and admitted authority/resource refs
- WHEN `molten node run` starts
- THEN it emits a `node-startup-receipt-v1` with decision `pass`
- AND the receipt binds the config ref and adapter startup receipts

#### Scenario: Implicit state is denied
- GIVEN a node startup command using default ambient filesystem paths as semantic identity
- WHEN startup validation runs
- THEN the node emits a denial receipt
- AND no production adapter starts

### Requirement: Control commands are receipt-backed
r[molten.node_runtime_daemon.spec.control_receipts] Local node control commands MUST cross the node boundary as canonical Preserves requests and return canonical receipts binding caller authority, request ref, sub-receipts, and decision.

#### Scenario: Authorized status request passes
- GIVEN a running node and an admitted status authority context
- WHEN a `node-status-request-v1` is submitted
- THEN the node emits a `node-control-receipt-v1` with decision `pass`
- AND the rendered status text is derived from that receipt

#### Scenario: Unknown control schema denies
- GIVEN a control request with an unknown schema
- WHEN the node control adapter receives it
- THEN the request is denied before any sub-operation runs

### Requirement: Adapter lifecycle is explicit
r[molten.node_runtime_daemon.spec.adapter_lifecycle] Node adapters MUST start, verify, shut down, and recover through canonical adapter receipts, not invisible process state.

#### Scenario: Adapter index rebuild is evidenced
- GIVEN a node state root with a rebuildable Redb-backed index
- WHEN the node starts
- THEN it verifies or rebuilds the index
- AND records an adapter receipt with the derived head/status refs

#### Scenario: Restart checks prior shutdown
- GIVEN a prior startup receipt and no clean shutdown receipt
- WHEN the node restarts
- THEN recovery emits diagnostics and verifies adapters before reporting healthy

### Requirement: Node config DTO is explicit
r[molten.node_runtime_daemon.config_dto] Node config evidence MUST use canonical `node-config-v1` records with explicit identity, state-root/profile, adapter, policy, capability, resource, and effect-profile refs.

#### Scenario: Init writes config
- GIVEN `molten node init --state-root <root> --node-id <id>`
- WHEN the init command succeeds
- THEN it writes a `node-config-v1` artifact under the explicit state root
- AND optional `--config-out` output preserves the same canonical config ref.

### Requirement: Startup receipt binds runtime evidence
r[molten.node_runtime_daemon.startup_receipt] Node startup MUST emit `node-startup-receipt-v1` binding config, identity, ordered adapter receipts, source-gate validation refs, version refs, capability refs, and resource refs.

#### Scenario: Run writes startup receipt
- GIVEN an initialized explicit node state root
- WHEN `molten node run` succeeds
- THEN it writes a startup receipt with decision `pass`
- AND the startup receipt binds all required adapter start receipts.

### Requirement: Ambient state is denied
r[molten.node_runtime_daemon.no_ambient_state] Node startup and init MUST reject ambient filesystem identity such as current-directory semantic roots.

#### Scenario: Ambient root rejected
- GIVEN the state root is `.`
- WHEN node init validation runs
- THEN startup state is denied before adapter startup.

### Requirement: Node artifacts are classified
r[molten.node_runtime_daemon.ledger_classification] Node config, startup, adapter, control, health, and shutdown artifacts MUST classify in ledger/catalog views by stable artifact kind.

#### Scenario: CLI outputs classify
- GIVEN node init/run/status/stop output files
- WHEN ledger artifact classification runs
- THEN each file is recognized as its node artifact kind.

### Requirement: Adapter order is deterministic
r[molten.node_runtime_daemon.adapter_order] Node runtime startup MUST start ledger, registry, chunk store, typed storage, eval cache, remote dataspace, service supervision, job runtime, coordination, plugin-host, catalog/MCP, and control adapters in deterministic dependency order.

#### Scenario: Scrambled profiles normalize
- GIVEN adapter profiles in any accepted order
- WHEN startup runs
- THEN adapter receipts appear in required dependency order.

### Requirement: Adapter receipts cover lifecycle decisions
r[molten.node_runtime_daemon.adapter_receipts] Adapter start, verify, deny, and shutdown decisions MUST be represented by canonical `node-adapter-receipt-v1` records.

#### Scenario: Shutdown emits adapter receipts
- GIVEN a running node
- WHEN `molten node stop` runs
- THEN shutdown adapter receipts are recorded before the node shutdown receipt.

### Requirement: Adapter indexes are verified before control
r[molten.node_runtime_daemon.index_verify] Node startup and control health MUST bind index verification or rebuild receipt refs before admitting control requests.

#### Scenario: Status binds indexes
- GIVEN a running node
- WHEN `molten node status` runs
- THEN the health receipt includes index receipt refs.

### Requirement: Resource profiles bind runtime budgets
r[molten.node_runtime_daemon.resource_profile] Startup and adapter receipts MUST bind resource profile receipt refs before runtime work is admitted.

#### Scenario: Resource refs present
- GIVEN an initialized local node
- WHEN startup runs
- THEN startup and adapter receipts include resource receipt refs.

### Requirement: Control request DTOs are canonical
r[molten.node_runtime_daemon.control_request] Local status, install, run, gate, and shutdown commands MUST be represented by `node-control-request-v1` and decided by `node-control-receipt-v1`.

#### Scenario: Status request receipt
- GIVEN a running node
- WHEN status is requested
- THEN a control receipt binds the canonical status request ref.

### Requirement: Local control uses Preserves boundary
r[molten.node_runtime_daemon.preserves_control] The local control surface MUST accept and emit canonical Preserves request/receipt records; rendered text is non-normative.

#### Scenario: Show summarizes artifact
- GIVEN a node receipt artifact
- WHEN `molten node show` runs
- THEN summary text is derived from the canonical Preserves artifact.

### Requirement: Control authority is gated
r[molten.node_runtime_daemon.authority_gate] Every control request MUST bind authority, policy, effect-handle or capability, and resource evidence before passing.

#### Scenario: Missing evidence denial
- GIVEN a control request without required evidence
- WHEN denial is emitted
- THEN the control receipt has decision `deny` and diagnostic text.

### Requirement: Control receipts bind subreceipts
r[molten.node_runtime_daemon.subreceipt_binding] Control receipts MUST bind subreceipt refs produced by artifact, job, remote, storage, gate, health, or shutdown operations.

#### Scenario: Stop binds shutdown
- GIVEN a stop control command
- WHEN shutdown succeeds
- THEN the control receipt includes the shutdown receipt ref as a subreceipt.

### Requirement: Graceful shutdown is receipted
r[molten.node_runtime_daemon.graceful_shutdown] Node shutdown MUST stop intake, drain admitted work, persist/close adapters, and emit a canonical shutdown receipt.

#### Scenario: Stop writes shutdown
- GIVEN a running node
- WHEN `molten node stop` runs
- THEN it writes `node-shutdown-receipt-v1` with decision `pass`.

### Requirement: Restart recovery verifies prior receipts
r[molten.node_runtime_daemon.restart_recovery] Restart health MUST verify previous startup and shutdown receipts, adapter refs, indexes, heads, and absence of open jobs before declaring replay eligibility.

#### Scenario: Clean restart eligible
- GIVEN a prior startup receipt and clean shutdown receipt
- WHEN restart health is evaluated
- THEN the health receipt decision is `pass` and replay status is eligible.

### Requirement: Health receipts describe replay status
r[molten.node_runtime_daemon.health_receipts] Node health receipts MUST bind adapter refs, index refs, head refs, open job refs, shutdown refs, and replay eligibility.

#### Scenario: Running status writes health
- GIVEN a running node
- WHEN status is requested
- THEN the node writes a health receipt and a control receipt.

### Requirement: Node runtime daemon tests cover lifecycle
r[molten.node_runtime_daemon.tests] The node runtime daemon slice MUST include library and CLI tests for init, run, status, control denial, shutdown, and restart recovery.

#### Scenario: CLI lifecycle test
- GIVEN the CLI integration suite
- WHEN the node daemon lifecycle test runs
- THEN init, run, status, stop, and stopped status all succeed with canonical artifacts.
