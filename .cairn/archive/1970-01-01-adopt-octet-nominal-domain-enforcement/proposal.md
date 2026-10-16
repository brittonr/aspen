## Why

The accepted `authority-identity-revocation` requirements condition Octet adoption on a provider policy becoming available. The old pinned Octet cohort lacks that policy, while the current provider implements `primitive_domain_alias`, `raw_domain_value`, and `newtype_invariant_bypass`. Molten must adopt the reviewed provider rather than leave a future-policy declaration standing in for enforcement.

The existing fifteen-domain profile uses logical migration labels, not the exact physical scopes consumed by Octet. Those labels also overstate the observed adoption: typed sets exist in `crates/molten-core/src/nominal.rs`, but they do not establish wholesale migration of node control, handler admission, effect admission, retention, cache, or replay implementations.

## What Changes

- Inventory all fifteen nominal aliases and the exact qualified typed core fields already present in the admitted reference sets.
- Mirror the reviewed provider catalog row contract in the human-authored Nickel inventory, explicitly identifying its review-only role; Octet's embedded generated catalog remains the runtime authority.
- Select `[octet] nominal_domain_mode = "deny"` without exemptions or disabled checks.
- Adopt an immutable provider cohort containing these rows and exports through coordinated parent-owned pin changes.
- Exercise real consumer-positive and deliberate raw-String-negative checks on that same cohort, preserving exact wire DTOs and excluding borrowed raw representations from claims the provider does not implement.
- Add a real Nix check hook and meaningful traceability only at actual enforcement/verification locations. Do not use an alias declaration, mock, compile-fail Rust test, or declaration-only marker as proof.

## Impact

- Consumer-owned files: `config/nominal-reference-domains.ncl`, `dylint.toml`, `docs/nominal-authority-references.md`, and this native Cairn package.
- Coordinated ownership: provider catalog/export changes belong to Octet's worker; root flake pins, check integration, runtime proof, and lifecycle decisions belong to the parent. Any narrowly required source repair must be coordinated explicitly.
- Wire compatibility: Preserves DTO strings, projections, schemas, historical fixtures, display strings, and diagnostic strings remain unchanged.
- Acceptance: proposal/design/tasks gates, immutable pinning, actual positive and negative Octet outcomes, canonical compatibility checks, review, and strict native lifecycle validation remain required. This proposal does not accept or archive unfinished work.
