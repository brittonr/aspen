## ADDED Requirements

### Requirement: Octet gate policy is canonical
r[molten.octet_fail_closed_ci.gate_policy] Molten MUST represent strict Octet source-gate policy as canonical `octet-gate-policy-v1` evidence that binds profile, command shape, required artifacts, deny statuses, critical lint classes, quarantine policy, and checks.

#### Scenario: Strict profile policy lists deny statuses
- GIVEN the strict CI Octet profile is evaluated
- WHEN Molten renders the gate policy
- THEN the policy lists `warning-only`, missing, malformed, stale, unsupported, and error outcomes as deny statuses
- AND names the required Octet artifacts and critical lint classes.

### Requirement: Octet gate receipt is canonical
r[molten.octet_fail_closed_ci.gate_receipt] Molten MUST represent Octet source-gate decisions with canonical `octet-gate-receipt-v1` records that bind decision, policy ref, command ref, status ref, summary ref, structured findings ref, object-corpus ref, fingerprint evidence ref, config hash, profile hash, toolchain, finding counts, baseline/review refs, diagnostics, and checks.

#### Scenario: Passing strict gate has complete evidence
- GIVEN an Octet run for a strict CI profile
- AND all required artifacts are present and bound by canonical content refs
- AND the run has no findings
- WHEN Molten emits the Octet gate decision
- THEN the decision is a canonical pass receipt
- AND the receipt references the exact command, config, profile, toolchain, and evidence artifacts used to decide.

### Requirement: Octet gate artifacts are ledger-classified
r[molten.octet_fail_closed_ci.ledger_classification] Molten MUST classify Octet gate policies, receipts, command artifacts, status artifacts, summary artifacts, object-corpus artifacts, structured findings, warning baselines, review manifests, source-gate validation receipts, and fingerprint evidence in the local ledger/catalog.

#### Scenario: Imported Octet artifacts are searchable
- GIVEN Octet gate artifacts are imported into the local ledger
- WHEN the ledger classifies them
- THEN operators can distinguish Octet status, summary, object corpus, fingerprint, gate receipt, baseline, and validation artifacts by kind.

### Requirement: Octet artifacts are bound by canonical refs
r[molten.octet_fail_closed_ci.artifact_ref_binding] Molten MUST bind `command.txt`, `status.json`, `summary.txt`, structured findings, object-corpus receipts, and fingerprint evidence by canonical content refs before accepting an Octet gate result.

#### Scenario: Summary drift changes receipt evidence
- GIVEN a gate receipt binds a summary artifact ref
- WHEN the summary text changes
- THEN the structured findings or summary ref changes
- AND stale receipts cannot silently cover the new summary.

### Requirement: Warning-only status fails strict CI
r[molten.octet_fail_closed_ci.status_semantics] Molten MUST treat `warning-only` as a deny status for strict Octet profiles even when the `cargo-octet` process exit code is `0`.

#### Scenario: Process success alone is not pass evidence
- GIVEN `cargo octet check` exits with code `0`
- AND the Octet status is `warning-only`
- WHEN a strict CI, release, admission, or upgrade gate evaluates the run
- THEN the gate emits a deny receipt rather than pass evidence.

### Requirement: Required Octet artifacts fail closed
r[molten.octet_fail_closed_ci.missing_artifact_denial] Octet gate evaluation MUST deny when required artifacts are missing, malformed, stale, unsupported, or not bound to the expected command, config hash, profile hash, toolchain, object corpus, or source scope.

#### Scenario: Missing status denies with receipt
- GIVEN the Octet artifacts directory lacks `status.json`
- WHEN strict source-gate evaluation runs
- THEN Molten emits a deny receipt with diagnostics
- AND no downstream consumer may claim a source-gate pass.

### Requirement: Critical lint findings deny without review
r[molten.octet_fail_closed_ci.critical_lint_denial] Strict and quarantine Octet profiles MUST deny unreviewed critical findings for panic/abort paths, unwrap/expect, ambient time or entropy in core evidence paths, unbounded loops, critical resource-shape failures, authority typing violations, harness backdoors, secret/capability rendering leaks, and missing adapter boundary evidence.

#### Scenario: Unreviewed critical finding denies quarantine
- GIVEN an Octet warning baseline contains a `no_unwrap` finding on a critical evidence path
- WHEN no review manifest covers the exact finding and profile
- THEN quarantine evaluation denies even though the finding existed before.

### Requirement: Object-corpus and fingerprint evidence is required
r[molten.octet_fail_closed_ci.object_corpus_denial] Strict source-gate pass claims MUST deny when configured critical paths lack object-corpus and fingerprint evidence.

#### Scenario: Missing object corpus denies
- GIVEN `status.json` and `summary.txt` are clean
- WHEN the required object-corpus receipt or object-set fingerprint is missing
- THEN the strict gate denies before any downstream evidence consumer can claim source-gate pass evidence.

### Requirement: CLI exposes Octet gate command
r[molten.octet_fail_closed_ci.cli_gate] Molten MUST expose a local command shape such as `molten test octet gate --artifacts target/octet --profile strict-ci --receipt-out ...` that reads Octet artifacts and writes canonical gate receipts.

#### Scenario: CLI writes deny receipt for warning-only
- GIVEN an Octet artifacts directory with warning-only status
- WHEN the operator runs the Octet gate CLI
- THEN the command writes a canonical deny receipt preserving diagnostics.

### Requirement: Strict CI sequence is documented
r[molten.octet_fail_closed_ci.ci_command_shape] Molten MUST document the strict CI sequence: Octet check, lib-only check where applicable, object corpus receipt, artifact import, Octet gate receipt, remediation plan, harness gates/tests, Clippy, and Cairn strict validation.

#### Scenario: Documented sequence includes source gate receipt
- GIVEN an operator follows the strict Octet source-gate sequence
- WHEN the sequence reaches the source-gate step
- THEN it produces an `octet-gate-receipt-v1` suitable for downstream validation.

### Requirement: Release and admission bind strict Octet receipts
r[molten.octet_fail_closed_ci.release_admission_binding] Release, upgrade, node-runtime startup, remote job admission, and evidence-bearing harness profiles that require source-shape evidence MUST bind passing strict Octet gate receipt refs or source-gate validation refs rather than raw `cargo octet` output.

#### Scenario: Node startup requires source gate receipt
- GIVEN node runtime startup claims source-gated daemon or adapter code
- WHEN startup evidence is evaluated
- THEN it must include passing strict Octet gate validation evidence for the relevant source scope
- AND missing, denied, stale, or tampered gate evidence denies startup before adapters start.

#### Scenario: Remote job admission rejects raw summary only
- GIVEN remote job admission receives only `summary.txt` or process output as source evidence
- WHEN target-side admission evaluates executable readiness
- THEN admission denies because a passing strict source-gate validation receipt is required.

### Requirement: Denied gates preserve diagnostics
r[molten.octet_fail_closed_ci.diagnostic_output] Molten MUST preserve raw Octet status, summary, structured findings, object-corpus, and diagnostics as evidence even when the gate denies, without treating those artifacts as pass receipts.

#### Scenario: Warning artifacts remain diagnostic
- GIVEN strict source-gate evaluation denies a warning-only run
- WHEN the receipt is inspected
- THEN the raw Octet artifacts are still referenced for diagnosis
- AND the decision remains deny.

### Requirement: Warning-only strict test coverage
r[molten.octet_fail_closed_ci.warning_only_test] Molten SHOULD test that `status=warning-only` denies under the strict profile.

#### Scenario: Warning-only fixture denies
- GIVEN a fixture Octet status with warnings and no errors
- WHEN strict gate evaluation runs
- THEN the test asserts the decision is deny.

### Requirement: Missing and stale artifact tests
r[molten.octet_fail_closed_ci.missing_status_test] Molten SHOULD test missing, malformed, stale, unsupported, and mismatched `status.json`, missing object corpus receipts, and mismatched config/profile hash denial.

#### Scenario: Stale metadata fixture denies
- GIVEN a fixture Octet status with stale config or profile hash
- WHEN strict gate evaluation runs
- THEN the test asserts the decision is deny with stale metadata diagnostics.

### Requirement: Critical lint tests
r[molten.octet_fail_closed_ci.critical_lint_test] Molten SHOULD test that unreviewed critical lint findings deny and exact review manifests are required for temporary quarantine acceptance.

#### Scenario: Reviewed critical finding is temporary
- GIVEN a critical finding and a matching unexpired review manifest for quarantine
- WHEN quarantine baseline evaluation runs
- THEN it may pass while strict CI still requires strict evidence.

### Requirement: Receipt binding tests
r[molten.octet_fail_closed_ci.receipt_binding_test] Molten SHOULD test that tampering with command, status, summary, structured findings, object-corpus, or fingerprint refs changes or denies the gate receipt.

#### Scenario: Tampered fingerprint denies downstream validation
- GIVEN a pass-shaped Octet gate receipt whose fingerprint ref is replaced with a malformed ref
- WHEN source-gate validation runs
- THEN validation denies before downstream side effects.
