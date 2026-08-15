## Why

`function_length` and `excessive_file_length` remain disabled broad Octet families. Prior splits moved the CLI shell into focused modules, but source-remediated-zero still requires continued functional-core / imperative-shell decomposition on remaining hotspots.

This change continues the size-shape burn-down in small behavior-preserving slices.

## What Changes

- Use the current no-disabled probe to identify the next size-shape hotspots.
- Split long imperative shells into child modules and extract pure helpers for deterministic logic.
- Preserve public CLI syntax, receipt schemas, canonical Preserves values, and fail-closed behavior.
- Add positive and negative tests when extracted logic changes or becomes newly isolated.

## Impact

This is source-shape cleanup. It should reduce file/function size warnings and make future changes easier to review without altering Molten runtime semantics.
