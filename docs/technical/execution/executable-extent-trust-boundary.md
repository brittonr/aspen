# Executable extent trust boundary

The executable-extent consumer checks one closed Mantle profile, independently remeasures its bytes, and conditionally maps them under current Molten-owned admission facts. It does not execute those bytes. This article assumes familiarity with content identities and W^X transitions. The [consumer contract](../../executable-extent-consumer.md) is authoritative; this [Technical companion](../README.md) explains the boundaries without extending the pilot's release status.

## Validity, identity, and authority are separate

The closed profile accepts a single 4096-byte, executable-read-only extent for little-endian `x86_64-linux-gnu`, using `mantle-flat-page-v1` with no relocations. These restrictions are not generic ELF loading support. The optional `executable-extents` feature keeps the pilot outside default release roots until a separate release decision selects it.

The [producer admission code](../../../src/executable_extent/producer/admission.rs) checks the bundle's schema and exact profile facts, its source and page sizes, member count, member offsets, permission, and identity links. It recomputes bundle identity from explicitly selected serialized material under the bundle identity domain. Producer-receipt validation checks linkage back to the bundle, publication observations, exact non-claims, receipt identity, and conformance evidence. A receipt with a valid-looking digest string but incorrect linked material is not sufficient.

`ExtentCodeRootProfile` keeps semantic code, built artifact bytes, extent manifest, producer receipt, runtime cohort, and policy identities distinct. In [mapping preparation](../../../src/executable_extent/orchestrator/mapping.rs), those identities come from different inputs: semantic code from the request, measured artifact and publication identities from admitted producer material, and runtime/policy from the explicit consumer profile. Collapsing them into one “code hash” would erase which question each identity answers.

## The shell remeasures rather than delegates belief

`consume_bundle` receives an application-owned `BundleSource` and `CurrentAdmissionPort`. The source port reads members relative to an authorized root; it is not permission to fetch arbitrary ambient paths. Manifest and producer-receipt reads are each capped at 65,536 bytes. Preparation reads every declared extent through the same source abstraction and checks its exact length and BLAKE3 identity before constructing `RemeasuredExtent` facts.

The closed profile's 4096-byte member is stricter than the shell's generic per-member safety ceiling. A broader internal byte cap does not widen the admitted format. Likewise, successful producer publication observations do not eliminate consumer-side remeasurement: a replaced member with unchanged metadata is caught by the actual content check.

The pure compatibility and activation logic receives typed facts, not an instruction to open files or map pages. Actual reads, Linux materialization, and teardown belong to the shell. This separation preserves a deterministic law over supplied evidence without moving ambient effects into `molten-core`.

## Current admission and an ordering nuance

`CurrentAdmissionPort::observe` supplies current artifact, runtime, resource, policy, and execution facts. An unavailable observation is an error, not an optimistic default. A conclusive activation denial yields an `inert` consumer receipt with no mapping observations. Valid immutable bytes can therefore remain inert because current policy or authority differs from the context in which they were produced.

There is a narrowly scoped ordering difference worth retaining during review. The [governing admission sequence](../../executable-extent-consumer.md) lists pure compatibility/W^X admission before asking for current facts. The inspected [orchestrator](../../../src/executable_extent/orchestrator/mod.rs) prepares remeasured facts, calls `admission.observe`, and then invokes `admit_code_profile` with the returned activation facts. This article does not claim a separate pre-observation invocation of that pure admission function. Both sources place mapping after admission, but the exact observable ordering of the current-facts port should be reviewed against these sources rather than inferred from the prose list.

## Mapping is not execution, and mapping is not completion

For admitted activation, `map_one` materializes and seals bytes, maps the extent through `executable-extent-linux`, and checks executable-read-only state and content identity. The resulting `MappedBundle` owns live mappings. The API does not call the mapped bytes.

Receipt completion is explicit. `MappedBundle::complete` consumes the owner, unmaps each mapping, checks that the final state is `Unmapped`, and only then builds the detached `mapped-and-unmapped` consumer receipt. Thus a successful `ConsumeOutcome::Mapped` is not yet the completed teardown receipt described by the end-to-end contract. A mapping or unmap error is returned as an error, not presented as a completed lifecycle observation.

These mutable mapping observations are detached from immutable world content. Binding the code-root profile into a world artifact root does not freeze current authority, preserve a live mapping, or turn a past teardown observation into a future execution entitlement.

## Illustrative hostile and stale cases

Imagine an intact manifest and producer receipt accompanied by a member whose final byte changed. This illustrative substitution fails extent remeasurement before current admission can authorize mapping. Repairing the member restores only structural/content eligibility. If the current-facts port then reports execution unauthorized, the outcome remains inert with no mapping observations.

A second illustrative mistake is to treat the ordinary artifact path as fallback evidence when extent admission fails. The governing contract explicitly rejects that interpretation when policy requires extents. Rolling back to ordinary artifacts removes the stronger profile and its claims; it does not relabel weaker evidence as successful extent admission.

## Review guidance and limits

Suggested verification should independently exercise changed member bytes, stale producer linkage, unavailable current observations, conclusive activation denial, and explicit map/unmap completion. Inspect dispositions and mapping observations rather than merely successful JSON decoding. No mapping or execution command was run for this documentation change.

Neither W^X admission nor a consumer receipt proves compiler correctness, code semantics, sandbox integrity, external authority freshness, storage authority, or release eligibility. The inspected shell establishes bounded materialization and lifecycle facts for the closed profile. It is not a general loader, a native execution permission oracle, or evidence that the mapped program behaved correctly.

## Sources

- [Executable-extent consumer contract](../../executable-extent-consumer.md)
- [Producer identity and publication admission](../../../src/executable_extent/producer/admission.rs)
- [Application-owned source and current-facts ports](../../../src/executable_extent/ports.rs)
- [Consumer orchestration and inert outcomes](../../../src/executable_extent/orchestrator/mod.rs)
- [Independent remeasurement, mapping, and teardown](../../../src/executable_extent/orchestrator/mapping.rs)
- [Technical companion](../README.md)
