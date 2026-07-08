## ADDED Requirements

### Requirement: Consensus manifests declare algorithm profiles
r[molten.consensus.algorithm_profile_manifest] Molten MUST represent each control-plane consensus group with an explicit algorithm profile, admitted profile version, read-consistency support, quorum rule, membership policy refs, placement refs, fault-model caveats, and required evidence refs. The current Raft-backed profile MUST remain the default admitted production profile until another profile has separate accepted implementation, proof, simulation, and policy evidence.

#### Scenario: Existing Raft group remains default
- GIVEN a control-plane group manifest that targets the currently admitted production behavior
- WHEN Molten loads the manifest
- THEN the manifest resolves to the Raft-backed profile
- AND the group still requires the existing Raft membership, read-index, commit, snapshot, and receipt evidence.

#### Scenario: Unknown algorithm profile denies
- GIVEN a group manifest names an unknown, unsupported, or misspelled algorithm profile
- WHEN group installation or readback validates the manifest
- THEN Molten denies the group before installation
- AND diagnostics identify the unsupported profile without falling back to another consensus algorithm.

### Requirement: Leaderless quorum profile is experimental and gated
r[molten.consensus.leaderless_profile_boundary] Molten MAY define a Meerkat/QuePaxa-inspired leaderless quorum profile only as an explicit experimental control-plane profile. The profile MUST deny production admission unless policy, proof/model evidence, deterministic simulation evidence, placement evidence, membership evidence, and operator caveat receipts all pass.

#### Scenario: Experimental profile requires evidence
- GIVEN a manifest requests the experimental leaderless quorum profile
- WHEN required proof, simulation, placement, membership, or policy evidence is missing
- THEN Molten denies production admission
- AND the profile can be inspected only as experimental or diagnostic evidence.

#### Scenario: Any-replica proposal is profile-scoped
- GIVEN the experimental profile is admitted for a deterministic simulation run
- WHEN a majority-connected non-leader replica proposes a control-plane command
- THEN the command can progress only through the profile's quorum rule
- AND the emitted commit receipt names the proposing replica, quorum evidence, and experimental caveat.

### Requirement: Consensus reads declare consistency mode
r[molten.consensus.read_consistency_modes] Molten MUST classify every control-plane read request and read receipt as `linearizable` or `local-stale`. Linearizable reads MUST bind read-index evidence or admitted algorithm-specific quorum evidence. Local-stale reads MAY expose local diagnostics but MUST NOT satisfy mutation guards, release gates, policy currentness checks, membership admission, or production pass evidence.

#### Scenario: Linearizable read has quorum-backed freshness
- GIVEN a client requests a linearizable control-plane read
- WHEN Molten serves the read
- THEN the receipt binds read-index or algorithm-specific quorum freshness evidence
- AND the returned state ref is at least as fresh as the admitted read evidence.

#### Scenario: Local-stale read is visibly non-authoritative
- GIVEN a client requests a local-stale control-plane read from a replica that may lag
- WHEN Molten serves the read
- THEN the receipt marks the response as local-stale
- AND downstream gates reject that receipt wherever linearizable currentness is required.

### Requirement: Consensus placement evidence is explicit
r[molten.consensus.replica_placement_evidence] Molten SHOULD emit canonical placement reports for consensus groups that bind group id, candidate members, admitted members, fault-domain policy, membership refs, placement policy refs, expected majority-reachability assumptions, latency diagnostics, denied candidates, and refresh requirements.

#### Scenario: Placement report binds selected members
- GIVEN an operator plans a control-plane consensus group
- WHEN placement evaluation runs
- THEN the placement report names the selected members, policy refs, membership evidence, fault-domain diagnostics, and majority-reachability assumptions
- AND the report ref is available to the group manifest or install receipt.

#### Scenario: Unsafe placement denies installation
- GIVEN a candidate placement concentrates members in a disallowed fault domain or lacks required placement evidence
- WHEN group installation validates the placement refs
- THEN Molten denies installation before consensus state is created
- AND diagnostics identify the missing or unsafe placement condition.

### Requirement: Consensus applications use deterministic canonical logs
r[molten.consensus.canonical_log_applications] Molten MUST model control-plane applications as deterministic state machines over canonical command/log envelopes. Application code MUST NOT infer consensus semantics from transport, leader identity, replica locality, or local cache state outside the admitted log/read evidence.

#### Scenario: Same log gives same application state
- GIVEN two replicas have the same admitted command log and application manifest
- WHEN each replica applies the log through the pure state-machine boundary
- THEN they produce the same state ref, application receipt refs, and status assertion refs.

#### Scenario: Local cache cannot bypass log evidence
- GIVEN a replica has local cached control-plane state but no admitted commit or read evidence for a requested current value
- WHEN a mutation guard or release gate evaluates the cache result
- THEN the gate denies the cache result as insufficient evidence
- AND requires the relevant log, commit, or linearizable read receipt.

### Requirement: Consensus non-claim boundaries are recorded
r[molten.consensus.non_claim_boundaries] Molten MUST record explicit non-claim boundaries for consensus groups: no Byzantine tolerance, no general-purpose database guarantee, no ordinary actor-message ordering, no global dataspace, no blob/gossip/job transport ordering, no lease-read semantics without accepted timing assumptions, and no production leaderless profile without accepted evidence.

#### Scenario: Unsupported use is diagnostic-only
- GIVEN a report tries to use consensus evidence as proof of Byzantine tolerance, general database correctness, ordinary actor ordering, global dataspace consistency, or transport delivery correctness
- WHEN evidence gates evaluate the report
- THEN the report cannot satisfy those claims
- AND diagnostics name the consensus non-claim boundary that applies.

#### Scenario: Lease read remains denied without timing policy
- GIVEN a group manifest lacks accepted lease timing assumptions and policy evidence
- WHEN a replica attempts to serve a control-plane read as a lease read
- THEN Molten denies the lease-read path
- AND requires linearizable read evidence or a visibly local-stale receipt.
