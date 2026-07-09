## Why

Molten has several configuration sources: Nickel profile exports, CLI flags, local fixture defaults, Nix checks, Cairn policy, and ledger-resolved evidence refs. Operators and reviewers need a deterministic way to see the effective configuration and how each value was chosen before trusting receipts or debugging drift.

Without an effective-config readback, configuration remains reviewable only by manually stitching together docs, CLI invocations, and emitted receipts.

## What Changes

- Add an effective-config readback model that records effective values, source provenance, defaults, overrides, profile refs, and caveats.
- Add CLI surfaces such as `molten config validate`, `export`, `explain`, `diff`, and `fingerprint` for source-controlled and runtime profile inputs.
- Define canonical BLAKE3 fingerprints over normalized effective config artifacts.
- Keep readback evidence explicitly non-authoritative: it helps review and diagnostics but does not replace subsystem gates.
- Add positive and negative fixtures for valid config readbacks, hidden local defaults, conflicting overrides, stale profile refs, and non-canonical rendered output.

## Impact

- **Files**: config/readback core, CLI config commands, docs, profile tests, and receipt/readback schemas.
- **Testing**: pure-core tests for normalization/diff/fingerprint plus CLI fixture tests for validate/explain output.
- **Safety**: effective-config readbacks are evidence-only diagnostics. They do not grant authority, policy admission, provenance trust, source-gate acceptance, resource rights, retention clearance, transport trust, or release eligibility.
