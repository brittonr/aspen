# Design: change proof checklists

## Scope

This change standardizes review expectations for proof-affecting Cairn changes. It does not block small documentation-only changes when they declare an explicit exemption.

## Checklist fields

The checklist should ask for the proof claim, out-of-scope claims, trusted assumptions, positive evidence, negative evidence, canonical refs, Hegel RS property coverage when core invariants change, traceability updates, and regeneration commands.

## Workflow

The checklist can live in tasks, design, or a dedicated proof section. Validation should be lightweight at first: templates and documented gate expectations. Later changes may add stricter Cairn validation if needed.

## Hegel RS usage

If checklist parsing or completeness validation becomes pure Rust core logic, Hegel RS properties should cover stable parsing, missing-field denial, exemption handling, and deterministic summary rendering.

## Non-goals

- No pull request automation.
- No replacement for canonical receipts or release gates.
