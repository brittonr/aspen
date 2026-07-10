# Node Runtime

## ADDED Requirements

### Requirement: Deployment profiles declare review tier

r[molten.prod_ops.release_profile.tiers] Molten SHOULD distinguish development, pilot, and release deployment profile tiers so fixture evidence, pilot evidence, and release-candidate evidence carry different validation expectations.

#### Scenario: Pilot profile remains pilot scoped

- GIVEN a deployment profile intended for local or pilot review
- WHEN the profile is exported or bound into production-readiness receipts
- THEN the profile tier records that it is development or pilot scoped
- AND the profile cannot be presented as release-tier evidence without passing release-tier validation.

#### Scenario: Release profile requires release validation

- GIVEN an operator prepares a release-candidate deployment profile
- WHEN the profile is exported for release review
- THEN release-tier validation requires the stricter evidence fields and placeholder-ref denial rules.

### Requirement: Release profiles reject placeholder refs

r[molten.prod_ops.release_profile.no_placeholder_refs] Release-tier deployment profiles MUST reject all-zero BLAKE3 refs, repeated-character dummy refs, declared fixture placeholders, and other configured placeholder evidence refs before they can be bound into release-readiness receipts.

#### Scenario: Non-placeholder release refs pass

- GIVEN a release-tier profile whose source-gate, policy, Octet, Cairn, stack-provenance, and production-profile refs are reviewed non-placeholder BLAKE3 refs
- WHEN release profile validation runs
- THEN validation passes and records the accepted refs as release-review inputs.

#### Scenario: Zero source-gate ref denies release profile

- GIVEN a release-tier profile whose source-gate input is `blake3:0000000000000000000000000000000000000000000000000000000000000000`
- WHEN release profile validation runs
- THEN validation denies the profile before release-readiness evidence can treat it as current.

#### Scenario: Dummy repeated ref denies release profile

- GIVEN a release-tier profile whose evidence input uses an obvious repeated-character fixture ref
- WHEN release profile validation runs
- THEN validation denies with a placeholder-ref diagnostic naming the affected field.

### Requirement: Release profiles bind freshness evidence

r[molten.prod_ops.release_profile.freshness] Release-tier deployment profile validation MUST bind current source-gate, policy, Octet, Cairn, generated-export, and profile content refs and MUST deny stale or missing refs before promotion evidence treats the profile as release-ready.

#### Scenario: Fresh release profile validates

- GIVEN a release-tier profile whose generated export, source-gate evidence, Octet evidence, Cairn validation evidence, and policy refs match the source candidate under review
- WHEN release profile freshness validation runs
- THEN validation passes and records the matched refs.

#### Scenario: Stale generated profile denies

- GIVEN a release-tier profile export whose generated JSON ref no longer matches the reviewed Nickel source
- WHEN release profile freshness validation runs
- THEN validation denies with expected and actual profile refs.

### Requirement: Release profile behavior has fixtures

r[molten.prod_ops.release_profile.fixtures] Release profile hardening SHOULD include positive fixtures for development, pilot, and release tiers plus negative fixtures for zero refs, dummy refs, stale refs, missing required evidence, optional stack provenance in release mode, and unsupported tier values.

#### Scenario: Negative fixture denies placeholder release evidence

- GIVEN a negative release profile fixture containing placeholder source-gate or policy refs
- WHEN fixture validation runs
- THEN the fixture fails before generated release evidence can be refreshed.

#### Scenario: Positive pilot fixture remains diagnostic scoped

- GIVEN a pilot profile fixture with local or synthetic evidence refs
- WHEN fixture validation runs
- THEN it may pass as pilot evidence
- AND it is not counted as release-tier evidence.
