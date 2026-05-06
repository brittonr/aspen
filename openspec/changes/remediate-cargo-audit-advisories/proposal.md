## Why

The repository `audit` flake check currently fails after the latest dependency advisory database refresh. `nix build .#checks.x86_64-linux.audit --no-link -L` reports `22 vulnerabilities found` and `14 allowed warnings found`, so Aspen no longer has a green dependency-security gate even though the worktree is clean.

The highest-risk clusters are dependency-owned security surfaces rather than ordinary compile failures:

- `wasmtime 36.0.3`: multiple 2026 advisories, including sandbox escape, out-of-bounds access, host data leakage, and resource-exhaustion classes. This touches Aspen's plugin/guest execution threat boundary.
- `rustls-webpki 0.103.9` and `aws-lc-sys 0.38.0`: certificate validation / CRL / name-constraint advisories. This touches TLS validation in transport and cache/upstream-client paths.
- `tar 0.4.44` and `astral-tokio-tar 0.5.6`: archive extraction and PAX handling advisories. This touches store/materialization/build-input handling.
- Existing warning debt such as `vmm-sys-util 0.11.2` remains visible through the snix/fuse stack and should stay explicitly classified rather than lost in new ignores.

## What Changes

Add a focused remediation plan for returning the Cargo audit gate to green without weakening Aspen's security posture:

- Inventory every current `cargo audit` vulnerability and warning into a checked-in, reproducible summary.
- Remediate the Wasmtime cluster first by updating or constraining plugin/runtime dependencies and validating Aspen plugin tests.
- Remediate the TLS cluster by updating or replacing the affected TLS/certificate-validation dependency path and validating transport/cache callers.
- Remediate the tar/archive cluster by updating affected archive crates or removing unsafe extraction paths, then validating materializer/snix/build paths.
- Preserve or tighten the audit gate: no broad `--ignore` additions without advisory-specific rationale, expiry/follow-up, and a compensating-control note.

## Scope

- **In scope**: Rust dependency updates, feature/edge pruning, Cargo/Nix lockfile updates, cargo-audit gate policy, advisory inventory evidence, targeted tests for Aspen surfaces affected by the dependency clusters.
- **Out of scope**: Rewriting the plugin sandbox architecture, replacing TLS providers wholesale without a focused follow-up, accepting raw secrets into logs/evidence, or treating warning-only advisories as resolved without explicit classification.

## Impact

- **Files likely touched**: `Cargo.toml`, `Cargo.lock`, `flake.nix`, `flake.lock` if Nix inputs or audit policy change, plugin/snix/transport crates if API updates require code changes, and OpenSpec evidence files.
- **Capabilities affected**: plugin execution hardening, TLS/certificate validation, archive/materialization safety, supply-chain audit governance.
- **Verification**: `cargo audit -n --ignore RUSTSEC-2023-0071`, `nix build .#checks.x86_64-linux.audit --no-link -L`, targeted cargo tests/checks for affected crates, `openspec validate remediate-cargo-audit-advisories --strict --json`, and `git diff --check`.
