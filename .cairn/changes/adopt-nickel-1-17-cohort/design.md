# Design: Adopt the Nickel 1.17 evaluator cohort

## Context

Molten embeds `nickel-lang 2.1.0` for configuration and policy surfaces. Its Nix shell uses Nickel CLI `1.16.0`.

The update must preserve the functional-core and imperative-shell boundary.

## Decisions

### Decision: Pin the complete cohort

**Choice:** Use `nickel-lang 2.2.0`, `nickel-lang-core 0.18.0`, and Nickel CLI `1.17.0`.

Bind source-built CLI use to commit `1320a983e6c3d1e2fb53dd2464b084b4903b1426`.

**Rationale:** One cohort reduces parser and contract drift between development and runtime paths.

### Decision: Keep product policy outside the evaluator

**Choice:** Molten retains contracts, defaults, profile meaning, authority, and target decoding.

Nickel remains an evaluator dependency. It does not become an authority or runtime-policy owner.

**Rationale:** A dependency update must not widen evaluator responsibility.

### Decision: Prove fail-closed compatibility

**Choice:** Run valid policy and configuration fixtures. Also run malformed, missing-import, contract-denial, oversized, and secret-bearing negative fixtures.

Stable error categories and redaction rules remain the compatibility surface.

**Rationale:** Valid fixtures cannot establish rejection behavior.

### Decision: Record bounded provenance

**Choice:** Release evidence records the crate versions, CLI version, upstream commit, and check results.

Evidence must keep the existing non-claims for policy correctness and runtime behavior.

## Flow

```text
exact Nickel cohort
  -> embedded evaluator adapter
  -> Molten-owned policy and decoding
  -> positive and negative fixtures
  -> repository release gates
```

## Risks and trade-offs

- Upstream API changes can affect error conversion and value decoding.
- A valid configuration can still describe an unsafe or unauthorized operation.
- Diagnostic text can change. Tests must retain stable categories and bounded context.
