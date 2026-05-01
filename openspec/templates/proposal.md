# Proposal Template

## Why

Describe the problem, why it matters now, and the user-visible outcome.

## What Changes

List the behavior, API, workflow, or policy changes. Keep this concise but specific enough to scope design and tasks.

## Capabilities

### New Capabilities
- `domain.requirement-id`: short description.

### Modified Capabilities
- `domain.requirement-id`: short description.

## Verification Expectations

Required for changes that add, modify, or remove delta specs under `specs/`.
Map each changed requirement or scenario ID to expected verification before design/tasks are approved.

- `domain.requirement-id`: positive verification expectation, such as fixture, checker, unit test, integration test, or manual gate.
- `domain.requirement-id.failure-scenario`: negative-path verification expectation when the proposal, impact, or delta specs mention rejection, failure, malformed input, unauthorized access, timeout, invalid state, missing input, or error behavior.
- `domain.requirement-id`: defer to design because `<rationale>` when the proposal can name the required behavior but not the exact rail yet.

`defer to design` entries must cite the requirement/scenario ID and a rationale. A bare `defer to design` is not enough.

## Impact

- **Files**: files, crates, docs, templates, or scripts expected to change.
- **APIs**: API surface changes, if any.
- **Dependencies**: new, removed, or feature-gated dependencies, if any.
- **Testing**: summary of the verification expectations above.
