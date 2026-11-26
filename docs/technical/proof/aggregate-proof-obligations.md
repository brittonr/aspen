# Aggregate Proof Obligations

A broad workflow claim becomes reviewable when its constituent obligations are explicit. Molten aggregate proof manifests package those obligations around a shared subject; layered manifests express permitted relationships between evidence roles. Neither replaces the gate that actually controls an operation. This article assumes the [proof workflow](../../proof-workflow.md) and explains the inspected composition checks and their boundaries. See the [Technical companion](../README.md) for adjacent proof topics.

## Decomposing a claim

The governing workflow names six obligation classes: `input-validation`, `canonicalization`, `admission`, `mutation-boundary`, `replay-determinism`, and `fail-closed-negative`. They separate questions often collapsed into “the workflow passed.” Parsing input says nothing by itself about policy admission; deterministic replay says nothing by itself about whether the original operation had authority.

`ProofObligationInput` records an identifier, class, subject reference, prerequisite references, receipt references, decision, requirement identifiers, optional coverage kind, and caveats. `AggregateProofInput` adds a manifest identifier, shared subject, required obligation identifiers, and the supplied obligations. The [builder](../../../src/testing/traceability/parts/p002/body.rs) derives diagnostics, sorts them, sorts obligations by identifier, creates the canonical value, and hashes that value. This makes obligation presentation independent of the supplied obligation ordering, without implying that every sequence-valued field is an unordered set.

The [diagnostic checks](../../../src/testing/traceability/parts/p004/body.rs) reject an empty obligation collection through a denial diagnostic, diagnose duplicate obligation identifiers, compare every obligation subject with the aggregate subject, check class-specific decisions, and diagnose required identifiers absent from the collection. Reference validation checks supplied content-reference syntax. It does not fetch prerequisite objects or independently establish their freshness and semantic validity.

## Negative evidence inside a passing aggregate

A negative obligation is expected to have decision `deny`; the other five classes expect `pass`. This rule is explicit in [obligation validation](../../../src/testing/traceability/parts/p005/body.rs). Thus a passing aggregate can contain denial evidence without contradiction: the aggregate claims that the required evidence has the expected outcome, not that every child operation was admitted.

Illustratively, consider a dispatch claim with one subject reference and three required obligations: input validation, mutation-boundary evidence, and fail-closed negative evidence. Valid input and an authorized mutation path use passing receipts. A stale authorization attempt contributes denial evidence with unchanged-state or no-mutation evidence supplied by the subsystem. The aggregate can be acceptable because denial is precisely the intended outcome of the third obligation.

If the negative obligation is instead copied from a different subject, the subject comparison diagnoses the mismatch even if its receipt reference is syntactically valid. If a reviewer adds replay to the required identifiers without supplying that obligation, `missing-child` prevents the aggregate from passing. However, the required set is supplied input: an omitted requirement cannot be discovered merely by comparing the supplied obligations with that same incomplete set. Reviewing decomposition remains part of the trusted boundary.

## Layering is a separate composition rule

Layered proof uses roles rather than obligation classes. The inspected role relation allows a gate to bind pure-core evidence; replay to bind pure-core or gate evidence; release to bind pure-core, gate, or replay evidence; and operator readback to bind any role. Pure-core layers bind no child role. Subject mismatches, missing child identifiers, unsupported role edges, duplicate layer identifiers, and pass promotion of operator readback receive diagnostics.

Denial propagates through the graph, as the [governing workflow](../../proof-workflow.md) requires. A layer is effectively denied when its own decision is not `pass` or when it transitively binds an effectively denied layer, and every `pass` layer that binds such a child receives a `denied-child-layer:<parent>:<child>` diagnostic, so a layered manifest cannot pass over a denied pure-core, gate, or replay layer. A layer that honestly reports `deny` over a denied child is not additionally diagnosed.

Cycle detection uses a three-colour depth-first search with an explicit active path. A child that is still on the active path closes a real cycle, and every layer on that cycle receives a `cycle:<layer>` diagnostic; a child that has already finished is a shared descendant and is not revisited. Acyclic diamonds, such as a release layer binding both a replay layer and the gate that replay also binds, therefore pass, while layers that merely lead into a cycle are not themselves reported as cyclic.

Coverage extraction fails closed as well: `coverage_from_aggregate_proof` returns a typed `aggregate-proof-denied` harness divergence, naming the manifest's diagnostics, whenever `manifest.decision` is not `pass` or the manifest carries any diagnostic, and only then extracts obligation coverage. Extracted coverage still does not replace review of the decomposition or the downstream gates that control an operation.

These discrepancies are tracked as defects by the Cairn change [`harden-traceability-proof-composition-and-static-audits`](../../../.cairn/changes/harden-traceability-proof-composition-and-static-audits/proposal.md).

## Verification and review guidance

The [aggregate and layer tests](../../../src/testing/traceability/parts/tests/p001/body.rs) demonstrate a positive aggregate, a missing required child, a permitted layer chain, readback pass rejection, an accepted diamond with shared descendants, cycle reporting limited to cycle members (including a self-loop), transitive denied-child propagation, and denied or forged-pass aggregate manifests yielding no coverage. Run them with `cargo test -p molten --lib trace_core`; inspect assertions rather than inferring coverage from a test name.

Review the required-obligation set against the actual workflow claim, follow prerequisite and receipt references, and keep subject equality separate from artifact authenticity. For negative mutation claims, follow state-preservation evidence rather than accepting a denial label alone. Aggregate manifests are evidence indexes with validation, not formal proofs, transitive trust engines, or authority tokens. Their canonical identity makes a particular composition referencable; it does not enlarge the claims of its children.

## Sources

- [Proof workflow](../../proof-workflow.md)
- [Replay coverage readiness](../../replay-coverage-readiness.md)
- [Aggregate and layer builders](../../../src/testing/traceability/parts/p002/body.rs)
- [Aggregate diagnostics](../../../src/testing/traceability/parts/p004/body.rs)
- [Obligation decisions and layered validation](../../../src/testing/traceability/parts/p005/body.rs)
- [Aggregate and layer regression tests](../../../src/testing/traceability/parts/tests/p001/body.rs)
- [Technical companion](../README.md)
