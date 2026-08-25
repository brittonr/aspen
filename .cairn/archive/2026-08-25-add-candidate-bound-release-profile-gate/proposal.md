# Add a candidate-bound release profile gate

## Why

Molten implements release profile validation, but operators and Nix checks cannot invoke it directly. Release review can therefore bypass the stricter profile checks or rely on unit tests alone.

## What Changes

- Bind release profile validation to an explicit candidate content reference.
- Expose validation through `molten test gate release-profile`.
- Add a Nix conformance check with positive and negative command fixtures.
- Document the command as review evidence that does not grant runtime or release authority.

## Impact

This change affects the release profile core, the evidence gate CLI, Nix checks, tests, and operator documentation. It does not claim that the repository is ready for broad production release.
