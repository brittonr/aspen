# Bound offline cluster run-index intake

## Why

The recovered F09/F10 Octet catalog identifies two `unbounded_collection_growth` findings at `src/cluster_harness/runner.rs:1004` (F628/F629). The compiled `molten cluster harness-verify --run-dir` command accepts a local operator-supplied `artifact-index.tsv`: it previously read the whole file and appended every parsed row before the core's existing 4,096-artifact assessment ceiling applied. Both over-count and over-byte regression cases were accepted before this source change. This is a real external local CLI input, not unauthenticated network admission or proof of a production cluster.

## Proposed change

Share the existing core 4,096-artifact ceiling with the parser, reject a 4,097th index entry before allocating its fields, and read at most 4 MiB plus one byte before rejecting oversized files. Keep 4,096 entries and exactly 4 MiB eligible for ordinary artifact and verification-companion checks. Preserve the existing no-symlink/path traversal and receipt rules.

## Ownership and non-claims

The isolated owner changes the test-harness CLI, regression tests, and product documentation. This package is reviewable separately from production world-head storage and from accepted testing-harness specifications. Warning-only scoped Octet is not full strict-ci acceptance. No lifecycle sync, waiver, pin change, commit, or production-evidence promotion follows from the proposal.
