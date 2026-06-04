## ADDED Requirements

### Requirement: Octet gate emits canonical fail-closed receipts
r[molten.octet_fail_closed_ci.spec.gate_receipts] Octet source-gate decisions MUST be represented by canonical `octet-gate-receipt-v1` records that bind the gate policy, Octet command, toolchain, config hash, profile hash, status artifact, summary artifact, structured findings, object corpus evidence, fingerprint evidence, baseline or review refs, finding counts, diagnostics, and checks.

#### Scenario: Passing strict gate has complete evidence
- GIVEN an Octet run for a strict CI profile
- AND all required artifacts are present and bound by canonical content refs
- AND there are no findings or all required reviewed exceptions are valid for that strict profile
- WHEN Molten emits the Octet gate decision
- THEN the decision is a canonical `octet-gate-receipt-v1` pass receipt
- AND the receipt references the exact command/config/profile/toolchain and evidence artifacts used to decide

#### Scenario: Process success alone is not pass evidence
- GIVEN `cargo octet check` exits with code `0`
- AND the Octet status is `warning-only`
- WHEN a strict CI, release, admission, or upgrade gate evaluates the run
- THEN the gate emits a deny receipt unless the profile explicitly admits an unexpired reviewed quarantine receipt covering every finding

### Requirement: Strict profiles reject warning-only source evidence
r[molten.octet_fail_closed_ci.spec.warning_only_denies] Strict Octet profiles MUST treat `warning-only`, missing, malformed, stale, unsupported, and error statuses as deny outcomes rather than pass outcomes.

#### Scenario: Warning-only run denies strict CI
- GIVEN an Octet `status.json` with `status = "warning-only"`
- WHEN the `strict-ci` profile evaluates it
- THEN the gate denies
- AND preserves the warning artifacts as diagnostics
- AND no downstream harness, release, admission, upgrade, or node-runtime startup receipt may claim Octet source-gate pass evidence from that run

#### Scenario: Clean run passes strict CI
- GIVEN an Octet status with zero findings
- AND required object corpus and fingerprint artifacts for configured critical paths
- WHEN the `strict-ci` profile evaluates it
- THEN the gate may pass and emit a canonical receipt

### Requirement: Required Octet artifacts fail closed
r[molten.octet_fail_closed_ci.spec.required_artifacts] Octet gate evaluation MUST deny when required artifacts are missing, malformed, stale, unsupported, or not linked to the expected command, workspace metadata, config hash, profile hash, toolchain, or source scope.

#### Scenario: Missing object corpus receipt denies
- GIVEN a strict profile requiring object corpus evidence for critical paths
- WHEN `status.json` and `summary.txt` exist but the object corpus receipt is absent
- THEN the Octet gate denies before downstream evidence consumers can claim source-gate pass evidence

#### Scenario: Stale config hash denies
- GIVEN a previously passing Octet status artifact
- AND `[workspace.metadata.octet]` or the effective cargo check arguments changed
- WHEN the gate compares the current config/profile hash to the artifact metadata
- THEN the gate denies as stale

### Requirement: Critical lint classes deny immediately
r[molten.octet_fail_closed_ci.spec.critical_lints] Strict and quarantine Octet profiles MUST deny unreviewed critical findings for panic/abort paths, unwrap/expect, ambient time or entropy in core evidence paths, unbounded loops, unbounded resource growth on critical surfaces, authority typing violations, harness backdoors, secret/capability rendering leaks, and missing adapter boundary evidence.

#### Scenario: Ambient clock finding in critical surface denies
- GIVEN an Octet finding classified as `ambient_clock` on a marked core, replay, report validation, admission, or source-gate path
- WHEN any CI profile evaluates the run
- THEN the gate denies unless a review receipt reclassifies the surface and binds the exact finding key, source fingerprint, and policy rationale

#### Scenario: Noncritical quarantined style finding can be temporarily covered
- GIVEN a noncritical finding covered by an unexpired quarantine receipt
- WHEN the quarantine profile evaluates the run
- THEN the finding may be counted as covered debt
- BUT the strict profile still denies until the finding is removed or reviewed for that strict profile

### Requirement: Downstream evidence must reference Octet gate receipts
r[molten.octet_fail_closed_ci.spec.downstream_binding] Release, upgrade, remote job admission, node runtime startup, and evidence-bearing harness profiles that require source-shape evidence MUST reference passing Octet gate receipt refs rather than raw `cargo octet` process output.

#### Scenario: Node startup requires source gate receipt
- GIVEN a node runtime startup receipt claims source-gated adapter or daemon code
- WHEN the startup evidence bundle is validated
- THEN it must include a passing Octet gate receipt ref for the relevant source scope
- AND missing or denying Octet receipts fail startup admission closed

#### Scenario: Release evidence rejects raw summary only
- GIVEN a release bundle contains `summary.txt` but no `octet-gate-receipt-v1`
- WHEN the release evidence gate evaluates it
- THEN the release gate denies because raw summaries are diagnostics, not pass receipts
