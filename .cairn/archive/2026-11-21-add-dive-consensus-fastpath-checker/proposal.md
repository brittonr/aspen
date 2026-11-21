## Why

Aspen issue [#62](https://github.com/brittonr/aspen/issues/62) asks for independent confirmation of the existing consensus fast-path model results with Dive 0.1.0. TLC remains authoritative; a third checker must establish semantic agreement, not just parse the inputs.

## What Changes

- Add the exact upstream x86_64 Linux Dive release to the existing Nix parity check, with its reviewed version, commit, archive SHA-256 and notices.
- Compare all three results on the unchanged baseline and missing-promise configurations, retain raw reports and digests, and deny incomplete or divergent results.

## Impact

- **Files**: `verification/tla-ts/`, `nix/consensus-fastpath-model.nix` and `docs/consensus-fastpath-tla.md`.
- **Testing**: Synthetic report-reader tests and the existing `checks.x86_64-linux.consensus-fastpath-model-parity` Nix check.

## Out of Scope

No TLA+ model, config, invariant, temporal property, budget or CI structure changes. No live-engine trace validation, Quint-to-Rust replay changes, performance claims or implementation proof.
