## Why

The no-disabled Octet probe still reports `excessive_file_length` and `function_length` warnings. These hotspots need decomposition, but they should not be mixed with import or path-name cleanup because they often require moving helper cores and shell orchestration boundaries.

This change isolates size-shape burn-down so long files and long functions can be split with functional-core / imperative-shell discipline.

## What Changes

- Track `excessive_file_length` and `function_length` as a dedicated active Cairn change.
- Split long modules into focused child modules where public Rust paths, CLI syntax, and receipt behavior remain stable.
- Extract deterministic helper cores and keep imperative shells thin when long functions contain logic rather than only command wiring.
- Refresh focused validation and no-disabled Octet evidence after each accepted slice.

## Impact

This is behavior-preserving decomposition work. It should reduce file/function size warnings while improving testability and keeping release evidence contracts unchanged.
