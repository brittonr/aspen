# Catalog Specification Delta

## Requirements

### Requirement: Release gates bind replay indexes
r[molten.release.replay_index_binding.gate] Molten SHOULD bind replay evidence index refs into dogfood release gate evidence while preserving evidence-only semantics.

#### Scenario: Release gate carries replay index refs
- GIVEN a passing local dogfood run with generated replay index evidence
- WHEN a release gate receipt is emitted
- THEN the release gate records at least one replay evidence index ref
- AND the release gate records that replay index evidence is evidence-only

### Requirement: Release readback denies stale replay indexes
r[molten.release.replay_index_binding.readback] Molten SHOULD deny release readback when replay index evidence is missing, malformed, stale, tampered, or not bound by the release gate.

#### Scenario: Tampered replay index denies Nix release verification
- GIVEN Nix dogfood evidence that references a replay index
- WHEN the replay index file is replaced with non-index content
- THEN release verification emits a deny receipt with replay index diagnostics

### Requirement: Release bundles carry replay index members
r[molten.release.replay_index_binding.bundle] Molten SHOULD include replay index Preserves members in release evidence bundles, signed-member checks, and release export member verification.

#### Scenario: Required signed members include replay index
- GIVEN release bundle verification with signed members required
- WHEN the replay index member lacks a valid signed receipt
- THEN bundle verification denies

### Requirement: Release replay bindings are discoverable
r[molten.release.replay_index_binding.catalog] The catalog SHOULD classify release artifacts that bind replay indexes with replay release-binding classifications and replay index refs.

#### Scenario: Release binding is found by replay index ref
- GIVEN an imported release artifact that binds a replay index
- WHEN replay evidence MCP search filters by `stage=release-binding` and replay index ref
- THEN the release binding artifact is returned

### Requirement: Release replay binding behavior is tested
r[molten.release.replay_index_binding.tests] Molten SHOULD test replay index emission, stale/tampered readback denial, signed bundle requirements, catalog/MCP discovery, and evidence-only checks.

#### Scenario: Replay index remains evidence only
- GIVEN release evidence with a valid replay index
- WHEN release readback passes
- THEN the replay index remains evidence only
- AND it does not replace source, policy, provenance, Octet, Cairn, signed keyring, authority, resource, transport, release promotion, or harness gate checks
