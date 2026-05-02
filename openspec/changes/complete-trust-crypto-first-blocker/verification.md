# Verification

## Task Coverage

- Gate `aspen-crypto` identity helpers behind `identity`.
  - Evidence: `crates/aspen-crypto/Cargo.toml`, `crates/aspen-crypto/src/lib.rs`,
    `openspec/changes/complete-trust-crypto-first-blocker/evidence/trust-crypto-secrets-forbidden-boundary.txt`
- Replace concrete `iroh` with `iroh-base` key types for identity consumers.
  - Evidence: `crates/aspen-crypto/src/identity.rs`,
    `crates/aspen-secrets/src/sops/provider.rs`,
    `openspec/changes/complete-trust-crypto-first-blocker/evidence/trust-crypto-secrets-compatibility.txt`
- Preserve `aspen-secrets::identity` compatibility re-exports.
  - Evidence: `crates/aspen-secrets/Cargo.toml`, `crates/aspen-secrets/src/identity/mod.rs`,
    `openspec/changes/complete-trust-crypto-first-blocker/evidence/trust-crypto-secrets-compatibility.txt`
- Fix no-default secrets tests for split KV capability traits.
  - Evidence: `crates/aspen-secrets/src/mount_registry.rs`,
    `openspec/changes/complete-trust-crypto-first-blocker/evidence/trust-crypto-secrets-compatibility.txt`
- Capture downstream metadata and readiness evidence.
  - Evidence: `openspec/changes/complete-trust-crypto-first-blocker/evidence/trust-crypto-secrets-downstream-metadata.json`,
    `openspec/changes/complete-trust-crypto-first-blocker/evidence/trust-crypto-secrets-readiness.md`

## Commands

Final commands are recorded in evidence artifacts and re-run before archive.
