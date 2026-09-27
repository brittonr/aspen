# Effect Manifest and Handler Profiles

Effect configuration joins an artifact's declared possibilities to a concrete handler context before shell execution. This article examines that join and its limits, assuming familiarity with schema references and capability contexts. It accompanies the [Technical companion](../README.md); [effect manifest handler profiles](../../effect-manifest-profiles.md) remains the governing description.

## Declaration, support, and invocation are different questions

An effect manifest declares effect identifiers and operations, input/output schema references, resource classes, and required capability references. A handler profile carries binding, policy, capability-context, resource, and evidence references. Declaring an effect does not invoke it, and a handler capable of an operation does not thereby authorize every request for that operation.

The implementation exposes two distinct admission functions. [`admit_handler_profile_for_manifest`](../../../src/effects/parts/mod/p006/body.rs) evaluates whether supplied handler support covers the manifest under the supplied current context. [`admit_effect_request`](../../../src/effects/parts/mod/p002/body.rs) evaluates an individual request against the manifest and profile. Neither function performs a storage write or network send; they construct evidence for a shell boundary that remains responsible for honoring admission.

Profile admission parses the canonical manifest and profile values, validates supported-effect declarations and determinism/replay classifications, validates current context references and evidence references, then accumulates semantic diagnostics. It produces `handler-profile-admission-receipt-v1` with exact manifest/profile identities and the evaluated context. Parsing failure and an emitted denial receipt are different outcomes; a caller cannot treat “returned a receipt” as synonymous with “passed.”

## The support relation

The [matching implementation](../../../src/effects/parts/mod/p009/body.rs) requires each declared effect to find a supported effect with the same effect identifier and operation. It then compares input schema reference, output schema reference, resource class, and capability reference vector. A name match without these other matches receives a schema/resource/capability mismatch diagnostic.

This is stronger than string-based handler discovery but narrower than proving an implementation conforms to a schema. The support list is explicit input. The admission function does not execute the adapter to establish that it actually produces the declared output shape. Capability-vector comparison is direct vector equality here, not an independently established unordered-set equivalence.

The same diagnostics function checks profile policy equality against `current_policy_ref`, profile capability-context equality against `current_capability_context_ref`, nonempty binding references, nonempty resource references, and nonempty admission evidence. “Stale or revoked” in the capability mismatch diagnostic describes the context disagreement detected by this check; the inspected function does not itself query a revocation service.

The governing prose calls these resource bounds and current context. The local mechanism binds references and compares supplied values. That separation matters: independently establishing the truth and currency of those references remains outside this helper. The resource-profile rules in [runtime limit profiles](../../runtime-limit-profiles.md) are not reimplemented by checking that a vector is nonempty.

## Worked reasoning: admitted profile, denied request

Consider an illustrative artifact declaring `storage.write` with operation `write`, schema references I and O, resource class `hostcall`, and required capability C. A matching supported-effect entry and current profile context can produce a passing profile admission receipt.

A subsequent request can still omit C. Request admission first compares artifact identity and handler-profile name, then finds the declared effect/operation and checks membership of every required capability in the request's capability list. The omission therefore denies even though profile admission passed. The [existing test](../../../src/effects/parts/mod/tests/m000/p004/body.rs) exercises a `storage.write` request with an empty capability vector and checks denial.

Conversely, including C does not establish cryptographic authorization by itself. At this layer it satisfies a reference-membership condition. Capability proof verification, scoped handle use, and any live adapter obligations remain separate concerns. In particular, `admit_effect_request` receives the manifest, profile, request, and evidence references; it does not take a profile-admission receipt and re-run the whole profile gate. The caller's sequencing and evidence binding are significant parts of the overall boundary.

## Replay and cache identity

`bind_effect_profile_replay_evidence` binds subject, manifest, profile, and profile-admission references for replay, transcript, evaluation-cache, job-DAG, or remote-execution evidence. Where expected manifest/profile references are supplied, a mismatch denies unless a compatibility reference is present. Changing a handler while retaining the same artifact therefore need not be invisible to replay or cache review.

The mechanism has an important limit: in the inspected [drift predicate](../../../src/effects/parts/mod/p009/body.rs), compatibility is recognized by reference presence after syntax validation. It does not retrieve and prove the contents of compatibility evidence. If expected references are absent, that particular comparison is not made. The governing phrase “explicit compatibility evidence is bound” must not be expanded into a claim of automatic semantic equivalence checking.

## Verification and non-claims

Suggested verification, not executed here, is `nix develop -c cargo test actions::tests --lib`, as documented by the governing page. Review passing support, schema/resource/capability mismatch, stale supplied context, missing request capabilities, and drift without compatibility. Canonical receipts should precede rendered diagnostics in the [proof workflow](../../proof-workflow.md).

These mechanisms implement explicit declarations and in-memory admission evidence, not Unison syntax, typechecking, or runtime compatibility. They do not establish adapter correctness, exactly-once effects, full replay completeness, external freshness, or production readiness. No live effect or proposed command was executed for this article.

## Sources

- [Effect manifest handler profiles](../../effect-manifest-profiles.md)
- [Runtime limit profiles](../../runtime-limit-profiles.md)
- [Proof workflow](../../proof-workflow.md)
- [Profile admission and replay binding](../../../src/effects/parts/mod/p006/body.rs)
- [Support matching and drift diagnostics](../../../src/effects/parts/mod/p009/body.rs)
- [Request admission](../../../src/effects/parts/mod/p002/body.rs)
- [Profile and request regression cases](../../../src/effects/parts/mod/tests/m000/p004/body.rs)
