## Why

Multiple Aspen CLI gates and Nix flake checks manufacture canonical `decision=pass` receipts from caller-supplied placeholder strings, synthetic fixtures, or stdout greps without executing the claimed runtime or validating real receipts:

1. `checks.dogfood-local-node` (`flake.nix:4646-4758`) self-signs receipts with a hardcoded key and passes `--octet-evidence "octet:external-clean-gate-required"` and `--cairn-evidence "cairn:external-strict-validate-required"`. `src/operator/parts/dogfood/p004/body.rs:176-266` checks only `!trim().is_empty()`, hashes the placeholder text, and emits a signed `decision=pass` release-promotion receipt.
2. Six top-level `checks.nixos-vm-*` checks (`nixos-vm-smoke`, `nixos-vm-live-control`, `nixos-vm-service-job`, `nixos-vm-restart`, `nixos-vm-fault`, `nixos-vm-aggregate` in `flake.nix:1418-1456, 3821-3876`) execute zero VMs and call `molten test nixos-vm shard-run ... --claimed-decision pass` over dummy strings (`<vm-shard-fixture ...>`), minting canonical `decision "pass"` receipts.
3. `checks.nixos-vm-multinode` (`flake.nix:4760-4783`) boots two unsigned VMs, asserts `molten-node.service` fails (`source-gate-trust.json does not exist`) via `journalctl` grep, and runs `ping` — executing zero Aspen node logic despite `AGENTS.md` and `docs/distributed-testing.md:140` describing it as actual VM integration.
4. `checks.requirement-traceability-gate` and `checks.deterministic-drift-gate` (`flake.nix:3878-3974`) test only disposable 1-file self-copies rather than Aspen's real tree, and multiple flake checks gate on `grep` against stdout `.txt` or `journalctl` in violation of Aspen's receipt-first rule.
5. `tests/evidence-matrix.ncl` uses 100% placeholder hashes (`blake3:000...0101`–`0222`), covers only 10 of 3,135 requirements, and aliases one `--ignored` test across 10 rows (both positive and negative); 6 of 7 product capabilities in `config/capability-report/capabilities.json` lack native execution producers.

## What Changes

- Require `dogfood release-promote` to parse and validate canonical signed/verified Octet and Cairn gate receipts rather than accepting arbitrary non-empty strings or placeholder text.
- Remove caller-supplied `--claimed-decision` pass minting from `molten test nixos-vm shard-run`; separate fixture-schema unit tests from real VM execution checks, and rename/upgrade `checks.nixos-vm-multinode` so unsigned denial and real multi-node VM execution are distinct and honestly named.
- Replace toy self-copy checks in `flake.nix` (`requirement-traceability-gate`, `deterministic-drift-gate`) with real repository checks and replace stdout/`journalctl` `grep` gating with canonical Preserves receipt validators.
- Replace placeholder hashes and single-test aliasing in `tests/evidence-matrix.ncl` with exact requirement-bound entries, and build native execution producers for the six product capabilities in `config/capability-report/capabilities.json`.

## Impact

- **Files**: `src/operator/parts/dogfood/p004/body.rs`, `src/nixos/vm.rs` (and parts), `flake.nix`, `tests/evidence-matrix.ncl`, `config/capability-report/capabilities.json`, `src/capability_qualification/`, `AGENTS.md`, and `docs/distributed-testing.md`.
- **Verification**: Negative tests MUST verify that placeholder strings, `--claimed-decision pass` forgery, self-copied fixtures, and aliased ignored tests all fail closed.
- **Non-claims**: Eliminating self-certifying gates ensures receipts cannot be minted from dummy text; it does not by itself supply external cloud, multi-host, or production release qualification.

## Out of Scope

- Fixing `layered_proof_diagnostics` or `tools/tracey/inherited_debt_guard.rs` (tracked in `harden-traceability-proof-composition-and-static-audits`).
- Expanding nextest profiles to `molten-core` (tracked in `close-ci-verification-blind-spots`).
