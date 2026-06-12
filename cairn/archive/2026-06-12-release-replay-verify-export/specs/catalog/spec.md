# Catalog Specification Delta

## Requirements

### Requirement: Release outputs include raw replay verify evidence
r[molten.release.replay_verify_export.local_output] Molten SHOULD emit raw generic deterministic replay verify receipts from local dogfood release outputs alongside replay indexes.

#### Scenario: Local dogfood writes replay verify evidence
- GIVEN a passing local dogfood run
- WHEN the operator requests a replay verify output path
- THEN Molten writes a `deterministic-replay-verify-v1` receipt
- AND the receipt remains evidence-only release review material

### Requirement: Release readback binds replay verify refs
r[molten.release.replay_verify_export.readback] Molten SHOULD bind replay verify refs in Nix dogfood release evidence and verification receipts, and deny missing, stale, malformed, tampered, or index-mismatched replay verify evidence.

#### Scenario: Replay index must contain replay verify ref
- GIVEN Nix dogfood release evidence with a replay verify receipt and replay index
- WHEN readback validates the output path
- THEN the replay index must list the replay verify ref
- AND mismatches deny release readback

### Requirement: Release bundles include replay verify members
r[molten.release.replay_verify_export.bundle] Molten SHOULD include replay verify Preserves members in release bundles and signed-member verification.

#### Scenario: Required signed members include replay verify
- GIVEN release bundle verification with signed members required
- WHEN the replay verify member lacks a valid signed receipt
- THEN bundle verification denies

### Requirement: Release exports include replay verify members
r[molten.release.replay_verify_export.archive] Molten SHOULD include replay verify Preserves and signed replay verify members in release export manifests and archive verification.

#### Scenario: Export archive carries replay verify evidence
- GIVEN a passing release export
- WHEN the archive is inspected or verified
- THEN the archive contains replay verify Preserves and signed replay verify members
- AND tampered or missing replay verify members deny archive verification

### Requirement: Replay verify release export behavior is tested
r[molten.release.replay_verify_export.tests] Molten SHOULD test replay verify output, readback binding, signed bundle requirements, export membership, and evidence-only caveats.

#### Scenario: Replay verify remains evidence only
- GIVEN release evidence with replay verify and replay index refs
- WHEN release readback, bundle verification, and export verification pass
- THEN replay verify evidence remains evidence only
- AND it does not replace source, policy, provenance, Octet, Cairn, signed keyring, authority, resource, transport, retention, release promotion, or release acceptance checks
