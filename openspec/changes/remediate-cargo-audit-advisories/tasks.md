## Phase 0: Spec foundation

- [x] Create OpenSpec package for cargo-audit advisory remediation.

## Phase 1: Advisory inventory

- [ ] Capture `cargo audit -n --ignore RUSTSEC-2023-0071 --json` and the Nix audit failure into change-local evidence.
- [ ] Classify each advisory by cluster, dependency path, owning Aspen surface, fix candidate, and initial disposition.

## Phase 2: Wasmtime/plugin runtime cluster

- [ ] Update, prune, or otherwise remediate the `wasmtime 36.0.3` advisory cluster without adding blanket ignores.
- [ ] Run focused plugin/runtime checks for affected crates and capture the remaining audit delta.

## Phase 3: TLS/certificate validation cluster

- [ ] Update, prune, or otherwise remediate `rustls-webpki 0.103.9` and `aws-lc-sys 0.38.0` advisories.
- [ ] Run focused transport/upstream-cache/TLS caller checks for the affected dependency paths and capture the remaining audit delta.

## Phase 4: Tar/archive extraction cluster

- [ ] Update, prune, or otherwise remediate `tar 0.4.44` and `astral-tokio-tar 0.5.6` advisories.
- [ ] Run focused materializer/snix/build-input checks for affected archive extraction paths and capture the remaining audit delta.

## Phase 5: Warning/exception policy

- [ ] Review warning-only advisories, including `vmm-sys-util 0.11.2`, and document whether each is fixed, pruned, or allowed with rationale.
- [ ] Ensure any new `cargo audit` ignore/allowance is advisory-specific and includes exposure rationale, compensating control, and removal trigger.

## Phase 6: Final verification and archive

- [ ] Run `cargo audit -n --ignore RUSTSEC-2023-0071` and confirm no unclassified vulnerabilities remain.
- [ ] Run `nix build .#checks.x86_64-linux.audit --no-link -L` and confirm the flake audit gate is green.
- [ ] Run strict OpenSpec validation and `git diff --check`.
- [ ] Archive the change after all remediation evidence is complete.
