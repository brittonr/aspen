# Consume artifact-auth from Radicle-backed HTTPS

## Why

Molten already consumes the reviewed `artifact-auth` revision `799459346d5416fbd7b9f55840a7371441b55afa`, but Cargo, Nix, unit2nix plans, and the release-dependency profile identify GitHub over SSH. The source has no registry release, so that transport remains an avoidable bootstrap dependency after the same Git object was accepted and published through the governed Radicle HTTPS adapter.

## Outcome

Change only the source transport to the accepted public Radicle HTTPS URL. Preserve the exact Git object, Nix content identity, package versions, crate graph, generated build-plan semantics, cryptographic behavior, and Molten authority boundaries. Remove every executable GitHub fallback for this dependency and emit typed cutover evidence.

## Scope

- Align both Cargo manifests, `Cargo.lock`, `flake.nix`, `flake.lock`, the release-dependency Nickel profile, and both pinned unit2nix plans on one Radicle HTTPS URL and exact revision.
- Preserve `artifact-auth-core` and `artifact-auth-ed25519` behavior and Molten's existing identity, key, membership, capability, transport, runtime, evidence, and release authority.
- Validate focused positive and adversarial identity behavior, exact source admission, generated-plan agreement, and fallback rejection.
- Retain GitHub only as historical prose or package metadata, never as an executable artifact-auth dependency fallback.

## Non-goals

This change does not migrate Molten itself to Radicle, move Molten to the unified `artifact` workspace, alter artifact-auth or Molten Rust APIs, claim semantic equivalence, enable artifact-auth in Radicle CI, or grant source transport any runtime or release authority.

## Impact

- **Files**: Cargo/Nix manifests and locks, generated unit2nix plans, typed Nickel release policy, source-admission checks, documentation, evidence, and Cairn lifecycle artifacts.
- **Testing**: Focused identity tests before and after cutover, positive/negative receipt and release-profile validation, lock/plan/source agreement, formatting, and focused Nix checks.
