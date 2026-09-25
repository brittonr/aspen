## Context

The shared Molten runtime follows a zero-ambient-import discipline but has no explicit admission fact that a component's import set is complete and declared. MoonBit's component-model workflow binds a WIT world, generates typed bindings, and keeps imports explicit so a runtime can serve a component with no hidden assumptions. The Molten host should enforce that same boundary as a typed admission decision.

## Decisions

### Decision: The manifest is the admission source of truth

**Choice:** every admitted component must declare its complete import table; the declaration is part of the admitted artifact facts.

**Rationale:** an undeclared import is an unverifiable ambient dependency. Declaring it first keeps admission deterministic and evidence bounded.

### Decision: WIT world matching is mandatory

**Choice:** imports and exports must match the declared WIT world exactly; drift fails before instantiation.

**Rationale:** MoonBit's typed-binding workflow shows that world drift is the practical failure mode. Rejecting early preserves the zero-import discipline.

### Decision: Admission stays pure

**Choice:** manifest, world, and import-set decisions live in a pure core family; the host only supplies observed artifact facts.

**Rationale:** this preserves the functional-core boundary and keeps runtime authority out of admission.

## Risks / Trade-offs

- WIT tooling versions drift; receipts bind tool and verifier identities so stale parses fail closed.
- Over-strict admission rejects useful components; the declared manifest allows exact exceptions instead of ambient defaults.
- Import-set facts need reliable extraction; the shell records the extraction tool identity alongside the manifest.
