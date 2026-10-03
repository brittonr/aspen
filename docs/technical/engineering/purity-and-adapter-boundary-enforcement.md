# Purity and Adapter Boundary Enforcement

Molten's purity boundary is an ownership boundary for decisions and effects, not a claim that every root-crate module is already extracted into a minimal core. This article assumes Rust module familiarity and explains the inspected dependency validator and effect planners. The [modularity inventory](../../modularity-boundaries.md) remains authoritative about the migration slice and its non-claims. Return to the [Technical companion](../README.md).

## Separate a fact from the operation that obtained it

A deterministic planner can consume an explicit freshness fact without reading a clock. It can consume an adapter-support fact without opening a transport. It can decide whether a destructive operation is admissible without deleting anything. The shell is responsible for obtaining those inputs under the appropriate authority and executing permitted effects afterward.

The inventory classifies the minimal `molten-core` surface as standard-library-only. Root-crate identity helpers, codecs, policy/evidence tooling, runtime orchestration, adapters, CLI code, and integration shells are different dependency classes. Calling all of them “the runtime” loses the distinction needed to review an import. In particular, Nickel policy authoring is not live runtime authority, and adapter availability is not trust.

The [planning implementation](../../../crates/molten-core/src/planning.rs) makes this distinction concrete. `AdmissionInputs` contains `has_authority`, `evidence_fresh`, `resource_allowed`, and `adapter_supported`. These are supplied facts, not capabilities acquired by the planner. `EffectPlan` contains a decision, effect kinds, and diagnostics. Returning `EffectKind::StoreDelete` does not execute a store deletion; it describes an admitted effect for a shell to handle.

## What the import validator actually enforces

`validate_dependency_boundaries` accepts explicit `BoundaryRule` and `ImportFact` slices. In the [dependency implementation](../../../crates/molten-core/src/dependency.rs), it checks each import against each rule, skips matching exemption classes, compares source prefixes, and detects denied target prefixes. A diagnostic carries the rule identifier, source file, forbidden target, and guidance.

This is deliberately smaller than a Rust compiler or whole-program effect analysis. The function does not discover imports, resolve aliases, expand macros, or inspect a filesystem. Its result is only as complete as the facts and rules supplied by the caller. Although `ImportFact` includes `is_public_export` and `BoundaryRule` includes `owning_layer`, the inspected matching loop does not use those fields to infer additional denials. They must not be described as an independently enforced layer lattice.

Prefix matching also means the representation of facts matters. Reviewers need to understand whether a target is a module path, source path, or another normalized identifier before concluding that a rule covers it. A compatible public alias and its implementation file are not automatically interchangeable strings. The inventory explicitly preserves compatibility aliases and staged ordinal shards; moving a file alone is not evidence that a semantic boundary has been established.

## Destructive planning as a worked example

Consider an illustrative retention request with authority, fresh evidence, allowed resources, and a supported adapter. Suppose remote clearance is absent even though the local index is complete and the plan is not stale. `plan_retention_gc` first accumulates admission diagnostics, then checks remote clearance, index completeness, and plan staleness. The missing clearance causes its denial helper to return an empty effects vector. No delete effect is planned.

Changing the adapter from unavailable to available cannot supply missing clearance. Similarly, presenting a receipt about a prior operation cannot itself change `has_authority`. Those are different inputs with different owners. Once all required facts pass, this planner returns store-delete and receipt-write effects, but storage errors, concurrent filesystem changes, and receipt persistence remain shell concerns.

There is an important limit to generalizing from this example. `plan_registry_discovery` can return `BoundaryDecision::Deny` while retaining `RegistryRead` and `ReceiptWrite` effects when discovery is evidence-only. Therefore “every deny plan has no effects” is false for the inspected API. The useful invariant is narrower: the retention denial path does not return destructive effects. Consumers should interpret the decision together with the specific planner's effect contract, rather than assume one universal denial shape.

## Layered review rather than a single gate

A useful review follows the data in both directions. First identify the shell that acquires authority and obtains facts. Then inspect the pure decision and its denied cases. Finally identify the shell that interprets the resulting plan. This distinguishes two bugs that import checks alone cannot separate: an ambient operation moved into a supposedly pure module, and a shell that ignores a correctly returned denial.

Structural audit rules add another layer. The [authority audit guide](../../ast-grep-runtime-authority-audits.md) describes inventory rules and narrower blocking scopes for converted adapters. They can catch prohibited syntax but do not prove the semantic adequacy of admission facts. Conversely, planner tests can exercise missing authority without proving that every filesystem call is capability-relative.

Suggested verification includes the inventory's focused `cargo test -p molten-core` check, representative positive and negative import facts, and affected adapter-rule fixtures. These are review instructions, not checks executed for this article. Relevant permanent tests are embedded beside the dependency validator and planners.

## Limits

This account does not claim complete crate extraction, global absence of ambient effects in the root crate, effect execution atomicity, or end-to-end production safety. Duplicate enqueue planning is not an exactly-once guarantee. Canonical codec identity and public API compatibility remain separate obligations when boundaries move. The intended architecture is preserved by explicit inputs, bounded ownership, and scoped evidence—not by renaming a module “core.”

## Sources

- [Modularity boundary inventory](../../modularity-boundaries.md)
- [Runtime-authority audit guide](../../ast-grep-runtime-authority-audits.md)
- [Dependency validator and embedded tests](../../../crates/molten-core/src/dependency.rs)
- [Effect planners and embedded tests](../../../crates/molten-core/src/planning.rs)
- [Technical companion](../README.md)
