## Phase 1: Aspen-owned diagnostic inputs
- [x] [serial] Capture one bounded, explicitly selected run from the existing live Raft fixture, with ordered observed earlier acknowledgement, read retry, new commit and current read. r[molten.testing.live_raft_trace.capture]
- [x] [serial] Adapt and freeze the post-election read-index model and its bounded configuration; state the exact live observation mapping and unsupported cases. r[molten.testing.live_raft_trace.model]

## Phase 2: Independent verification and retention

- [x] [depends:trace-lab-live-raft-mapping] Run the pinned independent checker against one complete observed sample and a separately identified bounded negative arm, or retain exact unsupported causes without promoting them. r[molten.testing.live_raft_trace.checking]
- [x] [serial] Retain exact source, tool, model, config, ordered facts, verdicts, non-claims and gate review; sync and archive only after evidence review. See [retained verification](evidence/verification.md). r[molten.testing.live_raft_trace.checking]

## Model-only verification

Pinned TLC 2.19 (`5a47802`, JAR BLAKE3 `36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38`) ran `tlc -workers 1 -metadir states -config RaftReadIndexWindow.cfg RaftReadIndexWindow.tla`. With three voters, term 2 and at most two entries, the checker exited zero: 81 generated, 69 distinct, zero queued, depth four. Model BLAKE3 `3cfbb662b374967291efa7e1d9bb18165b24da9ee15f2736105c25fcf29fcdf1`; config `a4be5f8ae6549ce8303d1fdc921f6aab5012303f1fe2ea84f46b70d8933884b5`; log `17ece448c30a6ddfd332008675409696b2ca45f98111c90db609a8f5fb362fd0`. The earlier preflight used the wrong JAR layout and stopped before invoking TLC. This result checks only the model, not the live fixture's trace or an independent negative arm.

