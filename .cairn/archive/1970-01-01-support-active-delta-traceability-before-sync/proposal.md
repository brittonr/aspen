## Why

The inherited-debt guard currently treats source references to requirements in active Cairn deltas as dangling because it reads accepted `.cairn/specs` only. That makes the required pre-sync source gate circular for implementation work: the package cannot sync before validation, and validation cannot recognize its proposed identities before sync. The current Molten consumer has 22 such references; suppressing them or syncing incomplete packages would misrepresent readiness.

The user explicitly approved a scoped workflow change: recognize validated active-delta identities for reference resolution while retaining accepted-only coverage and exact debt enforcement. This change does not authorize an exception for uncovered accepted requirements.

## What Changes

- Keep accepted requirements as the sole denominator for inherited coverage, missing requirements, and baseline comparison.
- Recognize identifiers declared in native active change delta specifications as pending identities for dangling-reference resolution only; exclude archive, proposal, design, task, and arbitrary documentation files.
- Report pending references separately from accepted coverage. Unknown references still fail, including baseline-write mode.
- Keep required Cairn structural validation and all-task/passing-gate sync/archive requirements unchanged.
- Add positive and negative behavior coverage and exercise the actual standalone guard on owned fixture trees and the current repository.

## Impact

- **Consumer and outcome**: Molten implementation packages can validate source references before sync without accepting their unfinished requirements.
- **Files**: `tools/tracey/inherited_debt_guard.rs`, existing traceability documentation, this change's project delta, and its append-only verification evidence. The reviewed inherited baseline is unchanged.
- **Testing**: standalone guard behavior tests; real CLI fixtures for accepted, pending, unknown, removed, and non-delta identities; inherited Tracey Nix check; formatting; strict native Cairn validation and proposal/design/tasks gates.
- **Owner and repeatability**: Molten lifecycle/traceability maintainers own the distinction. Exact fixture commands and repository results belong in `evidence/verification.md` before readiness is claimed.
