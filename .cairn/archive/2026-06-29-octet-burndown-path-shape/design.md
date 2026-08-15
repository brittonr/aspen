## Context

`path_segment_repetition` remains a large no-disabled warning family. It often points at helper names or nested module paths that repeat domain terms, so it should be handled separately from imports and file/function length.

## Design

### Path-shape boundary

This change owns source edits whose primary evidence is a lower `path_segment_repetition` count. Acceptable slices include:

- shortening private helper names without changing public APIs;
- introducing module-local aliases when they reduce repetition without hiding control flow;
- regrouping private child modules when the regrouping keeps module ownership clearer;
- renaming implementation-only structs or functions to avoid repeated path terms.

Public Rust paths, CLI syntax, receipt schemas, and evidence refs must remain stable unless a future change explicitly proposes a public compatibility migration.

### Validation

Each accepted slice should run focused Rust validation and a no-disabled Octet probe. Evidence must record the before/after `path_segment_repetition` count and any offsetting changes in import or size warnings.

### Non-goals

- Do not rename public receipt kinds or Preserves markers.
- Do not hide repeated paths by suppressing lints.
- Do not mix unrelated safety or source-scope work into path-shape slices.
