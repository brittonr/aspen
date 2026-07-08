# Design: Preserves boundary field contracts

## Scope

This change tightens the shape layer for schema-backed Preserves boundaries. It adds reusable field contracts but does not move subsystem-specific authority decisions out of their existing semantic gates.

## Proof checklist

- **Proof claim**: high-risk boundary shape validation rejects malformed field domains before semantic admission.
- **Out of scope**: proving that a shape-valid record has authority or provenance.
- **Trusted assumptions**: each field contract is reviewed as part of its boundary family.
- **Positive evidence**: existing canonical fixtures pass after broad fields are narrowed.
- **Negative evidence**: invalid decision strings, empty required ref sets, duplicate unique refs, oversized lists, and unsupported embedded records deny.
- **Canonical refs**: boundary schema ref, field contract descriptors, validation report ref.
- **Regeneration command**: focused `preserves_rail` schema tests and affected subsystem parser tests.

## Functional core

Introduce a pure `BoundaryFieldContract` model that can express type class, optional vocabulary, non-empty requirements, sequence limits, uniqueness rules, and embedded record labels. Validation consumes the contract and an `IOValue` field and returns deterministic diagnostics.

## Imperative shell

Existing parsers call the strengthened boundary validator before semantic parsing. No field contract performs filesystem, network, clock, process, or runtime policy access.

## Phasing

Start with boundary families already guarded by `validate_boundary_schema`. Promote additional record families only after positive and negative fixtures exist for the narrowed contracts.
