## Why

`path_segment_repetition` remains an active no-disabled Octet caveat. Prior path-shape cleanup proved that crate-private aliases and clearer module ownership can reduce repetition without changing user-facing CLI behavior.

This change continues path-shape burn-down while preserving public paths and receipt contracts.

## What Changes

- Refresh the no-disabled probe and identify path-repetition hotspots.
- Reduce repeated crate/module segments through local aliases, module ownership changes, or private helper renames.
- Preserve public Rust APIs, CLI syntax, receipt labels, and canonical Preserves values.
- Record before/after path-shape warning movement.

## Impact

This improves source readability and narrows a disabled lint family. It should not change Molten runtime semantics or external compatibility promises.
