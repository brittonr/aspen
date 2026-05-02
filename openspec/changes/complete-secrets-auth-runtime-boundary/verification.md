## Verification

- Evidence: `evidence/secrets-auth-runtime-compatibility.txt` records focused checks:
  - `cargo check -p aspen-secrets --no-default-features`
  - `cargo test -p aspen-secrets --no-default-features --no-fail-fast`
  - `cargo check -p aspen-secrets --features auth-runtime`
  - `cargo check -p aspen-secrets-handler --no-default-features`
  - `cargo check --bin aspen-node --features node-runtime-apps,blob,automerge,secrets,forge`
- Evidence: `evidence/secrets-auth-runtime-forbidden-boundary.txt` records the no-default dependency tree and forbidden-runtime grep.
- Evidence: readiness remains tracked by `docs/crate-extraction/trust-crypto-secrets.md`; this slice does not promote readiness.
