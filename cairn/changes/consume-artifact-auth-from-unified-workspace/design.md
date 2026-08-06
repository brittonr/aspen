# Design: Unified Artifact source migration

## Context

Molten consumes two authentication packages from the predecessor source. It consumes `artifact-binding-core` from Artifact revision `c932138d880ddf4c2967f4c024b489b5c0022bf1`. The three packages can use one source identity without changing product logic.

## Decisions

### Use one Artifact source

Cargo will pin the three consumed packages to one repository and revision. Nix will expose one non-flake `artifact-src` input. The release profile and unit2nix plans will use that identity.

### Separate source breadth from consumer breadth

The source workspace contains four packages. Molten consumes the two authentication packages and binding package. Source validation will reject transfer package entry into the consumer lock.

### Keep validation logic pure

A pure Nix validator will receive parsed manifests, locks, plans, policy, source metadata, and resolved Nix identity. It will return deterministic issue codes. Shell code will only load files and report failures.

### Keep historical evidence immutable

The accepted Radicle receipt will remain unchanged. A new typed Nickel receipt will describe the active unified-source migration. The predecessor source will not remain executable.

### Preserve behavior and authority

No Molten Rust implementation will change. Existing positive and negative identity tests remain the behavioral oracle. Molten retains runtime and authority decisions.

## Risks

- Separate inputs can survive and hide mixed state. Validation will require one root Nix input.
- The transfer package can enter the consumer graph. The lock and plan checks will reject it.
- Generated plans can retain the predecessor source. Both plans will be regenerated and checked.
- Historical evidence can be mistaken for active state. The new receipt will mark it historical only.

## Rollback

Rollback is explicit. Restore reviewed source declarations, regenerate Cargo, Nix, and unit2nix artifacts, rerun checks, and record a new receiver decision. No automatic fallback is allowed.
