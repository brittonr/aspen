# Ownership and Semantic Boundaries

Molten's boundaries are best understood as assignments of decision ownership, not merely a directory layout. This article assumes familiarity with the envelope spine and explains how to distinguish a pure law, an admitted effect, and a service's semantics during implementation review. The [architecture](../../architecture.md) and [fabric boundary](../../distributed-system-fabric.md) remain authoritative; this is a [Technical companion](../README.md), not an additional policy specification.

## Ownership is about what may decide

A primitive decides from explicit in-memory facts: whether an input is valid, which transition is available, which finite bound applies, or which effects may be planned. An adapter performs capability-rooted interaction with a substrate. A system extension owns an optional service protocol and its policies. An application composes admitted services. The distinction matters even when several roles currently reside in one root crate: co-location does not transfer authority between them.

The fabric's substitution test is particularly useful. Replacing a storage backend may change error reporting and transaction mechanics, but it should not silently change who can delete an object. Replacing a transport should not choose a different consistency policy. If replacing the adapter changes the semantic admission decision, the adapter probably owns more than mechanism. Conversely, a function requiring a live socket merely to determine whether supplied membership facts permit a transition has not isolated its pure decision boundary. These tests come directly from the [ownership law](../../distributed-system-fabric.md#ownership-law).

Workload neutrality is not the absence of semantics. It is the placement of semantics at the correct scope. A transactional service can define isolation; a log can define retention; a scheduler can define task ownership. Those policies do not become the global meaning of the underlying storage, timer, or communication primitive. Ordinary actor traffic likewise does not require a universal consensus path.

## A small executable boundary

The [planning core](../../../crates/molten-core/src/planning.rs) makes the distinction concrete. `AdmissionInputs` carries four facts: `has_authority`, `evidence_fresh`, `resource_allowed`, and `adapter_supported`. `plan_store_write` first rejects a malformed value and otherwise delegates to `plan_adapter_effects`, requesting `StoreWrite` and `ReceiptWrite`. The returned `EffectPlan` contains a decision, effect kinds, and diagnostics. It does not write a file or commit a database transaction.

This is intentionally a small decision surface. A boolean named `has_authority` is not itself a cryptographic verification procedure: the caller is responsible for deriving that fact through the relevant checked boundary. Equally, an admitted `StoreWrite` is not proof that persistence happened. Admission, execution, and evidence of execution are different stages, as described by the [modularity workflow](../../modularity-boundaries.md#evidence-policy-runtime-and-adapter-ownership).

The retention planner adds domain-specific prerequisites without moving deletion into the core. `plan_retention_gc` checks remote clearance, index completeness, and plan staleness alongside admission facts. Its positive result plans deletion and a receipt. Its negative result contains no destructive effect. The adapter's responsibility begins after this decision; it cannot treat a locally available delete API as substitute clearance.

## Worked failure scenario: a plugin requesting storage ownership

Consider an illustrative sandboxed plugin that labels an operation as a durable-state service and requests `DurableState` plus `ProtocolOwnership`. The operation's spelling is irrelevant to its tier. In the [tier validator](../../../crates/molten-core/src/fabric/tier.rs), both authorities require `SystemExtension`; the test `sandboxed_plugin_cannot_gain_system_authority_from_operation_shape` exercises this denial.

Changing the request's tier is not enough. A system-extension request also needs the six declared evidence categories: manifest, policy pass, provenance pass, explicit port bindings, resource grants, and lifecycle admission. The validator checks category presence, bounds, duplicates, and tier compatibility, then sorts accepted authority and evidence vectors. Its in-memory result is not the canonical encoding of those facts, nor does enumeration presence independently verify the underlying receipts.

This scenario exposes two common category errors. First, possessing code that can speak a storage-shaped protocol is not permission to own durable state. Second, installing a capable storage adapter cannot repair absent lifecycle admission. Both errors confuse availability with admission and allow an implementation detail to decide semantics.

## Review and verification guidance

For a proposed change, follow one operation through four questions:

1. Which explicit facts reach the pure decision, and who checked them?
2. Which effect kinds can appear on each decision branch?
3. Which shell actually performs those effects, and with which capabilities?
4. Which canonical evidence records the boundary without claiming more than occurred?

The existing planner and tier tests are useful review anchors, especially negative branches. Running their containing suites would be suggested implementation verification, not evidence executed for this documentation change. Review should also distinguish a denied destructive operation from permissible diagnostic evidence; not every denial means all observation must cease.

## Limits and source precision

This article does not establish full crate extraction, backend durability, service correctness, or production readiness. The modularity inventory describes an initial slice. Its statement that `molten-core` has only standard-library dependencies does not match the current [Cargo manifest](../../../crates/molten-core/Cargo.toml), which lists external dependencies. The pure ownership rule remains the architectural reference; a dependency-free implementation is not asserted here, and dependency purity cannot be concluded from package names alone.

## Sources

- [Architecture and layer map](../../architecture.md)
- [Distributed-system fabric ownership](../../distributed-system-fabric.md)
- [Modularity boundary inventory](../../modularity-boundaries.md)
- [Pure effect planning and tests](../../../crates/molten-core/src/planning.rs)
- [Extension tier validator and negative tests](../../../crates/molten-core/src/fabric/tier.rs)
- [Current core dependency manifest](../../../crates/molten-core/Cargo.toml)
