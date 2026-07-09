# Valence stack evidence adapter

Molten validates stack evidence locally before any runtime, release, transport, storage, or authority decision uses it. The `molten-core` Valence stack adapter maps Molten stack evidence members to Valence role/schema vocabulary and reports only identity compatibility.

The adapter is a pure in-memory core. Shells may load stack evidence artifacts, generated Valence policy rows, or receipts from files later, but file I/O, clocks, network access, process execution, and rendered output stay outside the adapter.

A passing adapter report means:

- every required Molten stack role is present exactly once;
- each member uses a valid `blake3:` artifact ref and supported Molten stack schema;
- each member carries the expected verification role for its Valence mapping;
- each mapped Valence role/schema row matches the reviewed adapter vocabulary;
- each member preserves the evidence-only non-claim boundary.

A passing adapter report does **not** grant runtime authority, release authority, transport trust, storage trust, UCAN authority, deployment approval, or permission to bypass subsystem gates. Downstream migration should first consume adapter reports as compatibility evidence, then separately require the existing subsystem authority, policy, provenance, source-gate, resource, release, and lifecycle receipts.

Implementation: `crates/molten-core/src/stack.rs` (`validate_valence_stack_adapter`).
Requirements: `r[molten.evidence.valence_stack_adapter.contract]`, `r[molten.evidence.valence_stack_adapter.validation]`, and `r[molten.evidence.valence_stack_adapter.docs]`.
