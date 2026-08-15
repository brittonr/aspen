# Node Runtime Delta: Production profile named units

### Requirement: Production resource thresholds are named
r[molten.prod_ops.profile_named_units.named_thresholds] Production deployment profile resource limits MUST be expressed in Nickel through named unit and threshold constants rather than unexplained numeric literals in the concrete profile body.

#### Scenario: Named thresholds define profile limits
- GIVEN a reviewer inspects the production profile source
- WHEN they read queue, receipt, store, delivery-latency, and recovery-time limits
- THEN each limit is derived from a named Nickel constant that states the unit and reviewed threshold meaning

#### Scenario: Threshold change is review-visible
- GIVEN a production resource threshold changes
- WHEN the profile diff is reviewed
- THEN the diff names the threshold being changed rather than exposing only an unexplained numeric literal

### Requirement: Named units preserve exported profile values
r[molten.prod_ops.profile_named_units.export_stability] Replacing production profile numeric literals with named Nickel constants MUST preserve the reviewed exported JSON values unless the same change explicitly updates the threshold.

#### Scenario: Current profile export remains stable
- GIVEN the current production profile is rewritten to use named unit and threshold constants
- WHEN the operator exports the profile through Nickel
- THEN the exported resource-limit values match the previous reviewed profile export

#### Scenario: Unintended numeric drift is caught
- GIVEN a named unit or threshold edit changes an exported resource-limit value without an explicit threshold-review update
- WHEN profile fixture validation runs
- THEN validation fails and reports export drift before production readiness receipts are updated
