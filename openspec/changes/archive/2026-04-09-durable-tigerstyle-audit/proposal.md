## Why

The Tiger Style audit from this session produced useful findings, but it did not leave durable repo evidence. The review was correct on three points:

- the audit summary lived in chat and an off-repo note instead of a tracked artifact
- a new napkin rule referenced an audit-parser failure without a committed reproducer
- no source refactors landed, so the work could not be treated as Tiger Style remediation

Aspen already has an archived `tiger-style-remediation` change for the unwrap/panic purge. This change is different. It defines how Tiger Style audits become reviewable repo artifacts and turns the current hotspot list into an implementation backlog with an initial refactor slice.

## What Changes

- **Durable audit artifact**: Add a committed Tiger Style audit report to the change directory with scope, method, ranked hotspots, and a clear audit-only vs remediation status.
- **Reproducible scanner behavior**: Add a repo-local audit harness plus a regression fixture for the trait-declaration false-positive that was discovered during scanning.
- **Executable remediation backlog**: Convert the current hotspot list into phased tasks with exact files, functions, and verification commands.
- **First bounded remediation slice**: Refactor a small set of production hotspots so the change contains real source improvements rather than audit notes alone.

## Capabilities

### New Capabilities

- `tigerstyle-audit-report`: Contributors can review a committed Tiger Style audit report without relying on chat history or home-directory files.
- `tigerstyle-audit-reproducer`: Contributors can rerun the scanner and verify that declaration-only trait methods are not counted as function bodies.
- `tigerstyle-remediation-backlog`: Contributors have a bounded, file-specific refactor plan tied to the audit findings.

### Modified Capabilities

- `engineering-audit-discipline`: Audit-only work becomes explicitly labeled and cannot be mistaken for completed remediation.

## Impact

- **Files**: `openspec/changes/durable-tigerstyle-audit/`, a repo-local Tiger Style audit script or helper, scanner fixtures/tests, and selected production source files in the first refactor slice.
- **APIs**: Internal function boundaries may change in the targeted hotspots as long functions are split and pure helpers are extracted.
- **Dependencies**: No new runtime dependency is required. The audit harness should use existing repo tooling where possible.
- **Testing**: The scanner gets a regression fixture. Refactored hotspots get characterization tests or targeted unit/integration coverage before behavior-preserving extraction.
