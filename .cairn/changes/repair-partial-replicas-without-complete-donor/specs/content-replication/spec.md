# Content replication: closure repair delta

## ADDED Requirements

### Requirement: Donor eligibility is per object
r[molten.closure_repair.donor_eligibility] Molten repair planning MUST evaluate donor eligibility per required object from admitted verification facts, and MUST NOT reject a donor for the objects it holds intact because the same replica is incomplete for others.

#### Scenario: Partial donors assemble the closure
- GIVEN several reachable replicas whose intact objects overlap and whose union covers the required closure with no complete single donor
- WHEN repair planning runs under stated recovery and resource conditions
- THEN the plan selects per-object donors from the partial replicas and repairs the full closure.

#### Scenario: Individually incomplete replicas are not rejected wholesale
- GIVEN every replica is individually incomplete
- WHEN the planner evaluates donors
- THEN no replica is excluded solely because it cannot donate every object.

### Requirement: Assembled bytes verify against established identities
r[molten.closure_repair.per_object] Molten MUST verify each repaired object against its already-established content identity before admission into the repaired set, and a verification failure MUST isolate to that object.

#### Scenario: Repaired object matches its identity
- GIVEN a donor supplies bytes for a required object
- WHEN verification runs on assembly
- THEN only bytes matching the established identity enter the repaired set.

#### Scenario: One failure does not poison others
- GIVEN a donor verification failure on one object while other objects verify
- WHEN the repair completes
- THEN the verified objects are retained and only the failed object is unresolved.

### Requirement: Incomplete repairs are explicit and bounded
r[molten.closure_repair.incomplete_outcome] Molten MUST report a typed incomplete-repair outcome naming the unrecoverable object identities when any required object has no intact verifiable copy, while preserving progress on repaired objects.

#### Scenario: Zero intact copies is named
- GIVEN a required object with no intact verifiable copy in any reachable replica
- WHEN the repair outcome is produced
- THEN the outcome is incomplete, names that identity, and does not claim closure repair.

#### Scenario: Progress is retained
- GIVEN an incomplete repair that repaired some objects
- WHEN the outcome is evaluated
- THEN the repaired objects keep their repaired status.

### Requirement: Repair stays inside the content boundary
r[molten.closure_repair.boundary] Molten MUST keep repair bounded to recovering bytes matching established identities and MUST NOT treat repaired content as current placement or as a determination of the authoritative world head.

#### Scenario: Repair does not confer placement
- GIVEN a stale replica supplies verified bytes
- WHEN the repair completes
- THEN the stale replica does not count as current placement.

#### Scenario: Repair does not confer authority
- GIVEN a completed closure repair
- WHEN any consumer asks which world head is authoritative
- THEN repair evidence alone does not answer that question.

### Requirement: Validation limits closure-repair evidence
r[molten.closure_repair.validation] Molten MUST retain positive and negative repository tests for closure repair and MUST limit evidence to simulated content verification under stated conditions.

#### Scenario: Closure scenario runs in normal tests
- GIVEN the multi-partial-donor fixture with bounded inventories
- WHEN normal repository tests execute it
- THEN the closure is repaired with per-object verification and recorded outcomes.

#### Scenario: No distributed or authority claim follows
- GIVEN the executed closure fixtures
- WHEN results are reported
- THEN they claim no live-transport equivalence, no authority determination, and no permanent availability.
