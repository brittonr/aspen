# Component import admission

This article explains how Molten separates a component's declared interface, independently observed interface, and authority bindings before execution. It assumes familiarity with WIT worlds and content-addressed artifacts. The [runtime profile](../../wasm-component-runtime.md) remains authoritative; this is a [Technical companion](../README.md), not a proposal to add host interfaces.

## Three questions, not one allowlist

Import admission answers three different questions: which surface belongs to the reviewed world, whether the supplied component actually exposes that surface, and which authority would justify an admitted import. A declaration alone cannot answer the second question, and a content identity cannot answer the third. Keeping these checks separate prevents a manifest from becoming its own evidence.

The current cohort is deliberately narrower than the general grant data model. It admits no imports and no WASI interfaces. The [declared-surface implementation](../../../src/wasm/component/imports/surface.rs) fixes the world imports to an empty list and exports to `invoke`. Its `DeclaredManifest` binds schema, profile, WIT package, world, import names, export names, and a canonical manifest identity. `build_manifest` rejects malformed, unsorted, or duplicate entries before constructing that identity. Surface entries are bounded to 128 per collection; that construction limit is not permission to use 128 imports in the current world.

`admit_declared` compares the manifest with both `DeclaredWorld` and `DeclaredObservation`. It checks world and package agreement, complete import/export equality, extraction-tool and verifier identities, and the manifest's recomputed identity. WASI-prefixed surfaces are independently denied. The manifest therefore describes a complete surface, not a partial set of interfaces that may be supplemented from the host environment.

## Plan facts and executable observations

The [execution shell](../../../src/wasm/component/runtime/shell.rs) validates the profile, classifies the artifact header, verifies materialization, reinspects artifact resource facts, and admits the observed import surface before constructing an execution plan. Production bytes come from the complete Mantle materialization boundary described by the governing runtime document. Test-only loose bytes do not become production evidence merely because their interface is compatible.

The pure planner's observation is derived from `ComponentArtifactFacts`; it is not a filesystem read or an engine execution. `plan_component_execution` first admits the declared surface, then checks identity, enabled features, resources, and grants. The shell supplies the independent observations needed to connect those typed facts to actual bytes. This distinction matters when reviewing a caller: successfully constructing a plan from self-consistent facts is different from completing the shell's materialization and inspection path.

There is a further independent check during [runtime instantiation](../../../src/wasm/component/runtime.rs). `verify_runtime_shape` enumerates the engine's component imports and exports, sorts them, and compares them with the materialization facts. Enumeration takes at most the expected length plus one, which is sufficient to detect an extra entry without treating the declared count as unlimited work. The current runtime then rejects any nonempty import list and instantiates with an empty linker. A structurally plausible grant cannot manufacture a missing host implementation.

## Grants bind roles rather than ambient privilege

The [admission model](../../../src/wasm/component/admission.rs) defines `ComponentImportGrant` with an import name, capability, and policy, authority, resource, and recorded-effect references. Grant validation requires exactly one matching grant for each import, checks the evidence-reference shapes and nonblank capability, and rejects unused authority. The resulting plan keeps role-separated evidence collections and incorporates grant bindings into runtime-configuration identity.

These mechanics explain how imports are accounted for without claiming that arbitrary future imports work today. The currently reviewed world and empty linker are stronger restrictions than the generic grant representation. Changing only an allowlist or providing a capability string does not update the WIT cohort, observed surface, linker implementation, or external evidence. The governing document explicitly places any future import support behind a future reviewed profile.

## Illustrative failure: hidden WASI dependency

Suppose a producer publishes a component whose manifest lists no imports, while its bytes depend on a WASI interface. This is an illustrative hostile case, not a reported run. An unchanged empty manifest does not make the dependency disappear: observed-surface equality rejects the mismatch. If the producer instead adds the interface to the manifest, equality with the pinned empty world fails, and the WASI-prefix check also denies it. Supplying an authority grant cannot repair either disagreement. If a caller forged intermediate facts, runtime shape verification remains another boundary, but callers should not rely on reaching that late check.

Notice what this reasoning does not establish: it does not show that import-free guest code is behaviorally correct. It establishes that the declared execution profile cannot silently acquire the host's ambient interfaces.

## Review and verification guidance

Review a proposed integration in order: exact component/WIT cohort, complete manifest, independently observed surface, plan admission, and actual linker construction. Treat sorted uniqueness and unused-grant rejection as authority-accounting invariants, not presentation preferences. Trace receipt fields back to the independently derived plan rather than accepting a self-consistent receipt hash as sufficient context.

The existing [shell tests](../../../src/wasm/component/tests/shell.rs) exercise materialized execution, wrong-world denial, artifact-kind rejection, and production rejection of loose bytes. These are useful starting points for a targeted verification session; they were inspected, not executed for this documentation change. A review should also examine negative import cases in the import test directory when changing the surface contract.

## Limits and non-claims

Canonical receipts record bounded stages and evidence references, not hostcall purity, source-language equivalence, entitlement beyond cited authority, or release eligibility. Empty imports remove one route to ambient effects; they do not prove whole-system security. The shell performs compilation and execution effects, while deterministic admission operates on supplied facts.

## Sources

- [Runtime profile and evidence boundary](../../wasm-component-runtime.md)
- [Declared manifest and surface comparison](../../../src/wasm/component/imports/surface.rs)
- [Plan and grant admission](../../../src/wasm/component/admission.rs)
- [Runtime shape and empty linker](../../../src/wasm/component/runtime.rs)
- [Execution shell sequencing](../../../src/wasm/component/runtime/shell.rs)
- [Technical companion](../README.md)
