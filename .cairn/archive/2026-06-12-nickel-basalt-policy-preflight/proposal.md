# Change: nickel-basalt-policy-preflight

## Why

Policy gate evidence previously carried marker strings for Nickel compatibility and Basalt preflight. After budget, actor registry, and capability fixtures became mandatory, policy preflight is the remaining place where pass evidence can rely on placeholders instead of an executable contract boundary.

## What

- Normalize harness admission policy through a deterministic Nickel static source/export step before runtime turns.
- Bind policy gates to canonical policy refs, Nickel source refs, Nickel export refs, Basalt contract envelopes, and Basalt preflight receipts.
- Reject missing, stale, or tampered policy preflight evidence during report validation and pass-evidence gating.
- Keep unreviewed Steel/dynamic predicates fail-closed until reviewed callable receipts exist.

## Impact

Reports and gate receipts gain additional policy evidence refs. Older reports with marker-only policy gates no longer satisfy evidence-bearing validation.
