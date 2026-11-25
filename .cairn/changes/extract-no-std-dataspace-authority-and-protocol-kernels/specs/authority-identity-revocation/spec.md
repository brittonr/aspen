# Authority identity revocation deltas

## ADDED Requirements

### Requirement: Capability delegation, authority currentness, revocation, and effect envelopes live in a no_std kernel
r[molten.authority.no_std_capability_authority_kernel] Capability tokens, claim selectors, delegation/attenuation, authority grant currentness, revocation cleanup, and effect-envelope validation MUST live in a `#![no_std]` + `alloc` crate (`molten-capability-authority-core`) built atop `molten-preserves-core`.

#### Scenario: Authority admission and revocation evaluate under no_std
- GIVEN a capability context, expected principal, current keys, and revocation records
- WHEN `molten-capability-authority-core` evaluates admission under `#![no_std]`
- THEN it MUST return deterministic canonical admission receipts without filesystem, network, or clock access.
