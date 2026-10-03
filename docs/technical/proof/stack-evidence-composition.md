# Stack Evidence Composition

Stack evidence composition checks whether heterogeneous evidence members use a reviewed vocabulary and preserve their limited claims. It does not collapse policy, capabilities, source gates, identity, lifecycle, and release into one universal permission. This article assumes the [Valence stack evidence adapter](../../valence-stack-evidence-adapter.md) contract and follows its pure in-memory implementation. The [Technical companion](../README.md) links related treatments of proof and authority boundaries.

## An envelope of distinct roles

The [stack implementation](../../../crates/molten-core/src/stack.rs) defines seven required `StackEvidenceRole` variants: Basalt, Ucan, Trellis, Octet, Valence, Cairn, and Mantle. Each `StackEvidenceMember` carries its role, schema, artifact reference, verification role, and non-claims. `StackEvidenceEnvelope` supplies the members as an explicit slice. There is no ambient discovery of missing members from disk or the network.

`validate_stack_evidence_envelope` iterates over the required roles. Missing roles and duplicate roles produce issues; each matching member is validated rather than choosing a duplicate as an authoritative winner. Basic member checks cover BLAKE3-reference syntax, supported schema shape, a nonempty verification role, an evidence-only non-claim, and a prohibited authority-overclaim phrase. A successful summary records member count and that the required roles were present.

This level is structural validation. A reference of the expected textual shape is not proof that the referenced artifact has been retrieved, that its bytes hash to that reference, or that its issuer has authority. The pure adapter receives neither artifact bytes nor a filesystem resolver. Artifact loading and any substantive subsystem validation stay outside this function.

## Adapter rows refine the vocabulary

`ValenceStackAdapterRow` links a Molten role and schema to a Valence role, Valence schema, verification role, and required non-claim. The default rows use `molten.stack-evidence.member.v1`, with role-specific Valence vocabulary. For example, Ucan maps to capability-admission evidence and Mantle maps to release-bundle evidence. These are descriptions of evidentiary purpose, not capability grants.

`validate_valence_stack_adapter` first incorporates envelope issues. It then requires one mapping row per required role and compares each row with the reviewed constants. A row with the wrong Valence role or schema does not become valid because its member reference is well formed. Members are checked against their row for schema, verification-role equality, and inclusion of the row's required non-claim as an exact list member.

This distinction is important: envelope-level validation is weaker than adapter validation. A nonempty verification-role string can satisfy the basic envelope check while disagreeing with the role required by the adapter. Likewise, basic evidence-only text checking does not replace the adapter's exact required-non-claim membership check. Neither mechanism is a general natural-language theorem prover; the code checks particular fields and strings.

Only when there are no accumulated issues does the adapter produce a `ValenceStackAdapterReport`. Its `supported_claim` explicitly restricts the result to role/schema/reference compatibility and evidence-only non-claim conformance. The [adapter documentation](../../valence-stack-evidence-adapter.md) states the same non-authority boundary and keeps filesystem access, clocks, networking, and process execution outside this core.

## Worked substitution failure

Consider an illustrative seven-member envelope whose references all have the correct shape. A packaging error copies the Basalt verification-role string into the Ucan member. The envelope may still satisfy the basic nonempty verification-role check. The adapter, however, compares that member with Ucan's expected capability-admission verification role and reports a mismatch. Seven members and seven plausible references were not enough: the error was in purpose binding.

Now suppose the role string is corrected, but two Mantle members and no Cairn member are supplied. Choosing the newest-looking Mantle reference would be an invented conflict-resolution policy. The actual validation reports duplicate Mantle and missing Cairn roles. The composition cannot substitute release evidence for lifecycle evidence merely because both are useful to a release review.

Finally, suppose all adapter checks succeed. A caller attempting a storage mutation still needs storage-specific authority and admission. The adapter report does not validate that action, its resource constraints, or its current subject. This is the same non-promotion principle found in the [proof workflow](../../proof-workflow.md): a higher-level evidence container cannot enlarge lower-level trust automatically.

## Verification and review guidance

A suggested review constructs a valid envelope from the reviewed rows, then changes one dimension at a time: omit a role, duplicate a role, alter a schema, replace a verification role, remove the exact required non-claim, or insert an overbroad claim. Inspect the typed issues, not a rendered success banner. These scenarios describe review work; this article reports no executed adapter test or runtime result.

For consumers, review both the mapping table and the underlying artifacts. Check that the selected table is the reviewed vocabulary, that evidence references refer to the intended subject and version, and that downstream gates separately validate substantive authority. Treat a report as compatibility evidence in a larger receipt graph, not as a canonical release receipt merely because it summarizes release-related members. No cryptographic verification of arbitrary referenced payloads, production readiness, deployment approval, or permission to bypass subsystem gates follows from adapter success.

## Sources

- [Valence stack evidence adapter](../../valence-stack-evidence-adapter.md)
- [Proof workflow](../../proof-workflow.md)
- [Replay coverage readiness](../../replay-coverage-readiness.md)
- [Stack roles, validators, mappings, and tests](../../../crates/molten-core/src/stack.rs)
- [Release profile validation](../../../src/prod/release/parts/profile/p000/body.rs)
- [Technical companion](../README.md)
