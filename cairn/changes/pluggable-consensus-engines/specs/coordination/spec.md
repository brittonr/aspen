## ADDED Requirements

### Requirement: Coordination consumes engine-agnostic currentness evidence
r[molten.coordination.engine_agnostic_evidence] Coordination services MUST consume normalized consensus commit, read, fencing, and currentness receipts rather than Raft-specific leader, term, index, or read-index internals. Coordination decisions MAY preserve engine-specific evidence refs for audit, but mutation admission and protected-action checks MUST evaluate the common evidence fields.

#### Scenario: Lock release uses normalized currentness
- GIVEN a client attempts to release a coordination lock after reading lock ownership through an admitted consensus engine
- WHEN coordination admission evaluates the release
- THEN it checks normalized currentness, fencing epoch, operation id, and resource evidence
- AND it does not require the backing engine to expose Raft-specific fields.

#### Scenario: Engine-specific receipt without normalized fields denies
- GIVEN a coordination request presents an engine-specific receipt that lacks normalized currentness or fencing fields
- WHEN coordination admission evaluates a protected action
- THEN admission denies the receipt as insufficient authority
- AND diagnostics identify the missing normalized consensus evidence.

### Requirement: Coordination switchover gates require active engine epoch
r[molten.coordination.engine_switchover_gates] Coordination services MUST reject mutation, release, election, barrier, rate-limit, registry, and membership decisions that rely on consensus receipts from inactive, superseded, or not-yet-activated engine epochs. Coordination status readback MUST show the active engine profile and epoch for protected control-plane state.

#### Scenario: Stale epoch cannot release lock
- GIVEN a consensus engine switchover has activated a new engine epoch for a coordination group
- WHEN a client presents a lock ownership receipt from the prior engine epoch
- THEN coordination denies the lock release or protected mutation
- AND diagnostics name the stale engine epoch and active engine profile.

#### Scenario: Status readback names active engine
- GIVEN a coordination service is backed by a pluggable consensus engine registry entry
- WHEN an operator requests service status
- THEN the status assertion names the active engine profile, profile version, engine epoch, read consistency mode, and currentness evidence ref
- AND local-stale status is still labeled non-authoritative.
