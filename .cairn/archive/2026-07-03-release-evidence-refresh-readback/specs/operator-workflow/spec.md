## ADDED Requirements

### Requirement: Release evidence refresh binds one current candidate
r[molten.release_evidence_refresh.current_candidate_matrix] Release evidence refresh SHOULD bind Rust validation, hermetic nextest, dogfood local-node, replay evidence, release bundle verification, promotion, summary, export manifest, and export verification to the same candidate source tree and Nix input set.

#### Scenario: Candidate evidence is mutually current
- GIVEN a candidate tree after proof-affecting changes land
- WHEN release evidence refresh runs
- THEN every release-review artifact references the same candidate evidence graph
- AND stale evidence from a previous candidate is not reported as current.

### Requirement: Dogfood readback names refreshed evidence
r[molten.release_evidence_refresh.dogfood_readback] Dogfood release readback SHOULD name the refreshed dogfood report, release gate, replay verify, replay index, Nix evidence, Nix verify, bundle verify, promotion, summary, export manifest, and export verify refs needed for review.

#### Scenario: Readback follows dogfood output refs
- GIVEN a dogfood-local-node check output for the candidate
- WHEN an operator reads the release summary or README evidence notes
- THEN the listed refs and output paths identify the current dogfood evidence rather than a stale prior run.

### Requirement: Release bundle graph readback is verified
r[molten.release_evidence_refresh.bundle_graph] Release evidence refresh SHOULD verify release bundle members, signed members, promotion, signed promotion where available, summary, export manifest, archive, and export verification against one candidate evidence graph.

#### Scenario: Bundle graph readback follows one candidate
- GIVEN a refreshed dogfood local-node output for the candidate
- WHEN release bundle verification, promotion, summary, export, and export verification run
- THEN their refs identify artifacts from the same candidate evidence graph
- AND stale or mismatched members are not reported as current.

### Requirement: Refreshed release graph denies stale members
r[molten.release_evidence_refresh.stale_denial] Release evidence refresh MUST preserve deny behavior for missing, duplicate, stale, unsigned, wrong-purpose, wrong-signer, revoked, or tampered release members.

#### Scenario: Stale member cannot refresh evidence
- GIVEN a release bundle member from an older candidate
- WHEN bundle verification or release readback evaluates the refreshed graph
- THEN verification denies with stale or mismatched member diagnostics.

### Requirement: Release evidence notes are evidence-only
r[molten.release_evidence_refresh.docs] Release evidence documentation MUST state that refreshed readback artifacts remain review evidence only and do not grant authority, policy, provenance, source-gate, resource, transport, retention, destructive-operation, or deployment trust.

#### Scenario: Documentation preserves caveats
- GIVEN refreshed release evidence paths are added to operator notes
- WHEN a reviewer follows the notes
- THEN the notes identify canonical receipts and retain the evidence-only caveat for every rendered summary.
