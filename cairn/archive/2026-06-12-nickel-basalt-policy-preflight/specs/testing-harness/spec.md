# Testing Harness Delta: Nickel/Basalt policy preflight

### Requirement: Policy gates use Nickel static normalization
r[molten.testing.nickel_basalt_policy_preflight.nickel_normalization] Evidence-bearing policy gates MUST include deterministic Nickel static source and export refs derived from the embedded suite policy. Report validation MUST re-run normalization and reject stale or tampered Nickel source/export evidence.

#### Scenario: Valid Nickel-normalized policy evidence
- GIVEN a suite with an explicit policy fixture or default empty policy
- WHEN the harness runs the suite
- THEN the report policy gate includes Nickel source and export evidence refs bound to the canonical policy ref
- AND report validation recomputes the same Nickel export evidence

#### Scenario: Tampered Nickel export fails validation
- GIVEN a report whose policy gate Nickel export JSON or export ref has been modified
- WHEN report validation runs
- THEN validation fails closed before accepting the report as pass evidence

### Requirement: Policy gates include Basalt preflight receipts
r[molten.testing.nickel_basalt_policy_preflight.basalt_receipt] Evidence-bearing policy gates MUST include a Basalt contract envelope and preflight receipt bound to the Nickel normalized source ref and canonical policy ref.

#### Scenario: Missing Basalt preflight fails validation
- GIVEN a report with policy-gate evidence lacking `<basalt-preflight ...>`
- WHEN report validation runs
- THEN validation fails closed with a Basalt preflight diagnostic

#### Scenario: Tampered Basalt preflight fails validation
- GIVEN a report whose Basalt preflight decision, reason, envelope ref, policy ref, or normalized source ref has been modified
- WHEN report validation runs
- THEN validation rejects the report rather than trusting marker-only policy evidence

### Requirement: Steel predicates remain fail-closed without reviewed receipts
r[molten.testing.nickel_basalt_policy_preflight.steel_fail_closed] Steel or dynamic policy predicates MUST NOT execute or satisfy policy gates unless future reviewed callable receipts are present and validated through Basalt.

#### Scenario: Unreviewed Steel predicate is rejected
- GIVEN a suite policy containing `<steel-predicate ...>` or `<dynamic-predicate ...>`
- WHEN the suite is parsed or preflighted
- THEN it fails closed before runtime turns or side effects

### Requirement: Gate receipts expose policy preflight refs
r[molten.testing.nickel_basalt_policy_preflight.gate_checks] Successful pass-evidence gate receipts MUST include checks and artifact refs for Nickel policy source, Nickel export normalization, Basalt policy gate, Basalt preflight receipt, and Basalt receipt binding.

#### Scenario: Successful gate receipt includes Nickel/Basalt checks
- GIVEN a deterministic report that validates and replays successfully
- WHEN `molten test gate check` emits a receipt
- THEN the receipt includes `nickel-policy-source`, `nickel-export-normalization`, `basalt-preflight-receipt`, and `basalt-receipt-binding` checks
