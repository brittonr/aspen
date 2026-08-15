# Project Delta: Policy Authoring and Runtime Boundary

### Requirement: Policy authoring, export, runtime consumption, and freshness are layered
r[molten.project.policy_boundary.layered_policy] Repository policy systems SHOULD separate authoring-time contracts, deterministic generated exports, runtime consumption of checked artifacts, and freshness validation.

#### Scenario: Policy layer responsibility is clear
- GIVEN a policy source, generated policy artifact, runtime admission path, or freshness check
- WHEN reviewers inspect the implementation
- THEN the artifact is assigned to authoring, export, runtime consumption, or freshness validation

### Requirement: Runtime does not invoke live policy tooling as authority
r[molten.project.policy_boundary.runtime_no_live_tooling] Runtime admission MUST NOT invoke Nickel evaluation, Cairn policy export, or policy tooling availability as live authority; it MUST consume checked exports, canonical refs, or policy-gate receipts.

#### Scenario: Runtime consumes checked policy
- GIVEN runtime admission requires policy data
- WHEN admission evaluates a request
- THEN it consumes checked policy exports, explicit policy refs, or canonical policy-gate receipts without running Nickel or Cairn policy commands

#### Scenario: Live policy tooling attempt is rejected
- GIVEN runtime code attempts to run Nickel or Cairn policy tooling to decide live authority
- WHEN boundary validation runs
- THEN validation fails or records the violation before release evidence is promoted

### Requirement: Generated policy freshness is validated
r[molten.project.policy_boundary.fresh_generated_policy] Generated policy artifacts SHOULD be validated for freshness against reviewed source contracts and the current expected schema before promotion.

#### Scenario: Fresh generated policy passes
- GIVEN reviewed policy source and generated policy artifacts match the current schema
- WHEN freshness validation runs
- THEN validation passes and records the source and generated artifact identities

#### Scenario: Stale generated policy fails
- GIVEN generated policy JSON is missing required schema fields, has duplicate ids, contains stale refs, or diverges from reviewed source
- WHEN freshness validation runs
- THEN validation fails before runtime or release evidence treats the artifact as current

### Requirement: Policy boundary has positive and negative fixtures
r[molten.project.policy_boundary.tests] Policy boundary changes SHOULD include positive fixtures for valid fresh exports and negative fixtures for stale generated policy, missing schema fields, duplicate ids, bad refs, or runtime live-tooling violations.

#### Scenario: Missing schema field fixture fails
- GIVEN a generated policy fixture omits a required current schema field
- WHEN policy freshness validation runs
- THEN the fixture fails for the expected missing-field invariant
