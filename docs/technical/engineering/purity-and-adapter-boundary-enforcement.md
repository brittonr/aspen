# Purity and Adapter Boundary Enforcement

Molten's purity boundary is an ownership boundary for decisions and effects, not a claim that every root-crate module is already extracted into a minimal core. This article assumes Rust module familiarity and explains where decision code and effect code actually meet in the inspected sources. The [modularity inventory](../../modularity-boundaries.md) remains authoritative about the migration slice and its non-claims. Return to the [Technical companion](../README.md).

## Separate a fact from the operation that obtained it

A deterministic decision can consume an explicit freshness fact without reading a clock. It can consume an adapter-support fact without opening a transport. It can decide whether a destructive operation is admissible without deleting anything. The shell is responsible for obtaining those inputs under the appropriate authority and executing permitted effects afterward.

`crates/molten-core` holds in-memory decision cores. It has no filesystem, process, network or clock access, but it is not dependency-free: the inventory lists its crates.io and Git dependencies. Root-crate identity helpers, codecs, policy/evidence tooling, runtime orchestration, adapters, CLI code, and integration shells are different dependency classes. Calling all of them “the runtime” loses the distinction needed to review an import. In particular, Nickel policy authoring is not live runtime authority, and adapter availability is not trust.

## No generic planner stands in for the real boundary

Earlier slices carried a generic boolean effect planner and an import-fact validator in `molten-core`. No production shell called them, so they were removed. A planner that takes `has_authority: bool` from its caller does not show that any shell obtained authority. Review the production decision that the shell actually calls. The one surviving piece is the `EffectKind` vocabulary in [`molten_core::planning`](../../../crates/molten-core/src/planning.rs), which the world-replay boundary uses to classify adapter effect ports.

Import direction is enforced by tools that inspect real source. The [authority audit guide](../../ast-grep-runtime-authority-audits.md) describes ast-grep inventory rules and narrower blocking scopes for converted adapters. Octet's source gate runs over the compiled crate graph. The authored rules in `docs/dependency-boundary-policy/` are Nickel-checked policy data. No Rust code consumes them, and they do not by themselves block an import.

## Destructive admission as a worked example

Consider an illustrative retention request with authority, fresh evidence, allowed resources, and a complete reference index, but no remote GC clearance. [`admit_destructive_evidence_with_root`](../../../src/retention/parts/mod/p035/body.rs) validates the supplied evidence, then resolves each referenced admission and clearance record through the capability-scoped retention root. It reports `has_delete_authority` only when authority, policy, supporting evidence, the reference index when declared complete, and remote clearance refs all resolve. With clearance missing, the admission carries diagnostics and denies.

[`apply_gc_plan_with_root`](../../../src/retention/parts/mod/p014/body.rs) recomputes the stored plan and re-runs that admission before it writes an apply receipt or tombstone. A stale plan or missing clearance therefore produces a denied apply receipt, not a deletion. Presenting a receipt about a prior operation cannot itself grant authority: the admission reads referenced records, not the caller's assertion.

The useful invariant is narrow. The retention denial path writes no destructive effect, but it still writes a denied apply receipt. A denial is not the same as "no effects". Read each decision together with its own effect contract.

## Layered review rather than a single gate

A useful review follows the data in both directions. First identify the shell that acquires authority and obtains facts. Then inspect the decision and its denied cases. Finally identify the shell that interprets the result. This distinguishes two bugs that import checks alone cannot separate: an ambient operation moved into a supposedly pure module, and a shell that ignores a correctly returned denial.

Structural audit rules can catch prohibited syntax but do not prove the semantic adequacy of admission facts. Conversely, admission tests can exercise missing authority without proving that every filesystem call is capability-relative.

Suggested verification includes `cargo test -p molten-core`, `cargo test -p molten --lib retention`, and the affected adapter-rule fixtures. These are review instructions, not checks executed for this article.

## Limits

This account does not claim complete crate extraction, global absence of ambient effects in the root crate, effect execution atomicity, or end-to-end production safety. Canonical codec identity and public API compatibility remain separate obligations when boundaries move. The intended architecture is preserved by explicit inputs, bounded ownership, and scoped evidence—not by renaming a module “core.”

## Sources

- [Modularity boundary inventory](../../modularity-boundaries.md)
- [Runtime-authority audit guide](../../ast-grep-runtime-authority-audits.md)
- [Effect-port vocabulary](../../../crates/molten-core/src/planning.rs)
- [Retention destructive admission](../../../src/retention/parts/mod/p035/body.rs)
- [Technical companion](../README.md)
