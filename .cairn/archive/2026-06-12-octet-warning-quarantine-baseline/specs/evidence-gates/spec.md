## ADDED Requirements

### Requirement: Octet warning baselines are explicit quarantine evidence
r[molten.octet_warning_quarantine.spec.baseline_artifact] Temporary Octet warning baselines MUST be canonical `octet-warning-baseline-v1` artifacts that bind the source scope, Octet config hash, profile hash, toolchain, source snapshot ref, stable finding keys, expiry, allowed profiles, burn-down targets, review refs, and checks.

#### Scenario: Baseline is visible evidence
- GIVEN CI is running in a quarantine profile
- WHEN the Octet gate evaluates existing warnings
- THEN the gate references an `octet-warning-baseline-v1` artifact
- AND downstream receipts identify the decision as quarantine-covered debt rather than strict source-gate pass evidence

#### Scenario: Hidden suppression file is rejected
- GIVEN Octet warnings are suppressed by local comments or hidden config with no canonical baseline artifact
- WHEN the quarantine profile evaluates the run
- THEN the gate denies because the warning debt is not auditable evidence

### Requirement: Quarantine comparison denies regressions
r[molten.octet_warning_quarantine.spec.no_new_findings] Quarantine profiles MUST deny when a current Octet run contains new, moved, unkeyed, escalated, malformed, or unsupported findings relative to the baseline and attached review receipts.

#### Scenario: One new warning fails quarantine CI
- GIVEN an Octet baseline covering all existing findings
- AND a new run with one additional finding key
- WHEN the quarantine profile evaluates the run
- THEN it emits an `octet-baseline-receipt-v1` deny receipt
- AND the deny diagnostics identify the new finding key

#### Scenario: Removed warning is accepted and counted
- GIVEN an Octet baseline covering an old finding
- AND a new run where that finding is absent
- WHEN the quarantine profile evaluates the run
- THEN the baseline receipt records the finding as removed
- AND the burn-down count decreases

### Requirement: Critical findings cannot be silently baselined
r[molten.octet_warning_quarantine.spec.critical_review] Baselines MUST NOT silently cover critical findings such as panic/unwrap, ambient time or entropy in core paths, unbounded loops, critical resource-shape failures, authority typing violations, secret rendering, harness backdoors, or missing adapter evidence; each retained critical finding MUST have a review receipt bound to the exact finding key, source fingerprint, risk rationale, and replacement plan.

#### Scenario: Baseline contains unreviewed critical finding
- GIVEN a baseline includes a `no_unwrap` finding on a critical evidence path
- AND no review receipt covers that exact finding and profile
- WHEN quarantine CI evaluates the baseline
- THEN the gate denies even though the finding existed before

#### Scenario: Reviewed critical finding is temporary
- GIVEN a critical finding has an authenticated review receipt with expiry and mitigation plan
- WHEN quarantine CI evaluates it before expiry
- THEN the gate may count it as reviewed debt
- BUT strict CI still denies unless the strict profile explicitly accepts that review receipt

### Requirement: Baselines expire and shrink
r[molten.octet_warning_quarantine.spec.expiry_and_shrink] Octet warning baselines MUST expire, and every refresh MUST reduce uncovered finding count or bind review receipts explaining deferred findings and a new burn-down target.

#### Scenario: Expired baseline denies
- GIVEN a baseline whose `expires-at` is before the current gate evaluation time or logical release milestone
- WHEN the quarantine profile evaluates an otherwise matching run
- THEN the gate denies and requires a refreshed baseline or strict warning-free run

#### Scenario: Refresh without shrink requires review
- GIVEN a baseline refresh with the same or higher warning count
- WHEN no review receipts justify deferred findings
- THEN the gate denies the refresh because warning debt did not shrink

### Requirement: Quarantine receipts do not replace strict pass receipts
r[molten.octet_warning_quarantine.spec.strict_separation] Quarantine pass receipts MUST NOT be accepted as strict release, upgrade, node startup, or remote admission source-gate pass evidence after the configured transition deadline.

#### Scenario: Release rejects quarantine receipt
- GIVEN a release evidence bundle contains an `octet-gate-receipt-v1` that passed only through `quarantine-ci`
- WHEN the release gate requires strict source evidence
- THEN the release gate denies until a strict Octet gate pass receipt is provided
