# Design: plugin grant contract binding

## Scope

This change strengthens plugin extension and capability grant authoring contracts. Runtime admission still depends on canonical Preserves evidence, Rust parser validation, plugin lifecycle state, authority, policy, resource, effect, and provenance gates.

## Proof checklist

- **Proof claim**: authored plugin grants cannot drift from the extension contract descriptor they claim to authorize.
- **Out of scope**: making a grant sufficient authority without runtime UCAN/policy/effect/resource admission.
- **Trusted assumptions**: the supplied extension contract export is reviewed and drift-gated.
- **Positive evidence**: the storage grant fixture validates against the storage extension contract.
- **Negative evidence**: wrong descriptor, operation mismatch, schema mismatch, resource over-scope, replay mismatch, missing revocation evidence, and invalid validity windows fail export.
- **Canonical refs**: contract envelope ref, grant envelope ref, generated JSON refs, Preserves grant refs.
- **Regeneration command**: plugin contract export drift gate and focused plugin host tests.

## Functional core

In Nickel, add a pure binding predicate that looks up the grant's descriptor in the supplied extension contract by operation and descriptor ref, then compares schema refs, replay class, effect refs, resource scope membership, contract profile, and grant evidence fields.

## Imperative shell

The Nix drift gate evaluates the bound grant fixture and negative fixtures. Runtime Rust keeps its existing fail-closed checks and treats Nickel output as reviewed input evidence only.

## Migration

Add a bound grant wrapper fixture first, then migrate existing grant envelope fixtures to use it once generated output drift is reviewed.
