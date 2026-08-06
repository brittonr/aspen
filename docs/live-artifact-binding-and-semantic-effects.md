# Live artifact binding and semantic effects

Molten consumes two reviewed producer revisions:

- the selected authentication and binding packages from `OnixResearch/onix-artifact` revision `c932138d880ddf4c2967f4c024b489b5c0022bf1`;
- `kamacite-core` from `OnixResearch/kamacite` revision `d76fe4abe543724d8fc0ac4b362187caf2e27622`.

Cargo, Nix, the Cargo and Nix locks, the release-dependency profile, and generated unit2nix plans must identify these revisions. Sibling path overrides are for explicit development only. They are not release evidence.

## Ownership

`artifact-binding-core` owns pure structural mechanics. It checks successor revisions, exact snapshot resolution, bounded graph reachability, stable pin paths, attribution shape, and conservative retirement classification.

Kamacite owns canonical semantic effect-operation descriptors, identities, directional compatibility artifacts, and role-preserving Valence projections.

Molten retains these responsibilities:

- artifact and dependency loading;
- compatibility and migration policy;
- authority, capability, provenance, resource, and lifecycle admission;
- atomic registry publication;
- runtime root collection and completeness claims;
- Preserves schemas and runtime receipts;
- deployment, retention, garbage collection, and deletion authority.

A shared plan is not proof that Molten published a binding. A semantic identity is not permission to run a handler.

## Functional core and shell

`crates/molten-core/src/live_binding/` is the pure core. It receives explicit, already loaded facts. It does not read files, inspect clocks, call a network, publish a registry revision, or delete content.

The shell must perform these steps in order:

1. Load the current binding and one immutable snapshot.
2. Load and verify the proposed target and dependency closure.
3. Check product compatibility and migration obligations.
4. Check authority, policy, provenance, resources, and lifecycle state.
5. Ask the pure core for a checked successor plan.
6. Publish one successor revision with Molten-owned atomic compare-and-swap.
7. Emit a Molten transition receipt for the observed publication result.

Any failure before publication leaves the old binding current. Rollback creates another checked successor revision that targets an older immutable artifact.

## One-snapshot resolution

Late binding is opt-in at one explicit unit boundary:

- request;
- turn;
- callback pass;
- job;
- protocol session.

The unit resolves one logical key from one supplied snapshot. It then pins the exact artifact and normalized dependency closure. Old work keeps its prior resolution after a cutover. New work can resolve the successor from a newer snapshot.

Nested late binding is denied unless the nested operation is explicit. The nested operation must produce separate replay and effect evidence.

## Root inventory and retirement

A retirement profile must account for all registered root classes:

- active execution and callbacks;
- sessions;
- tasks;
- durable values and cells;
- queues;
- timers;
- registries;
- effect handles;
- snapshots;
- rollback and retention pins;
- remote and evaluation-cache pins.

All roots, graph edges, generation attributions, and completeness observations bind to one immutable snapshot. An uninstrumented profile is incomplete. A missing root class, incomplete edge set, missing attribution, malformed shared attribution, contradictory attribution, or mixed snapshot cannot produce `Retired`.

A complete report can classify a generation as `Retired`, `Live`, `Incomplete`, or `Unknown`. Stable root-to-artifact paths explain live pins. Duplicate roots and edges are normalized. Cycles remain bounded diagnostics.

Retirement is an observation about supplied runtime reachability. It is separate from retention and garbage-collection policy. A retired generation can remain retained for rollback, legal hold, replay, evidence, or remote uncertainty. A retirement report never grants deletion authority.

## Semantic operation binding

Strict Molten effect profiles carry the exact Kamacite `effect-operation` identity through these surfaces:

- effect manifest;
- handler binding;
- effect handle;
- request and response;
- effect log;
- adapter import;
- remote execution;
- runtime receipt;
- replay identity and transcript;
- evaluation-cache key;
- job;
- upgrade check.

Exact equality is the default handler match. A display name, alias, path, or equal parameter shape cannot replace the semantic identity. A default-behavior change creates a different operation identity and invalidates strict replay, cache, job, remote-execution, and upgrade identities.

Legacy operation strings remain readable in version 1 artifacts. They cannot satisfy a strict semantic profile.

## Directional compatibility

A non-exact match requires both:

1. a Kamacite compatibility artifact for the exact source, target, direction, and context; and
2. Molten policy, capability, and provenance admission for that use.

Replay-only compatibility cannot authorize live host execution. Reverse substitution requires a separate compatibility artifact. Compatibility evidence does not prove broad semantic equivalence or handler correctness.

## Canonical artifacts

`src/live_binding_adoption.rs` constructs and parses bounded canonical Preserves envelopes for:

- binding records and snapshots;
- resolution and transition receipts;
- root inventories and generation attribution;
- retirement reports and deploy diagnostics;
- semantic operation bindings.

The corresponding schema markers are under `schemas/preserves-boundaries/`. The typed fixture authority is under `fixtures/semantic-operation/*.ncl`; JSON files are deterministic runtime projections.

Deploy diagnostics distinguish stale compare-and-swap state, incompatible targets, unreachable successors, semantic handler mismatch, incomplete root inventories, ambiguous attribution, and concrete live pin paths. Diagnostics do not mutate state.

## Validation and rollback

Focused validation covers producer source agreement, exact and stale transitions, old-work/new-work split, explicit nested resolution, complete and incomplete inventories, shared and exclusive attribution, cycles, deterministic pin paths, exact semantic handlers, directional compatibility, replay-only denial, operation-key drift, canonical Preserves round trips, and retention non-authority.

Dependency rollback restores both Cargo and Nix declarations to the last reviewed producer revisions, regenerates both locks and unit2nix plans with their owning tools, and preserves the rejected adoption evidence. Runtime rollback stops new late-bound resolution and publishes a checked successor to the prior immutable target. Existing work remains pinned throughout.
