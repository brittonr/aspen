## Phase 1: Manifest model

- [x] [serial] r[molten.testing.requirement_traceability.manifest] Define the deterministic requirement coverage manifest schema and generator inputs.
- [x] [parallel] r[molten.testing.requirement_traceability.operator_summary] Add a compact human-readable summary grouped by covered, exempt, missing-positive, missing-negative, and stale-reference requirements.

## Phase 2: Coverage gate

- [x] [serial] r[molten.testing.requirement_traceability.coverage_gate] Add a gate that requires positive and negative coverage for evidence-bearing or changed requirements unless a documented exemption applies.
- [x] [serial] r[molten.testing.requirement_traceability.stale_detection] Fail closed on stale requirement ids, missing test targets, missing commands, or missing evidence artifact refs.

## Phase 3: Fixtures, Nix, and docs

- [x] [parallel] r[molten.testing.requirement_traceability.fixtures] Add positive and negative fixtures for complete coverage, missing positive coverage, missing negative coverage, stale refs, and documented exemptions.
- [x] [serial] r[molten.testing.requirement_traceability.nix_surface] Expose the traceability gate through an explicit Nix or Cairn validation command used by release evidence review.
- [x] [parallel] r[molten.testing.requirement_traceability.docs] Document how to read and update traceability evidence when adding or changing requirements.
