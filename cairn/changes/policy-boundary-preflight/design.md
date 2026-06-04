## Context

The harness runtime already evaluates each step through a pure admission policy and records canonical admission-decision events. Validation now independently recomputes those decisions from the embedded suite policy. However, policy parsing itself is still implicit: if a suite has a policy fixture, the runner parses it and starts executing; if no fixture is present, the runner uses the allow-all default.

For Molten's intended security model, the policy boundary must be visible and receipt-backed before any runtime side effect. Nickel should own declarative static policy/config/schema validation, Basalt/UCAN should own capability-bearing admission context and gate receipts, and Steel should only appear as explicitly reviewed trusted callables. Until those engines are fully integrated, the harness can still enforce the shape of that boundary over the current Preserves deny-rule fixture.

## Goals

- Require explicit policy preflight evidence in evidence-bearing reports.
- Bind policy preflight evidence to a canonical policy snapshot ref derived from the embedded suite policy.
- Ensure the runner performs policy preflight before runtime turns or effect requests.
- Keep the current policy executable subset static, deterministic, and side-effect free.
- Fail closed for missing, malformed, stale, unsupported, or unreviewed policy boundary evidence.
- Preserve a clear upgrade path to Nickel static contracts, Basalt/UCAN context, and reviewed Steel predicates.

## Non-Goals

- Do not implement the full Nickel evaluator in this slice.
- Do not implement real Basalt/UCAN authorization contexts in this slice.
- Do not execute Steel predicates or serialize callable code in reports.
- Do not treat static policy denial as process failure by default; denials remain valid pass evidence when recorded and validated.
- Do not replace admission-decision evidence; policy preflight evidence is an additional prerequisite.

## Policy preflight evidence

The local harness emits a policy gate record before observations:

```preserves
<policy-gate-v1 "molten.harness.policy-gate.v1"
  <decision "pass">
  <policy-ref "blake3:...">
  <static-engine "preserves-deny-rules-v1">
  <nickel-contract "nickel-compatible-static-v1">
  <basalt-context "local-harness-preflight-v1">
  <steel-predicates []>
  <checks [
    <check "policy-schema" "pass">
    <check "canonical-policy-snapshot" "pass">
    <check "nickel-static-boundary" "pass">
    <check "basalt-preflight" "pass">
    <check "steel-predicate-review" "pass">
  ]>>
```

The `policy-ref` is the Blake3 canonical Preserves hash of the normalized policy snapshot. For this slice the snapshot is a canonical `<policy-v1 "molten.harness.policy.v1" [...deny rules...]>` value built from the parsed static fixture, including an empty deny-rule list for omitted policies.

## Validation rules

Report validation must:

1. Parse the embedded suite and derive the normalized policy snapshot.
2. Require exactly one policy gate record in the report field reserved for policy preflight evidence.
3. Verify the policy gate schema, decision, static engine marker, Nickel-compatible contract marker, Basalt preflight context, empty Steel predicate list, and required check names.
4. Verify the policy gate `policy-ref` equals the canonical ref of the embedded policy snapshot.
5. Reject any unreviewed `<steel-predicate ...>` or `<dynamic-predicate ...>` policy fixture record in the local harness parser.
6. Validate per-step admission evidence only after policy preflight evidence succeeds.

A stale report whose embedded policy changed without updating the policy gate ref must fail closed before the pass-evidence gate accepts it.

## Gate receipts

A successful pass-evidence gate receipt must include, in addition to existing report/admission/replay checks:

- `policy-preflight`
- `nickel-static-policy`
- `basalt-policy-gate`
- `steel-predicate-review`

Receipt artifact refs must include both the normalized policy snapshot ref and the policy gate evidence ref so downstream CI, release, admission, and upgrade gates can audit which static policy boundary was accepted.

## Future integration seam

When Nickel and Basalt are wired in, the record fields above can move from local marker values to real contract refs, Basalt/UCAN context refs, and reviewed Steel predicate receipt refs. The invariant remains unchanged: no side-effect-bearing runtime turn is accepted as evidence unless policy preflight evidence is present, canonical, bound to the suite, and validated before admission decisions and effect evidence are trusted.
