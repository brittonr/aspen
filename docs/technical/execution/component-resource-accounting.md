# Component resource accounting

Component resource accounting spans declarations, independent byte inspection, engine limits, and canonical execution evidence. This article explains what each layer measures and why none substitutes for the others. It assumes familiarity with Wasm memories, tables, and fuel. The [runtime profile](../../wasm-component-runtime.md) governs this [Technical companion](../README.md); it does not define a general operating-system resource quota.

## Resource facts have units and aggregation rules

`ComponentArtifactFacts` contains memory and table growth facts, instance/memory/table counts, imports, exports, identity fields, and enabled features. In [admission](../../../src/wasm/component/admission.rs), fixed growth means both `GrowthStrategy::Fixed` and `maximum == Some(initial)`. The initial value must fit the corresponding profile bound. Counts and collection lengths are checked independently: a small memory declaration does not compensate for too many memories or instances.

The [byte inspector](../../../src/wasm/component/admission/inspection.rs) clarifies an important accounting detail. Wasm memory declarations are expressed in pages; the inspector converts initial pages to bytes using checked multiplication by 65,536. Its memory growth fact records the maximum initial size encountered, not the sum of all memory sizes. The table growth fact similarly records the largest initial table, while separate counters record the number of memories and tables. Instance accounting adds both core-instance and component-instance section counts with overflow checks.

Accordingly, `max_memory_bytes` should not be described as a complete host-memory budget or a sum across all declarations. It bounds the represented per-memory size and is accompanied by a memory-count bound. Even multiplying those bounds would only give a rough upper bound on that class of declared guest memory; compilation data, runtime metadata, host payload buffers, and operating-system overhead are different quantities. The implementation does not turn that product into a measured resident-set guarantee.

## Declaration inspection is an independent boundary

The shell does not merely trust producer-supplied counts. `verify_component_artifact_facts` validates the admitted Wasm feature cohort and inspects the bytes, then compares observed growth facts and counts with the expected materialization facts. A discrepancy denies before instantiation.

Memory inspection rejects memory64, shared memory, custom page sizes, and any declaration whose maximum differs from its initial size. Table inspection rejects table64, shared tables, and nonfixed growth. Imported core memories and tables participate in inspection as well as locally declared ones. These are structural checks over the supplied component, not observations of an already-running workload.

There is a useful difference between “the facts are within bounds” and “the bytes have those facts.” The pure admission validator addresses the former. Independent parsing addresses the latter. A caller that understates a memory size cannot legitimately use pure validation as a substitute for byte inspection.

## Engine enforcement and canonical payloads

The [runtime](../../../src/wasm/component/runtime.rs) installs Wasmtime store limits for memory size, table elements, instances, memories, and tables, and enables traps on failed growth. It configures fuel consumption, maximum Wasm stack, NaN canonicalization, and a fixed reviewed feature selection. Host-size conversions for limits can deny rather than silently truncate a profile value.

Fuel is initialized when the store is created, before component instantiation. On successful invocation, remaining fuel is read from that store. It is therefore unsafe to label the difference between configured fuel and remaining fuel as wall-clock time or, without further qualification, as invocation-only work. Instantiation precedes invocation in the same fueled store. On an invocation error, zero remaining fuel selects `FuelExhausted`; other failures are classified separately as traps or guest denials.

The [execution shell](../../../src/wasm/component/runtime/shell.rs) canonicalizes the input Preserves value and checks the resulting byte length against `max_hostcall_bytes`. After invocation it checks output length against `max_result_bytes`, then parses canonical Preserves and hashes the value. The current cohort has no host imports, so the payload field name does not imply an active hostcall interface. The result-size check also occurs after the runtime has returned an output vector; it is an acceptance bound, not proof that no larger temporary host allocation occurred.

## Illustrative accounting failures

Suppose component bytes declare a fixed two-page memory, but accompanying facts claim one page. The illustrative values are 131,072 versus 65,536 bytes. Even if both are below the profile's permitted size, independent inspection rejects their disagreement. Raising a resource cap does not fix false materialization facts.

Now suppose both facts and bytes specify an initial page with a two-page maximum. This is not an admissible “small growth” exception: it violates fixed-growth inspection and admission. Finally, suppose an otherwise admitted component returns a short but noncanonical Preserves encoding. It fits the result byte bound yet still denies. Capacity, representation validity, and execution success are independent predicates.

Successful execution produces inspection, instantiation, and execution receipts linked by stage parents. Execution evidence binds canonical input/output identities and fuel facts. A post-instantiation fuel or output failure instead ends in a denial receipt. Resource exhaustion is therefore not converted into a successful result merely because earlier stages passed.

## Review and verification guidance

Review units, checked conversions, aggregation rules, and enforcement stage for every resource field. Ask separately whether an asserted bound concerns declarations, per-store engine limits, canonical payload acceptance, or total host resource use. Avoid translating one category into another in dashboards or release statements.

The existing [shell tests](../../../src/wasm/component/tests/shell.rs) cover forged resource facts, actual growth, malformed canonical output, and fuel exhaustion. They also inspect stage-specific denial evidence. These tests were read, not run for this article. A targeted validation session should preserve those distinctions rather than checking only a generic failure result.

## Limits and non-claims

Resource admission is not a performance guarantee, a proof of behavioral determinism for arbitrary hosts, or release eligibility. Fuel is not elapsed time; fixed guest declarations are not bounded total process memory. Deterministic admission and canonical receipts describe bounded facts under a specific cohort, while compilation and execution remain imperative-shell effects.

## Sources

- [Runtime resource and receipt contract](../../wasm-component-runtime.md)
- [Resource admission predicates](../../../src/wasm/component/admission.rs)
- [Nested declaration inspection](../../../src/wasm/component/admission/inspection.rs)
- [Engine and store limits](../../../src/wasm/component/runtime.rs)
- [Payload checks and receipt stages](../../../src/wasm/component/runtime/shell.rs)
- [Resource denial tests](../../../src/wasm/component/tests/shell.rs)
- [Technical companion](../README.md)
