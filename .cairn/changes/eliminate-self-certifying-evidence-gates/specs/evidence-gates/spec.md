# Evidence gates deltas

## ADDED Requirements

### Requirement: Release promotion requires validated canonical Octet and Cairn receipts
r[molten.evidence_gates.release_promote_requires_canonical_receipts] `dogfood release-promote` MUST reject raw descriptive or placeholder evidence strings and MUST require strictly decoded, passing canonical Octet gate and strict Cairn validation receipts bound to the candidate subject before emitting a passing promotion receipt.

#### Scenario: Placeholder strings deny release promotion
- GIVEN `--octet-evidence "octet:external-clean-gate-required"` or `--cairn-evidence "cairn:external-strict-validate-required"`
- WHEN `release-promote` evaluates the inputs
- THEN it MUST fail closed with a typed receipt-validation denial and MUST NOT emit `decision=pass`.

#### Scenario: Valid canonical receipts bind exact candidate
- GIVEN canonical passing Octet and strict Cairn receipts matching the exact candidate ref
- WHEN `release-promote` evaluates the inputs
- THEN it MUST bind both canonical receipt refs into the promotion receipt.

### Requirement: VM shard receipts cannot mint passing decisions from caller-supplied flags
r[molten.evidence_gates.vm_shard_derived_decision_only] `molten test nixos-vm shard-run` MUST derive shard pass/fail decisions from verified VM execution observations and MUST NOT accept a caller-supplied `--claimed-decision` flag to mint a canonical `decision "pass"` receipt over synthetic fixture strings.

#### Scenario: Synthetic text cannot mint a passing VM shard receipt
- GIVEN a caller supplies raw fixture text without observed VM execution artifacts
- WHEN `shard-run` runs
- THEN it MUST reject the request or emit a non-passing/inconclusive diagnostic rather than `decision "pass"`.

#### Scenario: Real VM observations produce derived shard receipt
- GIVEN observed VM execution artifacts with verified canonical outcomes
- WHEN `shard-run` validates them
- THEN the emitted shard receipt MUST reflect the derived outcome without trusting caller-claimed status flags.

### Requirement: Flake checks validate canonical receipts and real repository artifacts
r[molten.evidence_gates.flake_checks_canonical_non_circular] Flake checks for traceability, drift, dogfood, and VM behavior MUST inspect canonical Preserves receipts rather than grepping rendered stdout/logs, MUST NOT pass by comparing a file to its own copy, and MUST accurately name unsigned-denial versus live-execution VM checks.

#### Scenario: Self-copy drift check is forbidden
- GIVEN a drift check verifying deterministic reproduction
- WHEN two outputs are compared
- THEN the outputs MUST come from independent executions rather than copying one output file to the other.

#### Scenario: Rendered log text cannot pass a flake check
- GIVEN a command emits `decision=pass` to stdout while its canonical receipt records denial
- WHEN the flake check validates the run
- THEN the check MUST fail closed based on the canonical receipt.
