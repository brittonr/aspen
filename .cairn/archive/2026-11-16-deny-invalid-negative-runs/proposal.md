## Why

Issue #43: an invalid negative verification run could become qualifying negative coverage. A receipt uses `deny` both for an expected rejection and for an invalid run; conversion copied only the decision, fell back to the receipt's own hash when no artifact was produced, and treated an empty artifact list as present. With valid positive coverage, the strict manifest could report `covered` and `pass`.

## What Changes

- The receipt parser recomputes run diagnostics from coverage kind, exit status and produced artifacts and rejects a receipt whose diagnostics differ.
- Conversion keeps the receipt's run diagnostics on the evidence, never substitutes the receipt hash for an artifact, and treats an empty artifact list as absent.
- Coverage admission reports every run diagnostic as `stale-<kind>-invalid-run:<target>:<diagnostic>`, so an invalid run cannot discharge coverage.

## Impact

- **Files**: traceability receipt parsing, conversion and admission; the evidence model and its constructors; the receipt contract article.
- **Testing**: the issue's table through strict manifest admission (valid pair passes; exit-zero negative and artifact-free negative deny with visible reasons) and rejection of receipts with a dropped diagnostic.
- **Owner / consumer**: receipt-driven traceability and the `traceability scan` CLI. Execution of producers stays with #22 and #24.
