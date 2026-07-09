# Artifact Registry Delta: Name View Pointers

## ADDED Requirements

### Requirement: Name views are canonical metadata records
r[molten.artifacts.name_view_records] Molten MUST model names, aliases, tags, and channels as canonical metadata view records that point to immutable artifact refs or artifact-set refs and bind scope, issuer, policy refs, evidence refs, previous-view refs, and tombstones.

#### Scenario: Name update preserves artifact identity
- GIVEN a name view points from `policy/main` to artifact ref A
- WHEN an authorized update points `policy/main` to artifact ref B
- THEN Molten emits a new view receipt
- AND artifact refs A and B remain immutable and addressable.

#### Scenario: Unauthorized view update denies
- GIVEN a caller lacks the capability or policy evidence required to update a name view
- WHEN it submits a pointer update
- THEN Molten denies the update before mutating metadata
- AND records that no artifact identity changed.

### Requirement: Normative uses pin exact refs
r[molten.artifacts.exact_ref_pinning] Molten MUST require dependencies, transcript expectations, remote execution requests, migration recipes, storage type bindings, and policy admissions to record exact artifact refs after any name resolution.

#### Scenario: Transcript records resolved artifact ref
- GIVEN a transcript stanza refers to a human-readable artifact name
- WHEN Molten admits the transcript for replayable execution
- THEN the transcript or admission receipt records the exact resolved artifact ref
- AND future replay does not depend on mutable name lookup.

#### Scenario: Name-only execution request denies
- GIVEN a remote execution request names an entrypoint by mutable name only
- WHEN Molten evaluates execution admission
- THEN it denies until the request carries an exact artifact ref or admitted resolution receipt.

### Requirement: Ambiguous name resolution fails closed
r[molten.artifacts.name_ambiguity_denial] Molten MUST deny normative name resolution when multiple candidates match and no admitted scope or channel policy selects exactly one target.

#### Scenario: Scoped resolution selects one target
- GIVEN two artifacts share a display name in different scopes
- WHEN a request includes an admitted scope that selects one candidate
- THEN Molten resolves to that exact artifact ref
- AND records the scope decision in diagnostics.

#### Scenario: Ambiguous display name denies
- GIVEN a display name matches multiple candidate artifact refs and no scope policy disambiguates them
- WHEN the name is used for install, execution, migration, storage, policy, or release admission
- THEN Molten denies before side effects
- AND diagnostics list the candidate refs.

### Requirement: Name views are non-authority
r[molten.artifacts.name_views_non_authority] Molten MUST treat name views, aliases, tags, and channels as discovery metadata only; they MUST NOT grant capability, provenance, policy trust, source-gate trust, retention rights, transport trust, or execution authority.

#### Scenario: Name assists discovery only
- GIVEN a catalog query finds an artifact by name
- WHEN the operator requests details
- THEN Molten may render the name and exact ref together
- AND any subsequent use still requires normal admission gates.

#### Scenario: Trusted-looking name does not bypass gates
- GIVEN an artifact is named `trusted/release`
- WHEN a caller attempts execution without required provenance or policy evidence
- THEN Molten denies execution
- AND reports that the name has no trust authority.

### Requirement: Name view validation covers positive and negative paths
r[molten.artifacts.name_view_validation] Molten MUST include positive and negative fixtures for deterministic name resolution, exact ref pinning, ambiguous names, stale channels, unauthorized pointer updates, and name-only execution denial.

#### Scenario: Valid pointer fixture passes
- GIVEN an authorized name view update and exact target artifact ref
- WHEN validation runs
- THEN the view receipt verifies and the target ref is unchanged.

#### Scenario: Stale channel fixture denies
- GIVEN a release channel view has a freshness or revocation policy that is no longer satisfied
- WHEN validation runs for normative use
- THEN Molten denies resolution
- AND emits stale-view diagnostics.