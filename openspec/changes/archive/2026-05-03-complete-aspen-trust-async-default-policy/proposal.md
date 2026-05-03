# Complete Aspen Trust Async Default Policy

## Why

The trust/crypto/secrets owner/public API review kept the family `workspace-internal` because `aspen-trust` still exposed async/Tokio service APIs from its default public surface. Reusable trust helper/state surfaces need an explicit default dependency policy before they can be considered canonical extraction candidates.

## What Changes

- Gate `aspen-trust` async service modules behind an explicit `async` feature.
- Keep default and no-default `aspen-trust` focused on pure helper/state/wire modules.
- Enable `aspen-trust/async` from `aspen-raft`'s `trust` feature for runtime compatibility.
- Add `aspen_trust` as a real checker/policy package candidate for the trust/crypto/secrets family.
- Record dependency, compatibility, readiness, OpenSpec, markdown, and diff evidence.

## Impact

- **Files**: `crates/aspen-trust`, `crates/aspen-raft`, extraction policy/checker/docs/specs.
- **APIs**: `aspen_trust::key_manager` and `aspen_trust::reencrypt` require the `async` feature.
- **Dependencies**: default/no-default `aspen-trust` must not include normal `tokio` or `async-trait` edges.
- **Testing**: focused `cargo check`/`cargo test`, dependency graph guards, readiness checker, OpenSpec validation, markdownlint, and diff hygiene.
