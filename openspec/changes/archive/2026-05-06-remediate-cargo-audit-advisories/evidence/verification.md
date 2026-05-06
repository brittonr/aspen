# Cargo audit remediation verification

Status: captured
Captured: 2026-05-06

## Commands and outcomes

- command: `cargo metadata --locked --format-version 1 >/tmp/aspen-cargo-metadata-locked.json`
  - result: pass; lockfile resolves after dependency updates.
- command: `cargo audit -n --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2026-0066 --json > /tmp/aspen-cargo-audit-final.json`
  - result: pass; final local audit has 0 vulnerabilities, with warning-only advisories preserved for separate tracking.
- command: `nix build .#checks.x86_64-linux.audit --no-link -L`
  - result: pass; flake audit gate builds `/nix/store/fb32qsys2wwmwr41jwf8ahcs0jkq7wh2-crate-audit-0.0.0` and reports only 14 allowed warnings.
- command: `cargo check -p aspen-snix -p aspen-ci-executor-nix -p aspen-nix-cache-gateway --all-targets`
  - result: pass after repairing stale migration-test trait imports; validates snix/archive/TLS caller surfaces touched by lock updates.
- command: `cargo test -p aspen-snix migration -- --nocapture`
  - result: pass; 22 matching migration tests passed across unit/integration targets.

## Captured artifacts

- `evidence/cargo-audit-initial-summary.json`
- `evidence/cargo-audit-post-remediation-summary.json`
- `evidence/cargo-audit-final-summary.json`
- `evidence/advisory-inventory.md`
- `evidence/remediation-summary.md`
