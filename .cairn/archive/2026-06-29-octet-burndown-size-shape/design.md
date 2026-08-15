## Context

`excessive_file_length` and `function_length` warnings identify files and functions that are too large for easy review. Unlike import and path-name cleanup, these slices may require extracting deterministic helper cores from imperative shells.

## Design

### Size-shape boundary

This change owns source edits whose primary evidence is lower `excessive_file_length` or `function_length` counts. Acceptable slices include:

- moving cohesive private helpers into child modules;
- extracting pure helper cores from long command or receipt assembly functions;
- splitting long tests into focused modules while preserving coverage;
- replacing long match arms with explicit per-command helper functions.

Every non-trivial extraction should keep logic testable without standing up the world. I/O, file reads, environment, process execution, and receipt emission remain in thin shells.

### Validation

For each slice, run focused tests for the touched domain, formatting, and a no-disabled Octet probe. When core logic changes, add or preserve both positive and negative tests.

### Non-goals

- Do not move code only to game line counts while making ownership less clear.
- Do not combine many domain splits in one unreviewable slice.
- Do not claim disabled-family removal until refreshed evidence supports it.
