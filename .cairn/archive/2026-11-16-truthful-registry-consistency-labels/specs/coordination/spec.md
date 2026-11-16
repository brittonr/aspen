# Coordination Delta

## MODIFIED Requirements

### Requirement: Service registry state is coordination state
r[molten.coordination.service_registry] Molten MUST represent service demand, readiness, health, exposed refs, registry updates, and discovery as coordination requests and status assertions served through control-plane commit or committed-read evidence.

#### Scenario: Registry read binds committed-read evidence
- GIVEN a service endpoint registration has committed
- WHEN a reader requests the service registry entry with `local-committed` consistency
- THEN Molten emits a receipt bound to the control registry's committed-read receipt
- AND the status assertion contains the endpoint ref.

### Requirement: Coordination mutations use control-plane contracts
r[molten.coordination.raft_contract] Coordination mutations MUST execute through an admitted control-plane state-machine contract with commit receipts, and coordination reads MUST bind the control registry's committed state by default. The coordination runtime's contract is the in-process control registry; coordination receipts MUST NOT claim replication or read-index evidence it does not produce.

#### Scenario: Queue mutation commits through control plane
- GIVEN an admitted queue enqueue request
- WHEN Molten applies the request
- THEN the coordination receipt binds a control-plane commit receipt
- AND the status assertion is published only after the commit.

### Requirement: Coordination invariants are bounded and checkable
r[molten.coordination.trellis_predicates] Molten MUST expose bounded, deterministic checks for fencing monotonicity, mutual exclusion, FIFO queue behavior, semaphore bounds, committed-state binding of reads, and no-stale-token acceptance in coordination receipts or property fixtures. A coordination receipt MUST NOT report `pass` for a check whose property the runtime did not compute.

#### Scenario: Fencing invariant is checked
- GIVEN a generated coordination run with lock acquisition and stale release attempts
- WHEN the property fixture evaluates the run
- THEN stale tokens are denied
- AND the status assertion reflects the released state.

#### Scenario: Committed-state check passes only for admitted committed reads
- GIVEN coordination reads in `local-committed`, `local-stale` and denied modes
- WHEN Molten emits their receipts
- THEN `committed-state-bound` passes only for the admitted `local-committed` read.

### Requirement: Coordination reads declare consistency mode
r[molten.coordination.read_consistency_modes] Molten MUST carry an explicit read consistency mode on coordination read requests, receipts, and status assertions. Coordination requests default to `local-committed`: the committed state of the coordination runtime's in-process control registry, authoritative only inside that runtime. Because that registry has no replicated read index, the coordination runtime MUST deny `linearizable` for every operation with an explicit diagnostic. Local-stale reads MAY be emitted only as non-authoritative observations.

#### Scenario: Coordination read defaults to local-committed evidence
- GIVEN a coordination client reads a service registry entry, lock state, queue state, semaphore state, rate-limit state, election state, or barrier state without requesting stale diagnostics
- WHEN Molten serves the read
- THEN the coordination receipt binds the control registry's committed-read receipt and passes `committed-state-bound`
- AND the status assertion identifies the `local-committed` mode.

#### Scenario: Linearizable coordination request is denied
- GIVEN a coordination read or mutation requesting `linearizable` consistency
- WHEN the in-process coordination runtime admits it
- THEN Molten emits a deny receipt naming the missing replicated read index
- AND no state advances and no status assertion is published.

#### Scenario: Local-stale coordination read is labeled
- GIVEN an operator requests a local-stale coordination status read for diagnostics
- WHEN Molten serves the read from local state
- THEN the receipt and status assertion mark the result as local-stale
- AND the result is not accepted as current coordination authority.

### Requirement: Local-stale coordination reads cannot authorize protected actions
r[molten.coordination.local_stale_boundaries] Molten MUST reject local-stale coordination read receipts wherever a decision requires current state, including mutation admission, lock ownership, fencing-token validation, release gates, election leadership, barrier release, rate-limit enforcement, membership admission, or production pass evidence. Protected coordination operations MUST be admitted only with `local-committed` consistency.

#### Scenario: Stale lock read cannot release a lock
- GIVEN a client presents a local-stale read showing it as lock owner
- WHEN the client attempts a protected release or mutation
- THEN Molten denies the request unless separate committed, commit, or fencing evidence is present
- AND diagnostics identify the stale-read boundary.

#### Scenario: Stale registry read cannot satisfy admission
- GIVEN a policy or service admission gate requires the current service registry pointer
- WHEN the gate is given only a local-stale registry receipt
- THEN the gate denies currentness
- AND requires `local-committed` coordination evidence.

#### Scenario: Protected operation is admitted with local-committed
- GIVEN a lock acquire request with `local-committed` consistency and complete authority, policy and resource evidence
- WHEN Molten applies it
- THEN the acquire commits and issues a fencing token
- AND the same request with `local-stale` or `linearizable` consistency is denied without changing state.

### Requirement: Service registry pointers are control-plane state
r[molten.coordination_services_control_plane.service_registry] Service registry pointer updates MUST execute through control-plane commits and bind endpoint evidence refs.

#### Scenario: Service registration is read through committed state
- GIVEN a service registry pointer has committed
- WHEN a `local-committed` read request is served
- THEN Molten emits a committed-read receipt
- AND the dataspace assertion contains the endpoint ref.
