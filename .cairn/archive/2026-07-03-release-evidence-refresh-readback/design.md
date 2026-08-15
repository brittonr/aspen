# Design: release evidence refresh readback

## Scope

This change refreshes release-review evidence after the clippy and replay slices are complete. It is an evidence regeneration and readback-binding slice, not a behavior change to the release workflow itself.

## Proof checklist

- **Proof claim**: release evidence for the candidate tree is current, mutually bound, and sufficient for internal release review within existing caveats.
- **Out of scope**: broad production deployment approval, external publishing, new release authority, and changing source-gate policy.
- **Trusted assumptions**: Nix checks are hermetic for the declared flake inputs, and signed receipt verification correctly enforces key/purpose/currentness.
- **Positive evidence**: `nix build .#checks.x86_64-linux.nextest`, `nix build .#checks.x86_64-linux.dogfood-local-node`, release bundle verify, release promote, promotion summary, release export, and export verify pass for the same output path.
- **Negative evidence**: stale, missing, duplicate, unsigned, wrong-purpose, or tampered release members deny through existing release workflow tests or explicit manual verification notes.
- **Canonical refs**: nextest output path, dogfood report refs, release gate refs, replay verify/index refs, bundle verify refs, signed member refs, promotion refs, summary refs, export manifest refs, and export verify refs.
- **Regeneration command**: `nix build .#checks.x86_64-linux.nextest` followed by `nix build .#checks.x86_64-linux.dogfood-local-node` and release bundle/export verification commands from README.

## Functional core

No release workflow decision-law changes are planned. The shell executes Nix and release commands; existing pure validators continue to bind refs and decisions.

## Non-goals

- No new receipt schema unless stale evidence reveals a missing binding.
- No production pilot expansion.
- No Octet lint remediation beyond requiring current source-gate evidence already defined by accepted specs.
