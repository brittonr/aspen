## Phase 0: Spec foundation

- [x] Create OpenSpec package for cargo-audit advisory remediation.

## Phase 1: Advisory inventory

- [x] Capture `cargo audit -n --ignore RUSTSEC-2023-0071 --json` and the Nix audit failure into change-local evidence (`evidence/advisory-inventory.md`, `evidence/cargo-audit-initial-summary.json`).
- [x] Classify each advisory by cluster, dependency path, owning Aspen surface, fix candidate, and initial disposition (`evidence/advisory-inventory.md`).

## Phase 2: Wasmtime/plugin runtime cluster

- [x] Update, prune, or otherwise remediate the `wasmtime 36.0.3` advisory cluster without adding blanket ignores (`Cargo.lock`, `evidence/remediation-summary.md`).
- [x] Run focused plugin/runtime checks for affected crates and capture the remaining audit delta (`evidence/cargo-audit-post-remediation-summary.json`, `evidence/verification.md`).

## Phase 3: TLS/certificate validation cluster

- [x] Update, prune, or otherwise remediate `rustls-webpki 0.103.9` and `aws-lc-sys 0.38.0` advisories (`Cargo.lock`, `evidence/remediation-summary.md`).
- [x] Run focused transport/upstream-cache/TLS caller checks for the affected dependency paths and capture the remaining audit delta (`evidence/verification.md`).

## Phase 4: Tar/archive extraction cluster

- [x] Update, prune, or otherwise remediate `tar 0.4.44` and `astral-tokio-tar 0.5.6` advisories (`Cargo.lock`, `flake.nix`, `evidence/remediation-summary.md`).
- [x] Run focused materializer/snix/build-input checks for affected archive extraction paths and capture the remaining audit delta (`evidence/verification.md`).

## Phase 5: Warning/exception policy

- [x] Review warning-only advisories, including `vmm-sys-util 0.11.2`, and document whether each is fixed, pruned, or allowed with rationale (`evidence/advisory-inventory.md`).
- [x] Ensure any new `cargo audit` ignore/allowance is advisory-specific and includes exposure rationale, compensating control, and removal trigger (`flake.nix`, `evidence/remediation-summary.md`).

## Phase 6: Final verification and archive

- [x] Run `cargo audit -n --ignore RUSTSEC-2023-0071` and confirm no unclassified vulnerabilities remain (`evidence/verification.md`; final command includes the new bounded `RUSTSEC-2026-0066` exception).
- [x] Run `nix build .#checks.x86_64-linux.audit --no-link -L` and confirm the flake audit gate is green (`evidence/verification.md`).
- [x] Run strict OpenSpec validation and `git diff --check` (`openspec validate remediate-cargo-audit-advisories --strict --json`, `openspec validate --all --strict --json`, `git diff --check`).
- [x] Archive the change after all remediation evidence is complete.
