# Batch renderer delta

## ADDED Requirements

### Requirement: Complete render identity
r[molten.batch_render.identity] The render manifest MUST bind versioned scene/schema, camera, dimensions, sampling/RNG/seed, bounce limit, partition, renderer content identity, runtime/task profile and output format. Assignments MUST derive identity from the complete manifest and exact region.

#### Scenario: Changed settings
- GIVEN an accepted tile from another seed, scene, component or runtime profile
- WHEN supplied to assembly
- THEN assembly rejects it rather than reusing incompatible output.

### Requirement: Bounded verified image
r[molten.batch_render.assembly] The renderer MUST use at most 63 tiles and one assembly task within existing object/input/retention bounds. Assembly MUST reject missing, overlapping, malformed, stale or substituted output and produce manifest-bound RGB8.

#### Scenario: Forged pixels
- GIVEN a tile with altered pixels and a recomputed payload checksum
- WHEN assembly validates the tile
- THEN expected deterministic pixel verification rejects the substitution.

### Requirement: Evidence boundaries
r[molten.batch_render.evidence] Authored deterministic and separate-process fixtures MUST be executable through existing batch transport/runtime. Documentation MUST distinguish authored coverage from executed same-host and physical-host qualification.

#### Scenario: Unexecuted fixture
- GIVEN a script or test has only been authored
- WHEN reporting qualification
- THEN it is not described as a passing execution or cross-host evidence.
