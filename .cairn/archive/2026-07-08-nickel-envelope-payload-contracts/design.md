# Design: Nickel envelope payload contracts

## Scope

This change strengthens authoring-time Nickel exports for plugin extension contracts and grants. It does not make envelope metadata runtime authority and does not replace Rust Preserves validation.

## Proof checklist

- **Proof claim**: plugin extension export envelopes cannot pair a known schema id with an unvalidated or mismatched payload.
- **Out of scope**: executing Nickel in production runtime admission.
- **Trusted assumptions**: checked-in generated JSON and Preserves evidence remain reviewed artifacts.
- **Positive evidence**: valid contract and grant envelopes export and match generated drift-gated JSON.
- **Negative evidence**: wrong schema id, wrong payload shape, unsupported source, missing identity, and identity/payload mismatch fail export.
- **Canonical refs**: generated envelope JSON refs and checked Preserves export refs where present.
- **Regeneration command**: contract export drift gate.

## Functional core

In Nickel, define two typed envelope contracts: one whose payload is `PluginExtensionContract`, and one whose payload is `PluginCapabilityGrant`. Add pure predicates that derive the expected export identity from the payload and compare it to envelope metadata.

## Imperative shell

The Nix drift gate exports the Nickel fixtures and compares generated JSON. Runtime Rust continues to consume checked-in canonical evidence only.

## Migration

Replace `payload | Dyn` for plugin extension envelope fixtures with schema-specific envelope contracts. Keep the generic metadata helper available only for low-risk documentation fixtures or future typed wrappers.
