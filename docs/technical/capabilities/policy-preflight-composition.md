# Policy Preflight Composition

Policy preflight makes the material used by a decision explicit before execution; it does not collapse all admission obligations into one portable permission. This article follows the local harness's policy and capability preflight paths and explains how their evidence composes with request-time decisions. It assumes familiarity with canonical Preserves identity and the pure-core/effectful-shell split in the [architecture](../../architecture.md). The [Technical companion](../README.md) provides the broader reading map.

## Two different questions

Preflight asks whether the declared policy and authority material is acceptable for the harness boundary and whether its evidence is internally bound. Request admission asks whether a specific actor action is allowed under that material. Confusing them produces a common failure: an operator sees a passing preflight receipt and assumes every later request is authorized.

The runner makes their order visible. `run_suite_inner` calls `prepare_suite_run` before `collect_trace`. Preparation rejects missing explicit actor registries, capability fixtures, and budget fixtures; validates executor preflight inputs; checks the suite's step bound; and constructs policy, capability, and budget gate values. Only then does trace collection initialize runtime state and visit steps ([runner ordering](../../../src/harness/parts/runner/p000/body.rs)). This is an inspected local harness flow, not a claim about every process in the repository.

At the request layer, `AdmissionPolicy::decide_with_capabilities` first checks `CapabilityContext::authorize`, then evaluates policy deny rules. A grant match and absence of a deny rule are separate conditions. `AdmissionPolicy::allow_all` removes policy denials; it does not, when used through this combined entry point, manufacture a missing capability grant ([request admission](../../../src/runtime/admission/mod.rs)).

## The policy evidence chain

`policy_preflight_material` builds a canonical policy snapshot and computes its reference. It derives Nickel source from that policy, hashes the source as a Preserves string, evaluates a JSON export, and hashes the exported string. The Basalt contract envelope binds backend, contract identity and version, normalized source reference, input schema, output schema, and receipt schema. `validate_contract_envelope` supplies an acceptance result; rejection prevents successful preflight construction ([policy material](../../../src/harness/parts/schema/p019/body.rs)).

This yields a graph of named values rather than one unexplained success bit: policy snapshot, source, normalized export, contract envelope, and Basalt preflight result. The code records references connecting those values. Its source-evidence parser recomputes source and export hashes and reevaluates the source to compare the actual export with the recorded export. A matching source hash alone would not establish export normalization; both relations matter.

The Nickel operation is an actual interpreter call in the harness, through `nickel_lang::Context`, `eval_deep_for_export`, and `expr_to_json` ([normalization boundary](../../../src/harness/parts/schema/p020/body.rs)). It should not be described as ambient scripting inside `molten-core`. The architectural prohibition on such effects in the pure core remains authoritative.

## Capability evidence is independently bound

The capability gate binds a canonical capability snapshot, authority contract, Basalt authority preflight, and proofset evidence. Parsing checks agreement between the contract's normalized capability reference and the gate, the preflight's capability reference and the gate, the envelope references, and the proofset references. Validation against a suite additionally compares the embedded capabilities' hash and grant references, then reconstructs the expected gate and compares canonical hashes ([capability gate validation](../../../src/harness/parts/schema/p020/body.rs)).

Local fixture handling remains visible. When proofset-derived grant references are absent, parsing can use the authority preflight's grant references. Validation specifically checks local fixture derived grants when UCAN verification receipt references are empty. This is not evidence that a local fixture silently acquired external UCAN authority. The gate requires a `fixture-authority-evidence-only` check, and the [architecture](../../architecture.md) distinguishes fixture preflight from the broader authority integration boundary.

The [nominal reference contract](../../nominal-authority-references.md) contributes a different invariant: typed policy and evidence references cannot be accidentally substituted without explicit conversion. Exact typed references still do not prove that the referenced policy approves the request.

## Worked stale-evidence scenario

Imagine an illustrative suite whose policy snapshot P permits publication to a catalog. Preflight binds P, its generated source, export, and envelope. A maintainer changes the embedded policy to P2, adding a denial, but carries forward evidence from P. Each old object may remain individually well-formed and correctly content-addressed. The problem is the edges: old references no longer bind the actual suite material.

Likewise, replacing the suite's grants while retaining an old capability gate cannot be justified by showing that the old gate once passed. `validate_capability_gate_evidence` compares the current embedded capabilities and grants and recomputes the expected gate. This illustrates why canonical identities alone are insufficient: a valid object can be irrelevant to the current request or snapshot.

## Verification and review

Suggested review walks each binding edge in both construction and validation directions. Retain independent cases for changed policy material, changed grant material, missing preflight, and mismatched envelope/proofset references. At request time, distinguish missing authority from explicit policy denial; at the effect boundary, inspect whether denied execution is suppressed rather than merely annotated. Existing harness tests include missing and tampered preflight scenarios ([test source](../../../src/harness/parts/mod/tests/m000/p003/body.rs)); they are source evidence, not executions performed for this article.

## Limits and non-claims

A Basalt contract-envelope acceptance result is not a blanket proof of application correctness. Preflight is not a filesystem capability, transport identity, current delegation, or release approval. Actual filesystem containment still depends on typed root acquisition and capability-relative operations ([filesystem authority](../../local-filesystem-authority.md)). This article neither expands the local fixture's authority nor claims all runtime call sites use the same composition.

## Sources

- [Architecture](../../architecture.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Filesystem authority boundary](../../local-filesystem-authority.md)
- [Harness ordering](../../../src/harness/parts/runner/p000/body.rs)
- [Policy preflight material and normalization checks](../../../src/harness/parts/schema/p019/body.rs)
- [Capability gate and validation](../../../src/harness/parts/schema/p020/body.rs)
- [Request-time policy composition](../../../src/runtime/admission/mod.rs)
- [Authority preflight test cases](../../../src/harness/parts/mod/tests/m000/p003/body.rs)
