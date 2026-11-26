# Evidence and Authority Separation

Evidence describes or supports a claim. Authority determines whether an actor may perform an operation. Molten connects these through explicit admission rather than treating a receipt, discovered artifact, or available adapter as a transferable grant. This article assumes familiarity with canonical references and explains how inspected admission code preserves that distinction. It is a [Technical companion](../README.md); governing admission rules remain in the existing architecture and fabric documents.

## Why canonical evidence is not self-authorizing

Canonical encoding establishes a stable object for comparison and reference. It does not establish that every claim inside the object is true, that its issuer is trusted for the requested operation, or that its use is current. A receipt can accurately record a denial. A catalog can accurately list an artifact that the caller is not permitted to execute. A transport observation can accurately record receipt of bytes without admitting their requested effects.

The [modularity workflow](../../modularity-boundaries.md#evidence-policy-runtime-and-adapter-ownership) therefore separates four responsibilities: evidence parsing or verification, deterministic policy admission, runtime effect planning, and adapter execution. Evidence-only receipts do not themselves grant authority, provenance trust, transport trust, retention authority, execution permission, or replay trust. Those distinctions prevent one successful check from silently standing in for all remaining checks.

The [fabric's non-claims](../../distributed-system-fabric.md#non-claims) extend this reasoning to system-level conclusions. Port bindings, simulations, receipts, and reference matrices do not independently prove delivery, durable persistence, semantic correctness, consensus, or production readiness. The meaningful question is not “is there evidence?” but “which proposition does this evidence support, under which inputs and scope?”

## Admission makes conjunction visible

Retention's [destructive admission](../../../src/retention/parts/mod/p035/body.rs) shows the pattern. `admit_destructive_evidence_with_root` validates the supplied evidence, then resolves each referenced authority, policy, supporting-evidence, reference-index, and remote-clearance record through the capability-scoped retention root. It reports `has_delete_authority` only when the action is destructive and every one of those resolves. Fresh evidence cannot stand in for missing clearance, and an available store cannot stand in for absent authority. Any unresolved part adds a diagnostic and the decision becomes `deny`.

This is a conjunction over records the admission reads itself, not over flags the caller asserts. The caller still chooses which refs to present, and a referenced record is only as trustworthy as the store that holds it. Once the refs are supplied, the decision does not consult ambient machine state.

## Worked scenario: a receipt is not an activation path

Suppose an artifact includes a well-formed system-extension receipt. That still does not turn an application into a system extension. The [tier validator](../../../crates/molten-core/src/fabric/tier.rs) restricts requested authorities by tier and requires the declared system-extension evidence categories, including lifecycle admission. Artifact possession, operation names, and catalog presence are not alternate activation paths.

A deny decision does not always mean that nothing was written. A denied retention apply still writes its denied apply receipt so operators can see why deletion did not happen. Review must examine both the decision and the meaning of each recorded effect.

## Evidence granularity preserves scope

The [fabric evidence profile](../../distributed-system-fabric.md#evidence-granularity) selects canonical evidence at trust, lifecycle, semantic commit, checkpoint, failure, and operator-observation boundaries. Internal page reads, packets, polls, and cache lookups may instead be bounded aggregates or omitted under the reviewed profile. A diagnostic profile can expose more detail without becoming a production default by fallback.

Consequently, lack of a receipt for every internal operation is not automatically an evidence failure; completeness is relative to declared boundaries. Conversely, a dense diagnostic trace is not automatically stronger authorization evidence. More observations do not create a grant, and aggregating many narrow receipts does not remove their non-claims.

## Review and verification guidance

For every positive admission branch, ask which referenced record carries authority, which evidence establishes freshness, and which independent resource and implementation checks remain. For every receipt, identify the claim, subject, boundary, and explicit non-claims before using it downstream. Inspect denied branches for allowed observational work rather than imposing an indiscriminate “no effects on deny” interpretation.

Suggested verification targets include missing authority with a supported adapter, missing remote clearance with authority present, and incomplete extension admission. The retention and tier tests provide starting points; no test suite or live admission workflow was executed for this article.

## Limits and non-claims

These admission decisions expose decision structure but do not verify every upstream signature, token, export, or receipt. The article does not prove that all call sites supply truthful refs or all shells execute only admitted decisions. Nor does canonical receipt production establish production readiness. It describes the inspected boundaries and the stronger conclusions they intentionally do not support.

## Sources

- [Architecture: authority and adapter ownership](../../architecture.md)
- [Fabric evidence granularity and non-claims](../../distributed-system-fabric.md)
- [Modularity evidence-policy-runtime workflow](../../modularity-boundaries.md)
- [Retention destructive admission](../../../src/retention/parts/mod/p035/body.rs)
- [Extension-tier admission and tests](../../../crates/molten-core/src/fabric/tier.rs)
