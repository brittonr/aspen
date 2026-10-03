# Effective Config and Source Traces

An effective configuration readback explains selected values and the supplied evidence of where they came from. It is not the subsystem that grants permission to use those values. This article assumes familiarity with configuration precedence and canonical content references; see the [Technical companion](../README.md) and the authoritative [proof workflow](../../proof-workflow.md).

## Explicit inputs instead of ambient discovery

`build_effective_config_readback` receives an `EffectiveConfigInput`: profile references, a vector of `ConfigSourceInput` records, release mode, and caller-supplied diagnostics. Each source names a field, string value, source class, optional source reference, override-admission flag, and caveats. The [builder](../../../src/project/effective/parts/config/p000/body.rs) does not discover environment variables or query a ledger merely because a source is labelled `environment` or `ledger`. Those names classify explicit inputs.

This distinction separates observation from resolution. A shell or caller can collect observations and present them to the pure builder, but the builder cannot prove that every relevant external source was collected. Nor does the generic string value encode the resource units or semantic constraints of every subsystem. A selected string representing a queue depth is not automatically a resource-admission result.

The implementation validates nonempty field/value text, recognized source classes, reference syntax, and collection bounds. Non-default sources without references contribute denial diagnostics. Some invalid inputs return an error directly; other problems are accumulated into a readback with `decision = deny`. Consumers therefore need to distinguish construction failure from successfully constructed denial evidence.

## Precedence and trace preservation

The [selection helper](../../../src/project/effective/parts/config/p001/body.rs) ranks source classes in this order:

| Higher to lower | Class |
| --- | --- |
| 1 | `cli-override` |
| 2 | `profile` |
| 3 | `environment` |
| 4 | `ledger` |
| 5 | `default` |

The selector scans each field's supplied sources, replacing the candidate when it encounters higher precedence. An equal-precedence candidate with a different value contributes a conflict diagnostic. The implementation does not promise a global pairwise conflict analysis across all lower-priority candidates; its comparison is with the currently selected candidate. Consequently, this is not a general commutative merge algebra, and input trace order should not be discarded casually.

A selected CLI override with `admitted_override = false` is denied. In release mode, selection of a default produces `fixture-default-in-release`. Neither condition is repaired by a higher-looking label elsewhere in the record. The resulting field retains its selected source metadata and every supplied trace. Caveats are merged from all sources using an ordered set, so a caveat on a losing candidate is not silently forgotten.

Fields are grouped in a `BTreeMap`, and diagnostics are sorted and deduplicated. These normalizations provide predictable portions of the output. However, trace sequences and profile-reference sequences remain represented in the artifact. Canonical serialization does not imply that arbitrary permutations of semantically similar input lists have identical fingerprints.

## Worked reasoning: equal values, changed provenance

Consider an illustrative field `node.id`. Initially a reviewed profile supplies `node:local`. Later an admitted CLI override supplies the same string, with a different source reference. The visible value has not changed, but the selected authority of configuration selection—not authorization authority—has changed from profile to override.

`diff_effective_config_readbacks` detects `changed-source:node.id` because it compares source class and source reference independently of value. The [existing regression case](../../../src/project/effective/parts/config/p001/body.rs) exercises this exact distinction. A review that compared only final value strings would miss it.

The diff is deliberately narrower than whole-artifact identity comparison. It examines field additions/removals, values, selected sources, and caveats. A change only to a losing trace can alter the canonical readback while producing no field-level diff diagnostic. Likewise, a diff's `deny` means differences were observed, not that a particular deployment has been authorized or prohibited by its own gate. Reviewers should read the artifact identity, field diff, and subsystem decision as separate facts.

## Admission and evidence boundaries

The [runtime-limit guidance](../../runtime-limit-profiles.md) defines budget admission under compiled hard caps, including unit relationships and widening-override rules. The effective-config builder does not replace that admission. Its boolean `admitted_override` is supplied evidence of an upstream decision, not an implementation of the resource gate.

`evaluate_readback_authorization_use` makes the non-authority boundary explicit: it always emits denial for using a readback as authorization. Supplying subsystem evidence references does not turn this helper into a subsystem verifier. Canonical identity binds the readback's content; it does not promote its evidence class.

## Verification and limits

Suggested verification, not executed here, includes the existing `effective_config` library and CLI checks listed in the proof workflow. Review same-value source replacement, unadmitted overrides, defaults in release mode, malformed source refs, and the difference between trace identity and field-level diff. The checked-in tests already cover several of these consumer-visible cases.

No claim is made that the builder discovers all machine configuration, validates every field's domain, proves reference freshness, or installs the selected values. Rendered explanations are diagnostic views; production decisions still need their own receipts and authority.

## Sources

- [Proof workflow and effective-config evidence](../../proof-workflow.md)
- [Runtime limit profiles](../../runtime-limit-profiles.md)
- [Nickel product boundary](../../nickel-toolchain.md)
- [Readback builder and authorization boundary](../../../src/project/effective/parts/config/p000/body.rs)
- [Selection, serialization, and regression tests](../../../src/project/effective/parts/config/p001/body.rs)
