## Why

An external static audit at `8ae93a5e5` filed issues #44, #46, #54, #55, #56, #57 and #59. Each describes a protocol or state-machine behaviour that bounded model checking can reproduce exactly: a stale linearizable read after a leader change, crash-unsafe key rotation, failed release effects recorded as completed, reservations stuck in `Attempting`, an uncompacted Raft log and unbounded duplicate map, a rate limit that never resets, and lifetime-bounded ingress. Executable models give each issue a reproducible counterexample and give each proposed fix a checked acceptance condition before source changes begin.

## What Changes

- Add six bounded TLA+ modules under `verification/tla/audit/`, one configuration per scenario, and an `expectations.tsv` table recording the required outcome of every configuration.
- Add `verification/tla/audit/check.sh`, which runs every configuration through TLC 1.7.4 and the pinned, unchanged `tla-ts@0.0.10` publication, fails on any outcome that differs from the table, requires matching verdicts and invariant names, and requires equal distinct-state counts for exhaustive passes.
- Add the `audit-findings-tla` flake check in `nix/audit-findings-tla.nix`.
- Retain the raw TLC and TLA-ts outputs of the first rail execution in `evidence/tla-audit-findings-2026-11-15/`.

## Impact

- **Files**: `verification/tla/audit/`, `nix/audit-findings-tla.nix`, one import in `flake.nix`, and the evidence directory. No Rust source changes.
- **Requirements**: no requirement changes. The models record current behaviour and candidate fixes; they do not admit any fix.
- **Testing**: 23 configurations. Ten mirror current code and must keep failing; when a fix lands, its configuration and expectation change in the same change as the source.
- **Non-claims**: bounded models with atomic message delivery and no loss or reordering. A counterexample is a real behaviour of the protocol; a pass is evidence only within the stated bounds. The models are not live-engine proofs, production qualification, or release evidence. #54 and #55 are latent because no production dispatcher exists.
