# Content-store profile equivalence

Content-store profiles can share canonical content identity without sharing capabilities, effects, or operational guarantees. This article explains that restricted meaning of equivalence using the [content-store adapter contract](../../content-store-adapter.md) and the inspected pure preflight and transition code. Familiarity with ordered chunk manifests is assumed. It belongs to the [Technical companion](../README.md), not to a new backend conformance specification.

## Identity is the common boundary

The governing contract keeps manifest refs, ordered chunk refs, chunk lengths, fixed chunker parameters, transforms, and associated canonical refs independent of backend selection. A blob hash, endpoint identifier, ticket, object key, or local path is a locator or protection hint, not a replacement for Molten content identity. Thus two profiles may address the same canonical manifest while using unrelated retrieval machinery.

The strongest justified commonality is the admission and verification boundary, not identical backend behavior. Capability-local, Redb-indexed, live Iroh Blobs, and deterministic simulation profiles have different effects. A simulated disconnect is an input to a model; a live disconnect is an observed transport event. Both can feed the same deterministic transition rules, but the simulation does not establish live network readiness.

Local effects also remain subordinate to the [typed local-root boundary](../../local-filesystem-authority.md). A content command does not carry an ambient file path or database handle into the pure core. The shell acquires or receives the actual effect authority and supplies observations.

## Preflight defines the comparable operation

In [preflight implementation](../../../crates/molten-core/src/content_store_adapter/preflight.rs), `preflight_content_operation` validates profile, manifest, command shape, binding, and resources before returning accepted, denied, or cancelled. It compares `command.adapter_ref` with `profile.profile_ref`, compares manifest refs, checks required capabilities, and checks all manifest transforms against the supported set.

Bounds include total bytes, chunk count and size, concurrent operations, queued bytes, memory, logical deadlines, retry count, and requested range. Checked arithmetic makes overflow a distinct admission issue rather than an accidental budget bypass. A profile lacking a requested capability is not equivalent to one that implements it simply because both can retrieve ordinary chunks.

Range planning selects every chunk whose byte interval overlaps the requested interval. It checks nonzero range length, checked end-offset arithmetic, and containment within total manifest length. The returned refs retain occurrence order. Range support therefore describes verified content selection, not permission to expose an arbitrary unverified backend byte slice.

## Verification is ordered progress

The [transition implementation](../../../crates/molten-core/src/content_store_adapter/transition.rs) separates observation from verified state. `begin_partial_state` validates retained state and checks generation and operation binding before reconstructing the verified-byte count and missing suffix. `apply_chunk_observation` validates state, operation and manifest bindings, sequence, expected next ref, descriptor position, observed length, observed content ref, and event limits before advancing.

This core does not hash filesystem or network bytes itself. The contract assigns the shell the computation of the existing domain-separated chunk identity, followed by submission of an observation. A successful backend callback alone cannot satisfy that relationship. Shell honesty and correct measurement remain part of the adapter review, even when the in-memory transition is deterministic.

A completed sequence becomes `Verified`; the separate `mark_content_durable` transition requires that state and the `DurableCompletion` capability. Its deterministic check is not independent evidence that a shell performed durable storage. The distinction prevents conflating content verification with persistence guarantees.

## Illustrative duplicate and interruption example

Consider an illustrative manifest containing ordered occurrences `[A, A, B]`, with each occurrence four bytes long. After verifying the first `A`, the prefix is `[A]`, the missing suffix is `[A, B]`, and verified bytes equal four. A set-based representation would incorrectly erase the second occurrence and could overstate completion. The implementation retains vectors, reconstructs the suffix by prefix length, and has a [repeated-ref regression case](../../../crates/molten-core/src/content_store_adapter/tests.rs).

A range starting at byte three with length three overlaps both occurrences of `A`; the range planner returns both, not one deduplicated identity. Identity reuse saves no logical position in the manifest.

If transport then disconnects after the first verified occurrence, `classify_content_failure` yields `Uncertain`. With no verified prefix, its disconnect classification is `Retryable`; timeout is `Uncertain`. This is more precise than saying every disconnect has the same meaning. Retrying or resuming does not imply exactly-once transport or effect execution. The retained state describes verified progress, not a universal history of remote side effects.

## Review guidance and non-claims

Suggested cross-profile review compares the same manifest and supported command, verifies rejection before unsupported I/O, and checks corruption, truncation, duplicate occurrences, reordering, cancellation, and restart bindings. Keep profile identity in evidence rather than declaring all adapters interchangeable. These are suggested exercises; this article reports no execution.

For live claims, the governing document explicitly distinguishes `publish_live_iroh_chunks` and `execute_live_iroh_stream_get` from older local-copy compatibility helpers. Compatibility helper success is not live transport evidence.

Availability is not authorization. Backend protection effects do not grant read, reveal, retention, or deletion authority. The pure read and deletion admission functions inspect their separate authority records; neither profile equivalence nor a verified chunk upgrades those records. This article claims no uniform durability, confidentiality, provenance, performance, remote trust, or release readiness across profiles.

## Sources

- [Content-store adapter runtime](../../content-store-adapter.md)
- [Local filesystem authority](../../local-filesystem-authority.md)
- [Pure preflight and range planning](../../../crates/molten-core/src/content_store_adapter/preflight.rs)
- [Partial state, verification, failure, and authority transitions](../../../crates/molten-core/src/content_store_adapter/transition.rs)
- [Core content adapter regression cases](../../../crates/molten-core/src/content_store_adapter/tests.rs)
- [Technical companion](../README.md)
