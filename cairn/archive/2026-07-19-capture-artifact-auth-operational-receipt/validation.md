# Validation: Molten artifact-auth operational receipt

Validated on 2026-07-19 from implementation commit `82fcca2`.

## Focused evidence

- `cargo test artifact_auth_operational_receipt` passed positive restart/reopen replay and negative rotation, revocation, wrong-namespace, malformed, and tamper cases.
- `cargo fmt` passed for the touched cryptographic-identity files.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo octet check -p molten --artifact-dir target/octet-artifact-auth-ops-v4 -- --lib` completed warning-only with 1,996 warnings and zero errors. The operational split removed the newly introduced file-length finding and the operational code has no boolean-name finding; the remaining inventory is pre-existing repository-wide Octet debt.
- Cairn validate plus proposal, design, and tasks gates passed.

## Full evidence

- `cargo test --workspace` passed.
- `nix flake check -L --option builders ''` passed, including nextest, structural authority checks, NixOS VM, and dogfood/release-evidence checks.
- Cairn sync dry-run passed with receipt `d6c0f6818a94f0a24063717f1a178497a194b8a4959093237d0921af4f0a16db`.
- Cairn sync execution passed with receipt `3a04d919ca13b9ca9a3fc66d873e9f7b6052220e57ed798d7e54572671e3f657`; all four accepted requirement IDs are present in `cairn/specs/artifact-auth-operational-receipt/spec.md`.

## Bounded result and blocker

The exercised shell performs a real capability-file key operation, writes through the product `Receipts` namespace, reopens node state, and checks actual generation plus durable revocation state before replay. This evidence remains local and observational. No network revocation authority, membership/capability/federation authority, or cross-consumer admission review exists, so standalone authority remains unadmitted and rollback remains available.

The advisory VibeThinker audit was unavailable with `fetch failed`; deterministic source, test, Octet, Cairn, VM, dogfood, and Nix evidence remains authoritative.
