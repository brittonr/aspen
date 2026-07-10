## ADDED Requirements

### Requirement: VM shard metadata is scoped separately from executable VM evidence
r[molten.testing.vm_shard_scope.synthetic_metadata_boundary] Molten MUST classify VM shard receipts by explicit evidence scope so synthetic fixture metadata, executable NixOS VM observations, aggregate indexes, unavailable evidence, and diagnostic logs cannot be confused as the same platform claim.

#### Scenario: Synthetic shard metadata remains metadata evidence
- GIVEN a shard receipt produced from synthetic refs and fixture metadata only
- WHEN the shard is evaluated for a platform execution claim
- THEN the evaluator classifies it as fixture metadata or diagnostic evidence
- AND it does not satisfy executable VM pass evidence.

#### Scenario: Executable VM shard binds real child receipts
- GIVEN a shard produced by NixOS VM execution
- WHEN platform evidence is accepted
- THEN the shard binds executable VM child receipt refs and host-support state
- AND logs remain diagnostic-only attachments.

### Requirement: VM aggregates preserve child evidence scope
r[molten.testing.vm_shard_scope.aggregate_scope_denial] Molten MUST preserve each child shard's evidence scope in aggregate receipts and MUST deny aggregate pass claims that promote synthetic refs, unavailable evidence, retry-only success, or log-only outputs into executable platform evidence.

#### Scenario: Aggregate denies synthetic platform promotion
- GIVEN an aggregate requiring executable VM evidence
- WHEN one required child shard is backed only by synthetic metadata refs
- THEN the aggregate decision is deny for the executable platform claim
- AND diagnostics name the child shard and scope mismatch.

#### Scenario: Unavailable support is not pass evidence
- GIVEN a VM shard whose host support is unavailable
- WHEN the aggregate evaluates platform readiness
- THEN unavailable evidence remains unavailable or diagnostic according to policy
- AND it is not counted as a passing executable shard.
