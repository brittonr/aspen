# Prolly storage work reduction: measurements

Workload: the shell benchmark map (`src/prolly_map/tests/support.rs`): 64
entries under the standard profile (6 blocks, height 1), and one value update.
Counters are the pure `ProllyWorkCounters` plus the read-session observation.
The baseline column is the rebuild-first path, kept as the oracle:
`plan_edits_reference`, `diff_maps_reference`, and the per-block port load
(one `begin_read` per `read_block`, one decode in discovery and one in
validation). The numbers were captured by a throwaway test that called both
paths over the same Redb store and printed the counters; the test was removed
after capture.

| Operation | Counter | Baseline | After |
|---|---|---|---|
| Edit (one update) | leaves encoded | 5 | 1 |
| Edit | strategy | full-rebuild | incremental |
| Edit | prior validation nodes read / decoded | 6 / 6 | 6 / 6 |
| Diff (prior vs updated) | shared node count | 4 | 4 |
| Diff | validation nodes decoded | 12 | 8 |
| Diff | comparison nodes read | 12 (full flattened merge) | 4 |
| Diff | comparison subtrees not traversed | 0 | 4 |
| Diff | comparison nodes decoded | 0 (reuses flattened entries) | 0 (cache hits) |
| Snapshot load | blocks read | 6 | 6 |
| Snapshot load | read transactions opened | 6 | 1 |
| Snapshot load | nodes decoded | 12 | 6 |
| Snapshot load | bytes copied | 5993 | 5993 |

The generated benchmark (`config/prolly-map/generated/benchmark.json`) binds
`observed_shared_node_count = 4`, `observed_diff_subtrees_not_traversed = 4`,
and `observed_diff_comparison_nodes_decoded = 0` to the measured values through
`named_benchmark_thresholds_bind_measured_structural_facts`. The contract
requires sharing and avoided traversal to be separate, positive counts.

At tiny node capacities (core tests, `prolly_map::tests::work`, 256–640 byte
nodes over 96 entries) a 160-round generated edit sequence produces the
rebuild root and the rebuild blocks in every round. More than half of the
rounds take the incremental path and encode fewer leaves than the rebuild. A
rechunk bound of 4 entries against a 20-entry edit takes the rebuild fallback
and keeps the exact root.

Non-claims: these are logical work counts, not timings. They do not prove I/O
latency or throughput improvement on any host. The load holds its read
transaction only while it reads blocks, and nothing waits on an external
factor inside the session, so no retention pin is needed for the load.
