# Operator Workflow Delta

## ADDED Requirements

### Requirement: Pilot release freezes one immutable source candidate

r[molten.prod_release.pilot_candidate_freeze] Molten MUST identify a limited-pilot release candidate by an immutable Git commit, its Git tree, and a domain-separated BLAKE3 source reference before generating release evidence.

#### Scenario: Clean candidate source is frozen

- GIVEN a reviewed Git commit and its exact Git tree
- WHEN operators frame and hash the candidate identity
- THEN the recorded BLAKE3 reference binds both values
- AND candidate checks run from a clean detached checkout of that commit.

#### Scenario: Changed or dirty source denies publication

- GIVEN a checkout whose commit, tree, tracked files, or candidate reference differs from the frozen identity
- WHEN operators prepare candidate evidence
- THEN publication is denied before a release tag is created.

### Requirement: Limited-pilot publication requires fresh bounded evidence

r[molten.prod_release.pilot_evidence_publication] Molten MUST require fresh candidate-bound Rust, nextest, Nix, Cairn, Octet, VM, dogfood, bundle, promotion, export, profile, pilot-decision, and candidate-gate evidence before limited-pilot publication. A warning-only Octet result MUST remain a deny receipt and MAY support only the named pilot when its configuration identity is current and the pilot carries the source-gate caveat.

#### Scenario: Complete candidate evidence permits pilot publication

- GIVEN every required artifact passes its own gate
- AND every artifact binds the frozen candidate source
- AND the pilot decision names allowed workloads, exclusions, rollback triggers, stop conditions, and caveats
- WHEN operators review the release package
- THEN they may publish the limited internal pilot tag.

#### Scenario: Missing or weak evidence denies publication

- GIVEN a required artifact is missing, skipped, unavailable, stale, mismatched, fixture-only, or diagnostic-only
- OR warning-only Octet evidence has stale configuration identity or lacks the required pilot caveat
- WHEN operators review the release package
- THEN publication is denied
- AND the exact evidence blocker remains visible.

### Requirement: Pilot publication preserves production non-claims

r[molten.prod_release.pilot_non_claims] Molten MUST label the `0.1.0` publication as a limited internal pilot and MUST NOT claim general production readiness.

#### Scenario: Pilot notes state bounded scope

- GIVEN the candidate evidence passes
- WHEN release notes and the pilot decision are published
- THEN they state the supported internal workloads and operator controls
- AND they exclude real-WAN, sustained-SLO, fleet-scale, adversarial-security, production-consensus, and destructive-operation claims.
