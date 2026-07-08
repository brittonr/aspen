## ADDED Requirements

### Requirement: Coordination reads declare consistency mode
r[molten.coordination.read_consistency_modes] Molten MUST carry an explicit read consistency mode on coordination read requests, receipts, and status assertions. Coordination reads default to linearizable control-plane evidence; local-stale reads MAY be emitted only as non-authoritative observations.

#### Scenario: Coordination read defaults to linearizable evidence
- GIVEN a coordination client reads a service registry entry, lock state, queue state, semaphore state, rate-limit state, election state, or barrier state without requesting stale diagnostics
- WHEN Molten serves the read
- THEN the coordination receipt binds linearizable control-plane read evidence
- AND the status assertion identifies the read consistency mode.

#### Scenario: Local-stale coordination read is labeled
- GIVEN an operator requests a local-stale coordination status read for diagnostics
- WHEN Molten serves the read from local state
- THEN the receipt and status assertion mark the result as local-stale
- AND the result is not accepted as current coordination authority.

### Requirement: Local-stale coordination reads cannot authorize protected actions
r[molten.coordination.local_stale_boundaries] Molten MUST reject local-stale coordination read receipts wherever a decision requires current state, including mutation admission, lock ownership, fencing-token validation, release gates, election leadership, barrier release, rate-limit enforcement, membership admission, or production pass evidence.

#### Scenario: Stale lock read cannot release a lock
- GIVEN a client presents a local-stale read showing it as lock owner
- WHEN the client attempts a protected release or mutation
- THEN Molten denies the request unless separate linearizable read, commit, or fencing evidence is present
- AND diagnostics identify the stale-read boundary.

#### Scenario: Stale registry read cannot satisfy admission
- GIVEN a policy or service admission gate requires the current service registry pointer
- WHEN the gate is given only a local-stale registry receipt
- THEN the gate denies currentness
- AND requires linearizable coordination evidence.

### Requirement: Coordination supports explicit batched control-plane operations
r[molten.coordination.batched_control_plane_operations] Molten SHOULD support canonical batched or compare-and-swap-style coordination operation envelopes for low-write control-plane workflows. Batches MUST preserve per-operation ids, per-operation authority/policy/resource evidence, deterministic ordering, per-operation receipts, and a single enclosing control-plane commit or denial receipt.

#### Scenario: Valid batch commits deterministically
- GIVEN a batch contains admitted coordination operations with distinct operation ids and satisfied evidence
- WHEN the batch is applied through the control-plane state machine
- THEN Molten applies the operations in canonical batch order
- AND emits per-operation receipts plus an enclosing commit receipt.

#### Scenario: Invalid batch denies safely
- GIVEN a batch contains an operation with missing authority, stale compare input, duplicate operation id, unsupported primitive, or resource denial
- WHEN the batch is evaluated
- THEN Molten emits deterministic denial evidence for the affected operation or batch according to the manifest policy
- AND no undeclared partial mutation can be treated as committed.

### Requirement: Coordination remains small control-plane state
r[molten.coordination.small_control_plane_scope] Coordination services MUST remain scoped to small, explicit control-plane state such as locks, fencing tokens, queues, semaphores, rate limits, elections, barriers, and service registry pointers. They MUST NOT become the default storage path for job payloads, actor mailboxes, blob contents, gossip fanout, or ordinary dataspace state.

#### Scenario: Large payload stays out of coordination log
- GIVEN a job or actor request carries a large payload or ordinary message body
- WHEN the request references coordination services
- THEN coordination records carry only content refs, operation refs, or control-plane pointers where admitted
- AND the payload itself remains outside the consensus log.

#### Scenario: Ordinary queue traffic is not implied
- GIVEN ordinary actor or job traffic uses mailboxes, dataspaces, or job scheduling
- WHEN no explicit coordination request is present
- THEN Molten does not append a coordination control-plane command
- AND no coordination receipt claims authority over that ordinary traffic.
