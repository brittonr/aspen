## Context

Several proposed modularity changes depend on dependency direction staying clean. Manual review alone will miss regressions as the codebase grows. The boundary gate should be cheap enough to run with focused validation and precise enough to avoid noisy false positives.

## Design

### Rule source

Rules should be declared in a reviewed source-controlled configuration. Nickel is preferred for human-authored policy because contracts can validate rule identity, path globs, allowed dependencies, denied dependencies, and diagnostic metadata before a generated runtime input is refreshed.

### Validator behavior

The validator can initially scan Rust source imports and module paths. It should produce deterministic diagnostics with:

- rule id;
- source file;
- forbidden target or pattern;
- recommended owning layer or exemption path.

The validator should fail closed for unknown rule ids, malformed configuration, duplicate rule identities, and invalid path patterns.

### Fixture strategy

Positive fixtures demonstrate allowed imports for each layer. Negative fixtures demonstrate core-to-adapter, runtime-to-CLI, codec-to-domain, and unclassified public-export violations.

### Integration

The check should be runnable as a focused command and listed in the relevant validation notes. Later changes may wire it into Octet, Cairn release-readiness, or Nix checks, but the first slice can be a standalone validated rail.

## Non-goals

- Do not require perfect Rust semantic analysis in the first slice.
- Do not block generated code without an explicit generated-code exemption path.
- Do not replace existing tests, Octet gates, or Cairn validation.
