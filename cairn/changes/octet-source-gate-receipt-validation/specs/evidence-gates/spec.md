## ADDED Requirements

### Requirement: Downstream consumers validate Octet gate receipt content
r[molten.octet_source_gate_receipt_validation.spec.content_validation] Downstream evidence consumers MUST validate the actual canonical `octet-gate-receipt-v1` value before treating an Octet source-gate ref as pass evidence.

#### Scenario: Raw summary is not pass evidence
- GIVEN a downstream node startup, remote job admission, or upgrade plan with only `summary.txt`, `status.json`, or `cargo octet` process output
- WHEN source-gate validation runs
- THEN validation denies
- AND the consumer cannot claim `strict-octet-source-gate-bound`

#### Scenario: Denied gate receipt is rejected
- GIVEN an `octet-gate-receipt-v1` with decision `deny`
- WHEN a downstream consumer validates it as strict source-gate evidence
- THEN validation emits a canonical deny receipt
- AND no downstream side effect is admitted

### Requirement: Strict pass receipts must be current and scoped
r[molten.octet_source_gate_receipt_validation.spec.current_strict_scope] Source-gate validation MUST require decision `pass`, profile `strict-ci`, current Octet config/profile/toolchain refs, and source-scope object-corpus/fingerprint coverage for the downstream consumer.

#### Scenario: Stale config hash denies
- GIVEN a previously passing Octet gate receipt
- AND the current `[workspace.metadata.octet]`, command scope, pass-through args, `Cargo.toml`, or `dylint.toml`-derived profile evidence has changed
- WHEN node startup, remote job admission, or upgrade planning validates the receipt
- THEN validation denies as stale

#### Scenario: Quarantine profile is not strict source evidence
- GIVEN a quarantine-profile Octet receipt that covers existing warning debt
- WHEN a strict downstream consumer validates it for production startup, remote admission, or upgrade planning
- THEN validation denies because quarantine evidence is not a strict source-gate pass

#### Scenario: Missing fingerprint coverage denies
- GIVEN a pass-shaped Octet gate receipt without object-corpus or fingerprint evidence for the required consumer source scope
- WHEN source-gate validation runs
- THEN validation denies before downstream side effects

### Requirement: Consumers bind validation receipts before side effects
r[molten.octet_source_gate_receipt_validation.spec.consumer_binding] Node startup, remote job admission, and upgrade planning MUST bind `octet-source-gate-validation-v1` pass receipt refs in their own receipts before performing side effects.

#### Scenario: Node startup denies before adapters start
- GIVEN a node config that references a missing, denied, stale, or tampered Octet gate receipt
- WHEN startup validation runs
- THEN `node-startup-receipt-v1` denies
- AND production adapters are not started

#### Scenario: Remote job admission denies before executable readiness
- GIVEN a remote job admission request with invalid source-gate evidence
- WHEN target-side admission evaluates executable readiness
- THEN the job admission receipt denies
- AND it does not claim executable-artifact readiness or allow execution

#### Scenario: Upgrade planning denies before irreversible work
- GIVEN an upgrade plan that would move names, run storage migrations, or schedule irreversible tasks
- AND strict Octet source-gate validation fails
- WHEN the upgrade plan is evaluated
- THEN the plan denies before name moves, migrations, or transcript-gated work

### Requirement: Tampered source-gate evidence fails closed
r[molten.octet_source_gate_receipt_validation.spec.tamper_denial] Source-gate validation MUST deny when receipt diagnostics/checks claim pass but bound refs, counts, structured findings, object-corpus evidence, or fingerprint evidence are missing, malformed, or inconsistent.

#### Scenario: Object corpus ref tampering denies
- GIVEN an Octet gate receipt whose object-corpus ref has been replaced after the gate was generated
- WHEN downstream validation recomputes and checks the receipt evidence
- THEN validation denies and reports the mismatched object-corpus evidence

#### Scenario: Critical finding count tampering denies
- GIVEN an Octet gate receipt whose decision/checks claim pass
- AND structured findings still contain uncovered critical findings or inconsistent counts
- WHEN downstream validation runs
- THEN validation denies and records deterministic diagnostics
