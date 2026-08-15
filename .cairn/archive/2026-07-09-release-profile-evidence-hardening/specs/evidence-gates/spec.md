# Evidence Gates

## ADDED Requirements

### Requirement: Stack provenance is required for release tier

r[molten.evidence.stack_provenance.release_required] Release-tier evidence profiles MUST require a stack-provenance input that is current for the source candidate under review while preserving the rule that stack provenance is evidence-only and does not grant subsystem authority.

#### Scenario: Release profile has stack provenance

- GIVEN a release-tier profile with a current stack-provenance envelope and matching release candidate refs
- WHEN release evidence validation runs
- THEN validation records the stack-provenance ref and its evidence-only non-claims.

#### Scenario: Optional stack provenance denies release tier

- GIVEN release-tier policy config marks stack provenance as optional or omits the stack-provenance input
- WHEN release evidence validation runs
- THEN validation denies release-tier readiness before promotion evidence can pass.

#### Scenario: Stack provenance remains non-authority

- GIVEN release evidence validation accepts a stack-provenance input
- WHEN a subsystem later requires authority, policy, provenance, transport, source-gate, retention, resource, or execution trust
- THEN that subsystem still requires its own matching gates and MUST NOT treat stack provenance as authority.

### Requirement: Accepted stack policy hashes are reviewed non-placeholders

r[molten.evidence.stack_provenance.non_placeholder_hashes] Release-tier stack-provenance configuration MUST reject placeholder accepted Valence policy hashes and MUST record reviewed non-placeholder BLAKE3 policy digests.

#### Scenario: Reviewed Valence policy hash passes

- GIVEN stack-provenance release config lists a reviewed non-placeholder Valence policy hash
- WHEN release evidence validation checks the stack-provenance gate
- THEN the hash is accepted as a release-review input.

#### Scenario: Placeholder Valence policy hash denies

- GIVEN stack-provenance release config lists `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa` or another configured placeholder digest
- WHEN release evidence validation checks the stack-provenance gate
- THEN validation denies and reports the placeholder policy hash.
