## Why

Several gates inspect rendered Preserves text to detect sensitive markers, ambient job tokens, or retained content refs. Rendered text is useful diagnostics, but it is not the semantic object model. Text scanning can produce false positives from string payloads, miss values encoded through another shape, and couples safety checks to formatting.

## What Changes

- Add a structural Preserves value visitor for record labels, symbols, strings, sequences, sets, dictionaries, and embedded values.
- Replace semantic text scans in service sensitivity checks, job ambient-token denial, and upgrade cleanup ref checks with structural predicates.
- Keep text rendering only for operator diagnostics and tests that explicitly check display output.
- Add positive and negative tests for nested records, literal strings that only look like markers, and refs hidden in supported structures.

## Impact

- **Files**: `preserves_rail` visitor helpers, service records, job DAG validation, upgrade cleanup checks, and tests.
- **Testing**: structural markers deny when present as records or refs; inert strings are not treated as authority-bearing structures unless policy says strings are in scope.
