# Evidence Gates Delta: Executor Resource Gate Receipts

### Requirement: Gate receipts bind executor execution receipts
r[molten.evidence.executor_resource_gate_receipts.aggregate_ref] Gate receipts MUST include a canonical aggregate ref for all executor execution receipts embedded in the validated report.

#### Scenario: Execution receipt aggregate is named
- GIVEN a report with Steel or Wasm execution receipt events
- WHEN a gate receipt is emitted
- THEN its artifact refs include `executor-execution-receipts`
- AND the ref is derived from the canonical sequence of execution receipt events

### Requirement: Gate receipts expose executor resource checks
r[molten.evidence.executor_resource_gate_receipts.checks] Gate receipts MUST explicitly include checks for executor resource and ABI bounds that are required for pass evidence.

#### Scenario: Steel resource bounds are visible at the gate
- GIVEN a report containing reviewed Steel execution receipts
- WHEN a gate receipt is parsed
- THEN the receipt includes `steel-resource-bounds`

#### Scenario: Wasm ABI bounds are visible at the gate
- GIVEN a report containing reviewed Wasm ABI execution receipts
- WHEN a gate receipt is parsed
- THEN the receipt includes `wasm-abi-byte-bounds` and `wasm-guest-memory-bounds`

### Requirement: Missing executor resource checks fail closed
r[molten.evidence.executor_resource_gate_receipts.validation] Gate receipt parsing MUST reject receipts missing the execution receipt aggregate ref or required executor resource checks.

#### Scenario: Tampered gate receipt drops resource check
- GIVEN a valid gate receipt
- WHEN `steel-resource-bounds` or `wasm-abi-byte-bounds` is removed
- THEN receipt parsing fails closed
