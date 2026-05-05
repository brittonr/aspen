## Why

Aspen now has many security-sensitive surfaces: Iroh-only client RPC, Raft-backed control plane state, Forge/federation/proxy paths, SNIX store/cache capabilities, dogfood evidence, jobs/CI execution, Nix build paths, tickets/tokens, and operator CLIs. Recent focused hardening slices fixed specific bypass classes, but there is not yet one OpenSpec that defines a full-system audit program, the evidence it must produce, and the fail-closed remediation policy for every authority boundary.

A full Aspen hardening audit should make the next autonomous security stint reproducible: enumerate boundaries, prove each protected operation has an authorization mapping and negative regression, confirm no credential material leaks into logs/receipts/errors, validate execution/sandbox contracts, and turn every finding into a small verified follow-up change.

## What Changes

- Add a full-system hardening audit capability covering Aspen security authority boundaries end to end.
- Require a machine-readable audit inventory that maps crates, request variants, token/ticket files, persisted key prefixes, external process boundaries, and operator outputs to owning controls and evidence.
- Define an authorization matrix for client RPC, CI, Forge, federation, deploy, SNIX, secrets/trust, jobs, proxy/native handlers, and administrative operations.
- Require redaction/secret-handling checks across logs, receipts, CLI output, test fixtures, errors, and panic/debug formatting.
- Require negative tests or source-order drift tests for auth-before-dispatch, fail-closed request classification, reserved-prefix isolation, proxy token semantics, and credential output handling.
- Require prioritised remediation plans and small implementation slices for any finding.

## Capabilities

### New Capabilities

- `aspen-hardening-audit`: A reproducible full-system audit workflow with inventory, threat model, evidence, and remediation acceptance gates.

### Modified Capabilities

- `federation-auth-enforcement`: Audit must cover federation token propagation, proxy eligibility, and pull/push capability separation.
- `dogfood-evidence`: Audit evidence and receipts must remain useful without leaking credentials.
- `trust-crypto-secrets-extraction` and secrets specs: Audit must verify secrets stay in the intended core/runtime boundaries and are redacted at operator surfaces.
- `tigerstyle-audit`: Tiger Style remains a coding-quality gate, but this change adds a security-specific authority-boundary audit.

## Impact

- **Files**: New OpenSpec change under `openspec/changes/full-aspen-hardening-audit/`; later implementation slices will touch audit scripts, docs, tests, and security-sensitive crates.
- **APIs**: No immediate runtime API changes. Future remediation slices may tighten authorization mappings, CLI output, token issuance, or proxy eligibility.
- **Dependencies**: No new dependency in this spec-only slice. Future automation SHOULD prefer existing Rust/Nix tooling before adding dependencies.
- **Testing**: Validate this OpenSpec now; later implementation requires targeted cargo tests, Tiger Style, source-order/drift checks, negative fixtures, and dogfood/operator evidence checks.

## Out of Scope

- A third-party penetration test engagement is not defined here.
- Live cluster secret extraction, vault browsing, or printing token/ticket contents is forbidden.
- Broad rewrites are out of scope; findings must be drained as focused, reviewed, verified commits.
