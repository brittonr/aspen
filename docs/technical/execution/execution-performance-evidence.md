# Execution performance evidence

Molten's component performance rail separates host measurements from deterministic comparison and canonical evidence. This article explains how to interpret a benchmark result without turning it into authority, semantic equivalence, or a general runtime ranking. It assumes familiarity with confidence intervals and content-addressed materialization. The [performance contract](../../wasm-component-performance.md) governs this [Technical companion](../README.md); component correctness remains under the separate [runtime contract](../../wasm-component-runtime.md).

## First establish what was measured

The rail pins a Sightglass source revision and raw measurement schema. Compilation, instantiation, and execution remain separate phases, and fast/deep lanes have distinct bounded process and iteration plans. A run identifies the suite, workload, host class, engine cohort, resource envelope, and actual Mantle materializations. Reviewed template-fixture references are not substitutes for measured artifact references in an executable suite instance.

Sightglass's stock benchmark API is core-module oriented. A wrapper that embeds a Molten component is consequently an input artifact with its own Mantle bundle and engine-cohort identity, not a guest that Molten silently builds during measurement. The governing document records an earlier diagnostic CLI build and help-surface smoke check, but explicitly excludes that local diagnostic build from production receipts. This article reports no newly executed benchmark or performance result.

The [runner's file admission](../../../src/wasm/performance/parts/runner/p001/body.rs) opens each runner, engine, and benchmark leaf with no-follow semantics, requires a bounded regular file, reads and rehashes its contents, checks read-only permissions, rewinds, and retains the handle. Linux process locators use `/proc/<pid>/fd/<fd>` for those same handles. Unsupported hosts deny rather than selecting a path-based fallback. This avoids replacing measured handles with later pathname resolution; it is not a claim of general host integrity or protection against every privileged mutation.

The outer shell owns process fan-out and aggregate runtime/output bounds. Raw host process identifiers are normalized into bounded suite-local ordinals. Original paths, stderr, and operating-system process IDs remain diagnostics, not canonical run identity. This distinction permits canonical evidence to describe the admitted measurement rather than accidental host naming.

## Comparability is a predicate before arithmetic

The [comparison core](../../../src/wasm/performance/parts/comparison/p001/body.rs) checks exact agreement on suite, benchmark, source component, component profile, performance profile, engine cohort, engine artifact, runner artifact, runtime configuration, target, host class, measurement mechanism, and resource envelope. Consumer class, recorded effects, and phase/event keys also have to match.

A candidate may have different optimized output bytes while retaining the same source component identity. That allowance is not general permission to compare arbitrary workloads. Conversely, two runs displaying the same workload name are not comparable if one changed the runner artifact or resource envelope. Incompatible inputs are reported rather than ranked. This protects interpretation before any plausible-looking percentage is computed.

## Deterministic classification over supplied samples

Sample counts use bounded integers. The implementation accumulates checked sums, computes a scaled mean, derives sample variance with denominator `n - 1`, divides by sample count for variance of the mean, and uses integer square root plus the fixed normal-approximation multiplier for confidence half-width. At least two samples are required; a zero baseline mean denies ratio construction.

Let `b` and `c` denote the internally scaled baseline and candidate means, `h_b` and `h_c` their half-widths, and `d` the practical delta derived from the baseline mean and reviewed threshold. Improvement requires the candidate's upper endpoint plus `d` to be strictly below the baseline's lower endpoint. Regression requires the baseline's upper endpoint plus `d` to be strictly below the candidate's lower endpoint. Otherwise the class is `NoSignificantChange`. Lower endpoints saturate at zero, and arithmetic checks deny overflow rather than wrapping into an attractive result.

These rules are deterministic computations over recorded samples. They do not establish sample independence, a representative host population, or cross-machine reproducibility. In particular, `NoSignificantChange` is the classifier's residual category, not a proof of equal performance.

## Illustrative comparison reasoning

Consider already-derived, compatible interval summaries in common scaled units: baseline mean 1000 with half-width 10, candidate mean 900 with half-width 10, and practical delta 50. This is an illustrative calculation, not actual measurement output or a claim about the configured threshold. The candidate upper endpoint plus delta is `910 + 50 = 960`, below the baseline lower endpoint 990, so the rule classifies improvement.

If the candidate mean were 960 with the same half-width, `970 + 50 = 1020` would no longer lie below 990. A smaller point estimate alone would not meet the improvement condition. If the host class changed, neither calculation would be eligible in the first place: compatibility fails before classification. That last rejection is more important than refining the percentage difference between incomparable observations.

## Evidence and optimization boundaries

Canonical performance receipts carry phase samples and bind run, optional comparison, optimization, materialization, conformance, and recorded-effect references. As the governing contract explains, comparison validation includes the exact peer run and recomputes the comparison from both raw sample sets. A self-consistent invented label or a receipt without its peer is insufficient; contextual validation also compares against independently derived run facts.

Portable, Wizer-transformed, and precompiled artifacts all cross the exact materialization boundary. The performance shell does not run Wizer or compilation commands to fill missing inputs. Admission of a precompiled token does not itself activate unsafe deserialization. Optimization conformance and capacity caps are separate gates, not consequences of a favorable benchmark result.

## Verification guidance and non-claims

The existing [comparison tests](../../../src/wasm/performance/tests/comparison.rs) cover deterministic classes, incompatible host/runtime facts, stale suites, and insufficient samples. They were inspected, not executed here. Suggested review starts with exact materialization and comparability, then checks phase grids and arithmetic, then receipt recomputation. Operator summaries should remain explicitly non-normative.

Performance evidence is recorded-only. It does not prove security, behavioral correctness, authority, semantic equivalence, release eligibility, or cross-runtime superiority. Keeping measurement, deterministic calculation, and contextual evidence separate makes a result useful without making it say more than the admitted experiment supports.

## Sources

- [Performance evidence contract](../../wasm-component-performance.md)
- [Component runtime boundary](../../wasm-component-runtime.md)
- [Measured-file admission and raw sample representation](../../../src/wasm/performance/parts/runner/p001/body.rs)
- [Compatibility and fixed-point comparison](../../../src/wasm/performance/parts/comparison/p001/body.rs)
- [Comparison behavior tests](../../../src/wasm/performance/tests/comparison.rs)
- [Technical companion](../README.md)
