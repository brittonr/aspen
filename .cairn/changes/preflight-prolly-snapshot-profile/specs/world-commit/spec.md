# World Commit Specification Delta

## ADDED Requirements

### Requirement: Snapshot loads preflight the supplied Prolly profile

r[molten.prolly_map.snapshot_profile_preflight] The Prolly snapshot shell MUST validate the actual caller-supplied profile against core's existing profile contract before allocating traversal collections or reading any block. An invalid profile MUST produce the core's domain issues rather than a block-store observation. A valid profile at the existing maximum MUST remain admissible.

#### Scenario: Self-consistent profile exceeds the graph maximum

- GIVEN a profile whose identity matches its contents but whose graph-fact maximum exceeds the core maximum by one, and an absent root block
- WHEN the shell loads a snapshot
- THEN it MUST return the profile-bound domain issue without reading the block store

#### Scenario: Exact maximum reads an encoded snapshot

- GIVEN a valid profile at the core maximum and encoded blocks in the typed local store
- WHEN the shell loads a snapshot
- THEN it MUST return the validated snapshot
