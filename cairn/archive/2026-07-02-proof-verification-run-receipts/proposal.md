## Why

Traceability currently records target paths, command text, and content refs, but reviewers still need to trust that the named command actually ran and produced the referenced artifacts. A canonical verification-run receipt closes that gap by turning test execution itself into deterministic evidence that traceability can validate.

## What Changes

- Add a `verification-run-receipt-v1` evidence artifact for positive and negative proof runs.
- Bind argv, working surface, toolchain or Nix refs, exit status, produced artifact refs, stdout/stderr refs, requirement id, and coverage kind.
- Teach traceability to prefer receipt-backed evidence over hand-entered command strings.
- Add Hegel RS property tests for receipt determinism, stale artifact denial, and command/ref binding.

## Impact

- **Files**: traceability core and CLI, Preserves receipt schemas, tests, README proof workflow docs.
- **Testing**: unit tests for pass/deny receipts, CLI fixtures for real command receipts, and Hegel RS properties over generated receipt inputs and stale evidence cases.
