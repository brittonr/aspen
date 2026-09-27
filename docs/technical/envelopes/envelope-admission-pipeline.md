# Envelope Admission Pipeline

“Admitted” is meaningful only when the checked boundary is named. This article separates representation, envelope construction, declared identity, schema, and runtime permission checks. It assumes the canonical Preserves model in the [architecture](../../architecture.md#core-envelope-spine). This [Technical companion](../README.md) describes inspected helpers, not a claim that every ingress path invokes one universal pipeline.

## Separate predicates instead of one success flag

A receiver can establish that input parses, that its bytes are canonical, that its envelope shape is acceptable, that its declared reference matches, and that an action is permitted. Those propositions are not interchangeable. The implementation exposes separate entry points precisely where a reviewer must avoid reading more into a result than the function computes.

At the DTO boundary, `Envelope::from_dto` rejects an unsupported envelope version, parses `subject_preserves` and `body_preserves`, constructs `RuntimeValue` instances, and delegates construction to `Envelope::new`. Construction limits each of `blob_refs`, `capabilities`, and `evidence_refs` to 256 items. `ActorId::parse` and `Capability::parse` check nonempty bounded strings, with limits of 256 and 512 bytes respectively. These are envelope-local token checks, not the stronger nominal-domain grammar described elsewhere. See the [envelope source](../../../src/runtime/envelope/mod.rs).

There is an important construction-path qualification. The local actor, capability, and evidence wrappers derive transparent Serde deserialization, whereas the shared `ContentRef` has a custom deserializer that calls its checked parser. `from_dto` does not rerun every wrapper constructor, and `validate_core` checks reference-list lengths before calculating the boundary; it does not revalidate every field or independently check the version. Therefore neither API should be described as comprehensive re-admission of arbitrarily assembled public struct fields. This article reports that inspected scope rather than inventing a stronger invariant.

## Representation and schema checks are distinct

For packed input, the shared rail supplies `strict_canonical_decode`: parse, re-encode, compare bytes. `validate_boundary_bytes` uses strict decoding and then validates the value against a supplied `BoundarySchemaSpec`. These are generic boundary facilities; their existence does not establish that `Envelope::from_dto`, a textual DTO conversion, invokes them.

The schema report also has a subtle result convention. A noncanonical input produces an error. A canonical input with a schema mismatch can produce an `Ok(BoundaryCodecReport)` whose `decision` is `deny` and whose diagnostics explain the mismatch. A caller that checks only whether the function returned `Ok` would confuse report construction with admission. The [schema-validation implementation](../../../src/preserves/parts/rail/p005/body.rs) makes this distinction explicit.

## Remote identity is not capability approval

`admit_remote_envelope` validates the declared reference's syntax, recomputes the envelope's canonical hash, compares the two, and returns the envelope reference plus its listed blob references. It does not fetch blobs, check their bytes, authenticate the sender, or consult runtime policy. The [bridge implementation](../../../src/runtime/bridge/mod.rs) also provides `verify_blob_reference` as a separate byte-integrity operation.

Runtime action permission lives in a different model. `AdmissionPolicy::decide_with_capabilities` first calls `CapabilityContext::authorize`; absent a matching grant, it denies. With a grant, it evaluates policy deny rules. Both grants and rules can constrain actor, action, target, and value. `AdmissionRequest::from_step` derives those facts from a runtime step. The [admission implementation](../../../src/runtime/admission/mod.rs) does not turn envelope capability strings into grants merely because those strings are present.

## Worked failure: valid object, forbidden send

Suppose, illustratively, an envelope declares a canonical body and the correct envelope reference. Its blob-reference strings are well-formed, and a receiver successfully recomputes its identity. The requested send nevertheless has no matching capability grant in the supplied runtime context.

The representation checks can pass while the action is denied. Adding a capability-looking string to the envelope changes the hashed value but does not, by itself, alter the context consulted by `decide_with_capabilities`. Alternatively, a matching grant can exist while a deny rule rejects the particular actor, target, or body. In neither case should a remote identity result be relabeled as permission evidence.

At the architectural level, pending actions undergo pure validation and policy admission before turn commit. Denied work rolls back rather than becoming an ambient effect. That [turn model](../../architecture.md#dataspace-layer-synitsam-inspired) is broader than the small helpers inspected here; determining a particular adapter's ordering requires following that adapter's call sites.

## Verification and review guidance

Review an ingress path as a sequence of named predicates. Locate its actual decoder, constructor, expected-reference comparison, schema selection, grant context, policy call, and effect-release boundary. Check decisions inside diagnostic reports, not merely successful report allocation. Confirm which inputs are trusted construction products and which can arrive through deserialization or public fields.

Suggested checks include an unsupported DTO version, an oversized reference list, a canonical but schema-invalid record, a mismatched declared envelope reference, a missing capability grant, and a matching grant overridden by a deny rule. Existing [envelope tests](../../../src/runtime/envelope/tests.rs) and bridge tests provide several relevant cases. They were inspected, not run for this article.

## Limits and non-claims

This is a reasoning decomposition, not a newly mandated orchestration API. It does not prove universal ingress coverage, end-to-end transport authentication, effect atomicity, or release eligibility. Diagnostic observations and identity results are not canonical authority grants. Any such conclusion needs the governing authority and adapter contracts, not the word “admission” in a helper name.

## Sources

- [Architecture and pure admission model](../../architecture.md)
- [Nominal reference admission guidance](../../nominal-authority-references.md#core-and-wire-boundary)
- [Envelope construction and validation](../../../src/runtime/envelope/mod.rs)
- [Remote envelope and blob identity checks](../../../src/runtime/bridge/mod.rs)
- [Runtime grant and policy evaluation](../../../src/runtime/admission/mod.rs)
- [Boundary codec reports](../../../src/preserves/parts/rail/p005/body.rs)
