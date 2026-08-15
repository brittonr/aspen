# Design: Production profile enum contracts

## Context

Production readiness receipts depend on string markers being stable, reviewed evidence categories. Free-form arrays make marker spelling part of operator memory rather than part of the contract.

## Vocabulary contracts

Define an `AllowedValue` helper over reviewed string arrays, then derive contracts for:

- `AdapterName`
- `RedactionSetting`
- `LiveTransportSetting`
- `StartupExpectation`
- `ShutdownExpectation`

Each contract accepts only exact strings in its reviewed vocabulary and returns a diagnostic naming the invalid entry. The helper is immediate and idempotent. It does not infer aliases or normalize spelling.

## Review workflow

Vocabulary growth is a contract change. Adding a new adapter, setting, or expectation requires editing the allowed-value list, updating operator documentation, and adding export coverage for the new accepted value plus at least one rejected typo.

## Boundaries

Enum contracts validate marker identity only. They do not prove that an adapter implementation exists, starts, or satisfies conformance. Runtime startup receipts and adapter preflight evidence remain responsible for live behavior.
