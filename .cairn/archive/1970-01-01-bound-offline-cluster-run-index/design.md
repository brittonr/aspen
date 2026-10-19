## Context

`assess_run_directory` already limits considered artifacts to `MAX_RUN_ARTIFACTS = 4_096`, but `verify_cluster_run_directory` previously materialized an arbitrary-length index before entering that assessment. The input file is operator-controlled local filesystem data; denying count alone still leaves unbounded file allocation and a single oversized line.

## Decisions

### Reuse the existing artifact count

Export the existing assessment ceiling to the parser. Check immediately before appending a parsed row so the 4,097th row is not cloned. Blank lines retain their established handling; a 4,096-entry index remains eligible for normal validation, not automatically a passing run.

### Cap file bytes at the admission boundary

The 4 MiB ceiling budgets up to 1 KiB per existing maximum indexed artifact; it is local input policy, not a claim that any future artifact path must fit that per-row average. Read at most 4 MiB plus one byte from the operator-supplied file, then perform the same byte preflight for direct parser calls. Exact-limit input remains admitted to the parser. Size rejection precedes header/row errors when the input itself exceeds this bound.

## Risks and non-claims

Historical indexes exceeding the new local budget are denied; they are not silently truncated. No canonical receipt field, accepted Cairn specification, production cluster evidence gate, or test/production separation policy is changed. The harness child-run path and its two-node CLI integration retain their existing semantics.
