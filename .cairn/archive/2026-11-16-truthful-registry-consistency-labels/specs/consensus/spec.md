# Consensus Delta

## MODIFIED Requirements

### Requirement: In-process control registry admission receipts
r[molten.consensus.trellis_raft_core] The in-process control registry MUST bind append consistency, local commit, commit advancement, committed reads and snapshot restore to bounded predicate receipts named for the property the registry computes, each carrying the `in-process-registry` evaluator check. A registry receipt MUST NOT claim Trellis evaluation, quorum commitment, quorum membership or read-index freshness that the registry did not compute, and receipt families whose meaning changes MUST change version.

#### Scenario: Append admission names its evaluator
- GIVEN an append request for the control registry
- WHEN Molten admits the request
- THEN the decision binds a `registry-append-consistency` predicate receipt over the command envelope with the `in-process-registry` and prior-log-binding checks
- AND the receipt names no Trellis predicate.

#### Scenario: Local commit names no quorum
- GIVEN an admitted control-registry command
- WHEN the in-process registry commits it
- THEN the commit predicate is `registry-local-commit` bound to the log entry
- AND neither the predicate nor the commit receipt lists quorum members or a quorum-commit check.

#### Scenario: Version-one registry receipt is rejected
- GIVEN a version-one registry predicate, read or commit receipt
- WHEN Molten parses it
- THEN the parser rejects it.

### Requirement: Linearizable read admission
r[molten.consensus.linearizable_reads] The replicated consensus service MUST support linearizable control-plane reads through read-index admission by default and MUST deny lease-read semantics unless explicit future manifest timing assumptions and policy admission are implemented. A leader MUST NOT take a read index until its commit index covers every entry an earlier leader may have committed: the entry at its commit index belongs to its current term, or its commit index reaches the last entry in its log. Until then a linearizable read MUST be reported retryable rather than served. The in-process control registry has no replicated read index and MUST deny a linearizable read with an explicit diagnostic rather than serve it.

#### Scenario: Read-index admits current read
- GIVEN a leader that has satisfied the configured read-index condition
- WHEN a client requests a linearizable read
- THEN the read can be served from state that is at least as recent as the admitted read index.

#### Scenario: Lease read without lease policy is denied
- GIVEN a group manifest that does not admit lease-read timing assumptions
- WHEN a node attempts to serve a read as a lease read
- THEN the runtime denies the lease-read path and requires read-index or another admitted linearizable read path.

#### Scenario: New leader has not committed a current-term entry
- GIVEN a previous leader committed and acknowledged a write that the new leader holds above its commit index
- WHEN a client requests a linearizable read from the new leader before any current-term entry commits
- THEN the leader reports the read retryable and registers no pending read
- AND after a current-term entry commits, the read index covers the acknowledged write.

#### Scenario: New leader already holds only committed entries
- GIVEN a new leader whose commit index reaches the last entry in its log, with no current-term entry committed
- WHEN a client requests a linearizable read
- THEN the leader takes its commit index as the read index, which covers every write an earlier leader acknowledged.

#### Scenario: In-process registry denies a linearizable read
- GIVEN an in-process control registry holding the requested entry
- WHEN a client requests a linearizable read from it
- THEN the read receipt is denied with the diagnostic that the in-process registry has no replicated read index
- AND the receipt binds no committed-read predicate.

### Requirement: Consensus reads declare consistency mode
r[molten.consensus.read_consistency_modes] Molten MUST classify every control-plane read request and read receipt as `linearizable`, `local-committed` or `local-stale`. Linearizable reads MUST bind read-index evidence or admitted algorithm-specific quorum evidence. A `local-committed` read MUST be served only by the in-process control registry, MUST bind that instance's committed state, term, index and group manifest, and is authoritative only inside that instance: it carries no replication or quorum read-index evidence. Local-stale reads MAY expose local diagnostics but MUST NOT satisfy mutation guards, release gates, policy currentness checks, membership admission, or production pass evidence.

#### Scenario: Linearizable read has quorum-backed freshness
- GIVEN a client requests a linearizable control-plane read from the replicated consensus service
- WHEN Molten serves the read
- THEN the receipt binds read-index or algorithm-specific quorum freshness evidence
- AND the returned state ref is at least as fresh as the admitted read evidence.

#### Scenario: Local-committed read binds the registry's committed state
- GIVEN an in-process control registry that has committed an entry
- WHEN a client requests a `local-committed` read of that entry
- THEN the receipt passes and binds the registry's current committed state, term, index and a `registry-committed-read` predicate
- AND after a further commit, a new read binds the new state and index.

#### Scenario: Local-stale read is visibly non-authoritative
- GIVEN a client requests a local-stale control-plane read from a replica that may lag
- WHEN Molten serves the read
- THEN the receipt marks the response as local-stale and binds no committed-read predicate
- AND downstream gates reject that receipt wherever linearizable currentness is required.

### Requirement: Consensus engines are registered by explicit profile identity
r[molten.consensus.engine_registry] Molten MUST maintain an explicit consensus engine registry keyed by algorithm profile id and admitted profile version. Each registry entry MUST declare implementation id, supported read consistency modes, currentness evidence classes, membership/config capabilities, production-admission status, required evidence refs, conformance receipt refs, and operator caveats. Supported modes and currentness classes MUST describe the selected implementation, not unimplemented algorithm guarantees. The in-process registry MUST declare `local-committed` and `local-stale` modes with local committed-state currentness and MUST NOT advertise linearizable reads or quorum read-index evidence.

#### Scenario: Registered Raft profile resolves
- GIVEN a control-plane group manifest names the admitted Raft algorithm profile and profile version
- WHEN runtime construction resolves the manifest through the consensus engine registry
- THEN the registry returns the selected engine descriptor
- AND the descriptor binds implemented read modes, currentness classes, membership capability, conformance refs, and production-admission status.

#### Scenario: Unknown engine profile denies
- GIVEN a control-plane group manifest names an unknown, disabled, or misspelled algorithm profile
- WHEN runtime construction resolves the manifest through the consensus engine registry
- THEN Molten denies runtime construction before opening control-plane state
- AND diagnostics name the unsupported profile without falling back to another engine.

#### Scenario: Local registry cannot be admitted for linearizable reads
- GIVEN the in-process registry engine and a model-environment admission request
- WHEN the request requires linearizable mode or capability
- THEN admission denies the unsupported mode or capability
- AND local-committed admission remains available without claiming quorum evidence.

