## Why

The no-disabled Octet probe still reports many `path_segment_repetition` warnings. Those warnings represent naming and module-shape debt that is distinct from import hygiene, file/function length, source-scope false positives, and smaller correctness-oriented warnings.

This change gives path-shape work its own Cairn package so repeated-path remediation can proceed in focused, reviewable slices.

## What Changes

- Track `path_segment_repetition` as a dedicated active Cairn change.
- Rename or regroup module-local helpers, intermediate structs, and child modules where doing so reduces repeated path segments without public API or CLI drift.
- Preserve canonical receipt values and evidence-only trust boundaries.
- Keep the disabled-lint caveat visible until refreshed no-disabled evidence proves this family is clean or safely narrowed.

## Impact

This is mechanical source-shape work. It should lower repeated path warnings while preserving behavior and keeping public command and receipt contracts stable.
