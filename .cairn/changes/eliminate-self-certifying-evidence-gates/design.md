# Design

## Receipt-Verified Release Promotion

- Update `src/operator/parts/dogfood/p004/body.rs` so `release-promote` accepts canonical Octet gate and Cairn validation receipt payloads (not free-form text strings), decodes them strictly as canonical packed Preserves, verifies their schema kind, subject binding, and `decision == "pass"`, and fails closed on placeholder or unverified strings (`octet:external-clean-gate-required`, `cairn:external-strict-validate-required`).
- Keep local dogfood key material explicitly labeled as non-production fixture trust that cannot satisfy release-tier promotion gates.

## Honest VM Shard and Multinode Checks

- Remove `--claimed-decision` from `molten test nixos-vm shard-run` so shard decisions are derived solely from observed structured VM execution outcomes and verified artifacts, never caller-asserted CLI flags.
- Reclassify synthetic shard fixture checks (`vmShardCheck` in `flake.nix`) out of `checks.nixos-vm-*` into explicit unit/schema fixture tests, rename the unsigned startup denial check to `nixos-vm-unsigned-startup-denial`, and wire a real multi-node VM execution check that boots admitted nodes and exercises cross-node transport/lifecycle operations.
- Update `AGENTS.md` and `docs/distributed-testing.md` so every Nix check name accurately reflects what executes.

## Canonical Receipt Gating in Flake Checks and Real Capability Producers

- Replace `grep -q 'decision=pass'` on `.txt` stdout and `journalctl` grepping in `flake.nix` (`dogfood-local-node`, `nextest-config`, `nixos-vm-multinode`) with canonical receipt verification invocations (`molten` CLI readback/verify commands checking packed Preserves receipts).
- Upgrade `requirement-traceability-gate` and `deterministic-drift-gate` to operate over Aspen's real repository state and independent re-execution outputs rather than `cp left.preserves right.preserves` or 1-file temporary trees.
- Audit `tests/evidence-matrix.ncl`: eliminate sequential dummy `blake3:000...` hashes, disallow mapping one ignored test to both positive and negative rows across multiple requirements, add missing mapped requirements from `config/capability-report/capabilities.json`, and implement real native execution producers (`molten evidence qualify`) for durable state recovery, effect uncertainty, cross-process transport, world replay/promotion/restore, and control-plane failover (keeping cross-host explicitly bounded until two-host targets exist).

## Pure Core vs Shell Boundaries

- Receipt validation, decision derivation, and matrix integrity checks remain pure deterministic functions; Nix VM orchestration, CLI file I/O, and subprocess execution remain thin shells.
