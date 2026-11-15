## Phase 1: Models and rail

- [x] [serial] Add bounded TLA+ modules and configurations for #44, #46, #54, #55, #56, #57 and #59 with source mappings and non-claims.
- [x] [parallel] Add witness configurations showing passing #44 configurations reach post-election reads.
- [x] [serial] Add `expectations.tsv` and `check.sh` with TLC and TLA-ts parity.
- [x] [serial] Add the `audit-findings-tla` flake check and wire it into `flake.nix`.
- [x] [parallel] Mutation-check the rail: a wrong expectation fails; the key-file fix without a directory sync violates `ReportedRotationDurable`.

## Phase 2: Validation

- [x] [serial] Execute the rail derivation in the Nix sandbox: 23 of 23 configurations matched, TLC and TLA-ts agreed on every verdict and on all exhaustive counts. Raw output retained in `evidence/tla-audit-findings-2026-11-15/`.
- [ ] [serial] Evaluate the rail through the repository flake (`nix build .#checks.x86_64-linux.audit-findings-tla`). The first execution used registry nixpkgs with tlaplus 1.7.4, not the flake-locked nixpkgs revision.
- [ ] [serial] Run strict Cairn validation with the sibling provider policy.
- [ ] [serial] Run the commit and push hooks (whitespace, formatting, Octet, non-strict Cairn).

## Phase 3: Review and integration

- [ ] [serial] Review the models against the cited source and the issue text.
- [ ] [serial] Sync, archive, and integrate by fast-forward onto `origin/molten`.
