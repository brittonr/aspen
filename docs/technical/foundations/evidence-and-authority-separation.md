# Evidence and Authority Separation

Evidence describes or supports a claim. Authority determines whether an actor may perform an operation. Molten connects these through explicit admission rather than treating a receipt, discovered artifact, or available adapter as a transferable grant. This article assumes familiarity with canonical references and explains how the inspected planning core preserves that distinction. It is a [Technical companion](../README.md); governing admission rules remain in the existing architecture and fabric documents.

## Why canonical evidence is not self-authorizing

Canonical encoding establishes a stable object for comparison and reference. It does not establish that every claim inside the object is true, that its issuer is trusted for the requested operation, or that its use is current. A receipt can accurately record a denial. A catalog can accurately list an artifact that the caller is not permitted to execute. A transport observation can accurately record receipt of bytes without admitting their requested effects.

The [modularity workflow](../../modularity-boundaries.md#evidence-policy-runtime-and-adapter-ownership) therefore separates four responsibilities: evidence parsing or verification, deterministic policy admission, runtime effect planning, and adapter execution. Evidence-only receipts do not themselves grant authority, provenance trust, transport trust, retention authority, execution permission, or replay trust. Those distinctions prevent one successful check from silently standing in for all remaining checks.

The [fabric's non-claims](../../distributed-system-fabric.md#non-claims) extend this reasoning to system-level conclusions. Port bindings, simulations, receipts, and reference matrices do not independently prove delivery, durable persistence, semantic correctness, consensus, or production readiness. The meaningful question is not “is there evidence?” but “which proposition does this evidence support, under which inputs and scope?”

## The planner makes conjunction visible

`AdmissionInputs` in the [planning implementation](../../../crates/molten-core/src/planning.rs) separates `has_authority`, `evidence_fresh`, `resource_allowed`, and `adapter_supported`. The common admission helper accumulates diagnostics for missing authority, stale evidence, denied resources, and unsupported adapter capability. An available adapter cannot compensate for absent authority; fresh evidence cannot compensate for denied resources.

This is a conjunction over caller-supplied facts, not a complete policy engine embedded in four booleans. The caller still has to establish those facts using the relevant verification and authority context. The benefit of the small pure boundary is that once facts are supplied, the planner does not consult ambient machine state to decide whether to proceed.

Another function, `plan_evidence_policy_runtime_flow`, accepts evidence verification, policy admission, planning availability, and adapter availability separately. It denies if the first three prerequisites fail. Otherwise it retains a receipt-only plan; when the adapter is unavailable, it adds a diagnostic rather than pretending an application effect occurred. This particular planner does not execute a worker or write service state. Reading its `Admit` decision as evidence that some unspecified external operation succeeded would exceed the function's contract.

## Worked scenario: catalog discovery without execution rights

Suppose an illustrative operator discovers an artifact in a local registry. Discovery succeeds, but the operation has neither authority nor provenance admission. In `plan_registry_discovery`, a present discovery input with absent trust prerequisites produces `BoundaryDecision::Deny` while retaining `RegistryRead` and `ReceiptWrite` effects. The diagnostic explicitly classifies registry discovery as evidence-only.

This branch is subtle. A deny decision does not universally mean that every effect vector is empty. Here the planner preserves read-only discovery and its evidence while refusing the stronger trust conclusion. A caller that checks only whether the effect vector is nonempty could misinterpret a denied discovery as general permission. A caller that throws away all denied-result evidence could also obscure the explanation operators need. Review must examine both the decision and the meaning of each effect kind.

Now imagine that the artifact includes a well-formed system-extension receipt. That still does not turn an application into a system extension. The [tier validator](../../../crates/molten-core/src/fabric/tier.rs) restricts requested authorities by tier and requires the declared system-extension evidence categories, including lifecycle admission. Artifact possession, operation names, and catalog presence are not alternate activation paths.

## Evidence granularity preserves scope

The [fabric evidence profile](../../distributed-system-fabric.md#evidence-granularity) selects canonical evidence at trust, lifecycle, semantic commit, checkpoint, failure, and operator-observation boundaries. Internal page reads, packets, polls, and cache lookups may instead be bounded aggregates or omitted under the reviewed profile. A diagnostic profile can expose more detail without becoming a production default by fallback.

Consequently, lack of a receipt for every internal operation is not automatically an evidence failure; completeness is relative to declared boundaries. Conversely, a dense diagnostic trace is not automatically stronger authorization evidence. More observations do not create a grant, and aggregating many narrow receipts does not remove their non-claims.

## Review and verification guidance

For every positive admission branch, ask which supplied fact carries authority, which evidence establishes freshness, and which independent resource and implementation checks remain. For every receipt, identify the claim, subject, boundary, and explicit non-claims before using it downstream. Inspect denied branches for allowed observational work rather than imposing an indiscriminate “no effects on deny” interpretation.

Suggested verification targets include missing authority with a supported adapter, stale evidence with authority present, discovery without trust, and incomplete extension admission. Existing planning and tier tests provide starting points; no test suite or live admission workflow was executed for this article.

## Limits and non-claims

These small planners expose decision structure but do not verify every upstream signature, token, export, or receipt. The article does not prove that all call sites supply truthful facts or all shells execute only admitted plans. Nor does canonical receipt production establish production readiness. It describes the inspected boundaries and the stronger conclusions they intentionally do not support.

## Sources

- [Architecture: authority and adapter ownership](../../architecture.md)
- [Fabric evidence granularity and non-claims](../../distributed-system-fabric.md)
- [Modularity evidence-policy-runtime workflow](../../modularity-boundaries.md)
- [Planning decisions, discovery, and tests](../../../crates/molten-core/src/planning.rs)
- [Extension-tier admission and tests](../../../crates/molten-core/src/fabric/tier.rs)
