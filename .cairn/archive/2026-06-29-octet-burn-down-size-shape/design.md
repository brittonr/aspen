## Context

The accepted remediation model requires broad size warnings to remain visible until source splits and code-shape changes remove or narrowly scope them. Size-shape work is safest when it is incremental and behavior-preserving.

## Design

### Hotspot selection

Start from the latest no-disabled probe and choose one cohesive domain hotspot at a time. Prefer CLI shell and orchestration files before touching core logic. If core logic is extracted, move it into a pure helper with explicit input/output types and focused tests.

### Split pattern

Keep parent modules as thin dispatch shells. Move command payloads, IO helpers, receipt rendering, and deterministic core conversions into child modules with clear ownership. Avoid public path churn unless there is a deliberate path-shape change in a separate Cairn package.

### Evidence

Record before/after `function_length` and `excessive_file_length` movement for the touched files. If a size warning remains, document whether it is still active, intentionally deferred, or blocked by a broader public API shape.

## Validation

Run focused tests for the touched domain before and after core changes, then run formatting, Clippy, and a no-disabled Octet probe. Run broader checks only when a split crosses multiple domains.

## Non-goals

- Do not mix broad import-hygiene or path-shape cleanup into the same slice unless required for compilation.
- Do not change canonical receipt values, command syntax, or fail-closed denial behavior.
- Do not mark the size-shape category complete while the no-disabled probe still reports unscopeable Molten-owned size warnings.
