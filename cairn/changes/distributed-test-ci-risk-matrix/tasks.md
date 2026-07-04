# Tasks: distributed-test-ci-risk-matrix

## Phase 1: Matrix model and surfaces

- [ ] [serial] r[molten.testing.distributed_ci.profile_matrix] Define the distributed test risk/cost matrix and expose each profile through documented nextest, Nix, app, or release-readiness surfaces.
- [ ] [parallel] r[molten.testing.distributed_ci.metadata_binding] Add canonical distributed test metadata binding for source, Nix inputs, test binary, shard/profile, seed, topology, fault plan, receipt refs, and variance declarations.

## Phase 2: Gates and retry policy

- [ ] [serial] r[molten.testing.distributed_ci.traceability_required_gate] Require traceability coverage for evidence-bearing distributed requirements during release/CI review, including positive and negative evidence.
- [ ] [serial] r[molten.testing.distributed_ci.retry_policy] Enforce zero retries for CI/release pass evidence and separate exploratory retry/quarantine diagnostics from authoritative pass evidence.
- [ ] [parallel] r[molten.testing.distributed_ci.unavailable_handling] Ensure skipped or unsupported VM/fault/soak profiles record unavailable evidence and cannot satisfy pass claims.

## Phase 3: Fixtures and documentation

- [ ] [parallel] r[molten.testing.distributed_ci.negative_fixtures] Add negative fixtures for missing shard artifacts, stale traceability refs, missing negative coverage, retry-only success, and undeclared variance.
- [ ] [serial] r[molten.testing.distributed_ci.docs] Document the matrix, commands, evidence claims, retry boundary, and how release reviewers inspect refs.
