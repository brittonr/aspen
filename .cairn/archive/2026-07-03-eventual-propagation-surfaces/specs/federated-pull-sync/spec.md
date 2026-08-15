## ADDED Requirements

### Requirement: Anti-entropy status is explicit local state
r[molten.eventual_surface.anti_entropy_status] Molten SHOULD represent anti-entropy query results, missing-set calculations, fetch plans, imports, denials, and peer availability as local dataspace assertions with canonical evidence refs.

#### Scenario: Missing set is visible without importing trust
- GIVEN a peer advertises inventory for a federated surface
- WHEN the receiver computes its missing set
- THEN Molten records local status assertions for missing, already-present, denied, and planned fetch refs
- AND those assertions do not import remote content until verification and admission pass.

### Requirement: Remote sync imports require verification and admission
r[molten.eventual_surface.remote_sync_boundary] Molten MUST treat federation announcements, gossip hints, docs observations, and inventory records as propagation hints that cannot import artifacts or mutate registries without receiver-driven verification and local admission.

#### Scenario: Announcement alone cannot import artifact
- GIVEN a peer announces an artifact ref over gossip or federation inventory
- WHEN the receiver has not fetched, hash-verified, and admitted the artifact
- THEN the artifact remains unimported
- AND any status assertion records the announcement as a hint rather than local trust.

### Requirement: Eventual surface validation is reproducible
r[molten.eventual_surface.validation] Molten SHOULD validate eventual propagation surfaces with focused merge-law tests, remote dataspace tests, federation anti-entropy tests, remote sync boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Hint-only import regression fails
- GIVEN a regression imports an artifact from a federation announcement without content verification
- WHEN focused eventual-surface validation runs
- THEN the negative remote-sync boundary fixture fails
- AND the change cannot complete until receiver-driven verification is restored.
