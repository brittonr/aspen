# Facade and Core Navigation

Molten's public import paths, physical source paths, and semantic ownership boundaries are related but not identical. This article is a source-navigation guide for readers who already know the architecture and need to trace a claim to the implementing code without mistaking an alias for a separate subsystem. The [Technical companion](../README.md) provides the wider article index; the [modularity inventory](../../modularity-boundaries.md) remains the governing account of the intended boundary slice.

## Three maps are necessary

The API map answers which path a caller imports. The physical map answers which file is compiled into that module. The ownership map answers whether the code is a pure decision, codec, adapter, policy/evidence shell, or integration surface. None can safely substitute for the others.

For example, a module named `runtime` does not by its name prove the absence of IO. A module marked `#[doc(hidden)]` remains public if declared `pub`; hiding it from generated documentation is not a capability boundary. A compatibility alias changes the path through which items are exposed, not the authority attached to those items. The [architecture's layer map](../../architecture.md#layer-map) describes semantic roles that must still be checked against implementation.

The root [module declarations](../../../src/lib.rs) explicitly map many internal names to files with `#[path]`. `playback` maps to `deterministic/replay.rs`, `codec` to `preserves/rail.rs`, `engine` to `runtime/mod.rs`, and `blocks` to `chunk/store.rs`. Opening a guessed `src/playback.rs` would miss this structure. The reliable starting point is the declaration, followed by its explicit path.

## The compatibility façade

The root library includes a [façade body](../../../src/parts/lib/p000/body.rs) containing `compat_module!`. The macro creates a public module and re-exports the target's public items. Its invocations map `deterministic_replay` to `playback`, `preserves_rail` to `codec`, `runtime` to `engine`, and `chunk_store` to `blocks`, among others.

Thus `molten::deterministic_replay` and the internal root module `playback` lead to the same implementation surface, not competing replay engines. The old-looking domain alias is often exactly what existing source uses. A navigation search that insists API names must equal filenames can accidentally invent a second convention or overlook existing functionality.

The same façade exposes `core_api` by re-exporting `molten_core::*`. Its root `prelude` adds `MoltenError` and `Result` alongside `core_api::prelude::*`. The pure crate's own [entrypoint](../../../crates/molten-core/src/lib.rs) explicitly exports selected planning, codec, policy, dependency, and other types and functions through its prelude. This is a preferred import surface, not evidence that every historical root module has been physically extracted into that crate.

## Following included source without losing context

Some entrypoints are include lists rather than full semantic implementations. The [replay entrypoint](../../../src/deterministic/replay.rs), for example, includes numbered body files. These are compiled in the enclosing module's context. A declaration or helper may therefore live in a neighboring included file, and an ordinal filename conveys little about ownership.

The modularity inventory describes remaining ordinal shards as staged compatibility artifacts. A reviewer should follow the actual includes, search for the relevant symbol within that known subtree, and inspect callers or tests before making a behavioral claim. The absence of logic in a short entrypoint does not mean the module is a stub. Conversely, finding a pure helper in one shard does not establish purity of the entire enclosing module.

## Worked navigation example: a denied store-write plan

Suppose an illustrative review asks why `molten::prelude::plan_adapter_effects` returns no store-write effect when authority is absent. Begin at the façade: the root prelude re-exports the core prelude. In the core entrypoint, `plan_adapter_effects`, `AdmissionInputs`, `EffectKind`, and `BoundaryDecision` are exported from `planning`.

The [planning source](../../../crates/molten-core/src/planning.rs) then supplies the behavior: `plan_adapter_effects` computes admission diagnostics and returns the common deny result if any exist. Missing `has_authority` contributes a diagnostic. That common deny result has an empty effect vector. The façade body also contains a test calling the function through the preferred prelude with authority false and asserting denial without effects.

This path supports a precise claim about that planner and import surface. It does not support a claim that every denied plan is empty: `plan_registry_discovery` in the same source deliberately retains read and receipt effects on a trust-denied discovery. Navigation must finish at the specific branch, not merely at a suggestive module name or test title.

## Review guidance and inventory drift

Record the public import, declaration mapping, physical implementation, and semantic owner together when reviewing a boundary change. Inspect the actual Cargo manifest when discussing dependencies; an architectural layer label is not a dependency audit. Suggested checks include compiling preferred imports and exercising the relevant positive and negative planner branches. These are recommendations, not checks executed for this documentation change.

One source discrepancy is explicit: the [modularity inventory](../../modularity-boundaries.md#core-and-dependency-classes) calls `molten-core` standard-library-only, while its current [manifest](../../../crates/molten-core/Cargo.toml) lists external dependencies, including BLAKE3, Serde, Basalt, and pinned core crates. This article does not reconcile that historical statement by silently redefining “dependency.” The intended no-ambient-effects ownership boundary remains separate from the observed package dependency list.

## Limits and non-claims

This guide describes inspected mappings, not a guarantee that all public aliases are permanent or every module is fully extracted. It proposes no compatibility removal or new API. Successful import resolution does not establish canonical identity, authority, or production readiness; those require the relevant domain and admission checks.

## Sources

- [Architecture and layer map](../../architecture.md)
- [Modularity API inventory and ownership](../../modularity-boundaries.md)
- [Root module-to-file mappings](../../../src/lib.rs)
- [Compatibility aliases, core façade, and prelude tests](../../../src/parts/lib/p000/body.rs)
- [Core modules and prelude exports](../../../crates/molten-core/src/lib.rs)
- [Effect planning implementation](../../../crates/molten-core/src/planning.rs)
- [Replay include entrypoint](../../../src/deterministic/replay.rs)
- [Current core Cargo manifest](../../../crates/molten-core/Cargo.toml)
