# Assembling candidate-scoped review inputs

Mode: How-to

## Goal and prerequisites

Prepare review inputs for one source candidate without turning fixture success, reference syntax, or operator assertions into release approval. You need a reviewed candidate reference, accessible underlying evidence, the selected profile tier, the expected generated export identity, and the actual generated export identity. Missing evidence is a review blocker, not an invitation to reuse a plausible-looking digest.

This procedure is source-checked and was not executed for this documentation batch. Follow the [production operator runbooks](../../production-operator-runbooks.md) for deployment operations and the [readiness companion](../../technical/proof/release-readiness-and-proof-scope.md) for proof-scope theory. This page concerns assembly and inspection, not node startup or promotion.

## 1. Decide the claim before selecting the tier

Write down the candidate, intended workload, platform, and requested decision. A development profile permits local-fixture use; a pilot review has bounded operational claims; release-tier validation requires stronger supplied inputs. Do not change the tier merely to obtain a pass for a release claim.

Separate at least three questions in your review notes:

- Are dependency representations consistent with their reviewed profile?
- Are the release-profile references structurally acceptable and the declared export references equal?
- Does the actual candidate have adequate, current evidence for the requested operation?

The dependency checker, profile validator, and candidate gate address different portions of those questions. None replaces all three.

## 2. Assemble evidence by role, not by filename

Collect the six release-profile evidence roles: source gate, policy, Octet, Cairn, stack provenance, and production profile. For each, record the realized reference, where the bytes can be inspected, the producing operation, candidate or subject binding, execution scope, decision, and caveats. This is a review worksheet, not a proposed new serialized schema.

Keep generated-export expectations independent of observations. The expected reference comes from the reviewed generation inputs; the actual reference comes from the export being reviewed. Copying one variable into both fields conceals stale generation rather than proving freshness.

Retain negative evidence beside positive evidence as required by the [proof workflow](../../proof-workflow.md). A rendered summary can direct you to the evidence, but it cannot establish an expected-deny decision or unchanged state by itself.

## 3. Validate only after resolving real inputs

The following guarded recipe uses no sample digest. `REVIEW_OUT` must name a new output file in an isolated review directory; shell variable guards do not authenticate references or guarantee path isolation. Verify those properties before running it.

Source-checked, not executed. The route is declared by [main aliases](../../../src/main.rs), the [included test command enum](../../../src/main/root/parts/command/p000/body.rs), and [gate arguments](../../../src/cli/evidence/gate/command.rs). The [gate implementation](../../../src/cli/evidence/gate/ops.rs) emits the resulting value before returning a decision-denial error.

```sh
molten test gate release-profile \
  --profile-id "${REVIEW_PROFILE_ID:?Set the reviewed profile identifier}" \
  --tier release \
  --candidate-ref "${CANDIDATE_REF:?Supply the realized candidate reference}" \
  --source-gate-ref "${SOURCE_GATE_REF:?Supply reviewed source-gate evidence}" \
  --policy-ref "${POLICY_REF:?Supply reviewed policy evidence}" \
  --octet-ref "${OCTET_REF:?Supply reviewed Octet evidence}" \
  --cairn-ref "${CAIRN_REF:?Supply reviewed Cairn evidence}" \
  --stack-provenance-ref "${STACK_REF:?Supply reviewed stack provenance}" \
  --production-profile-ref "${PROFILE_REF:?Supply the reviewed production profile}" \
  --expected-generated-export-ref "${EXPECTED_EXPORT_REF:?Supply the reviewed expectation}" \
  --actual-generated-export-ref "${ACTUAL_EXPORT_REF:?Supply the observed export identity}" \
  --stack-provenance-required \
  --accepted-valence-policy-hash "${VALENCE_POLICY_HASH:?Supply the reviewed policy hash}" \
  --out "${REVIEW_OUT:?Set a fresh isolated output file}"
```

Inspect the canonical Preserves artifact, its diagnostics, and its evidence-only caveat. Do not substitute the terminal's pass line for that inspection. Some malformed inputs return errors before a validation value is constructed, and output IO can fail; absence of an artifact is not a canonical deny receipt.

## 4. Check the candidate-binding matrix separately

The candidate gate uses `CandidateEvidenceBinding` with `artifact_ref` and `source_ref`. Its groups are Rust validation, nextest, Nix checks, Cairn validation, Octet, dogfood, release-bundle verification, promotion, export verification, and pilot decision. Follow each required group's artifact to its producer and compare its source binding to the candidate under review.

The [candidate implementation](../../../src/prod/parts/readiness/p002/body.rs) explicitly records that declared binding does not prove external artifact truth. A structurally accepted binding is not evidence that a benchmark ran, a VM booted, or a promotion was authorized. Keep unavailable scopes visible and do not replace them with fixture metadata.

## 5. Resolve a stale-candidate case safely

Suppose the candidate is B but the available nextest evidence names A. Preserve A's receipt; do not relabel its source reference as B. Determine which evidence must be regenerated for B under the governing workflow. If that execution is unavailable, record the missing B evidence and stop the corresponding readiness claim.

Likewise, a passing profile with equal export refs cannot cure an unrelated stale source-gate artifact. Inspect source-gate freshness and policy at its own boundary. The current profile helper checks supplied values without dereferencing all evidence. Its policy-hash placeholder helper is also narrower than a full lowercase-hex parser; reviewers must obtain the real accepted policy hash rather than infer authenticity from acceptance.

## Sources

- [Handbook](../README.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Proof workflow](../../proof-workflow.md)
- [Readiness and proof-scope companion](../../technical/proof/release-readiness-and-proof-scope.md)
- [Release-profile validator](../../../src/prod/release/parts/profile/p000/body.rs)
- [Candidate input types](../../../src/prod/parts/readiness/p000/body.rs)
- [Candidate-binding implementation](../../../src/prod/parts/readiness/p002/body.rs)
- [Conformance rails, not candidate approval](../../../flake.nix)
