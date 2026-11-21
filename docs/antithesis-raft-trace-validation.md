# Antithesis Raft models: source inspection and Aspen trace scope

This is the original related-work and default-off scoping slice of [Aspen #63](https://github.com/brittonr/aspen/issues/63). It adds no recorder, checker, or runtime dependency. The [references catalog](references.md) records both repository pins and all four X posts. X did not serve the posts to the reader; the links and descriptions come from the issue and its [coauthor-post comment](https://github.com/brittonr/aspen/issues/63#issuecomment-7048460037). Wayne's point is that a specification proof says what the design guarantees, while traces from fuzzing and deterministic simulation can test whether particular executions follow that design. His follow-up restates the TLAPS and Veil proof claim.

## Frozen subset: HashiCorp Raft

Read only the [HashiCorp artifact at `abcc6e40ec7a082a480be956981f345aa989664c`][hraft] through GitHub, not the etcd artifact. The SHA-256 of the fetched `HashicorpRaft.tla` bytes was `d41436646fea8a59b1cd394e882a99339b52769379eb421819e6055039c623a7`, matching the first entry in [the proof manifest][hashes]. The fetched `MCRaft.tla` and `MCCount2.cfg` also matched their manifest entries. This confirms those source identities, not the proofs or every entry in the manifest.

The [model declarations][properties] and [proof entry points][theorems] agree on these exact statements:

| Declared TLAPS theorem | Property inspected in the model |
| --- | --- |
| `ElectionSafetyThm == Spec => []ElectionSafety` | Any two recorded elections in the same term name the same server, even if the first leader has since stepped down. |
| `LogMatchingThm == Spec => []LogMatching` | If two logs contain entries of the same term at the same index, their prefixes through that index agree. |
| `CommitSafetyThm == Spec => []CommitSafety` | Of any two recorded committed prefixes, one is a prefix of the other. |
| `StateMachineSafetyThm == Spec => []StateMachineSafety` | Any two recorded applications at the same index hand the state machine the same entry, across nodes and restarts. It says nothing about the state machine's contents or outputs. |
| `LeaderCompletenessThm == Spec => []LeaderCompleteness` | For each server currently in the leader role, every prefix committed in an earlier term is a prefix of its log. |

`InvInvariant == Spec => []Inv` is the shared induction result; [`Inv`][inv] includes `OneVotePerTerm`, which compares recorded votes for the same voter and term. The model assumes a finite fixed voter set, a `Nil` sentinel outside that set, and Boolean `PreVote` and `Snapshots` switches. The author theorem claims are for the model without TLC's term or log-length bounds, with either value of the switches. The authors also report Lean kernel checks of a Veil translation; their [equal bounded state counts][veil] are evidence for the translation, not a proof of equivalence. The inspected [`MCCount2.cfg`][count] is only a bounded TLC instance: three servers, one value, terms and logs at most two, one entry per batch, at most two messages under `Bounded`, and both switches off. Its reported 6,488 distinct states are not an independent result here.

**Limits.** The [frozen model guide][limits] describes small steps for elections, split durable vote writes, replication and conflict deletion, application, restart, pre-vote, `TimeoutNow` leadership transfer, and snapshot creation, compaction and installation. A growing set of message facts permits delayed, lost, duplicated and reordered deliveries. It models the design with five patches applied in the harness, not arbitrary upstream releases. It omits membership changes, async/fast-path heartbeats, the `Restore` API, stores that cannot delete from the front of the log, application contents and liveness. The article separately notes that etcd's lease-based reads are out of scope. Storage assumes an individual write is atomic and not corrupted and completed writes retain order; the harness can crash between writes but does not corrupt or lose synced data.

The [trace binding][binding] is separate evidence: wrappers report ordered writes, messages, replies, application calls and role changes; boot readback resolves an announced write that may or may not have survived a crash. A per-node sequence gap is an error. For reports mapped to several model actions (or with custom handling), the correspondence and the implication from the binding's `Safe` summary to model properties are argued, not mechanically proved. The authors report that their patched three- and five-voter runs conformed; the driver also checks client-visible behavior, including reads, outside the theorem above. Source inspection confirms these distinctions but does not establish a proof-checker verdict, reproduce the authors' executions, or verify either implementation.

## Bounded TLC result; proof replay not run

The original [three-file subset][hashes] passed all three upstream SHA-256 checks. On Linux x86_64, Aspen's pinned TLA+ 1.7.4 package (`TLC2 2.19`, revision `5a47802`) checked the unmodified [`MCCount2.cfg`][count] and `MCRaft.tla`:

```sh
timeout --kill-after=5s 120s "$TLA_PACKAGE/bin/tlc" \
  -workers 2 -metadir states-tlc-2.19 -config MCCount2.cfg MCRaft.tla
```

The checker exited zero: **113,485 generated, 6,488 distinct, zero queued**, graph depth 18. The distinct count matches the authors' bounded result. The checker JAR BLAKE3 was `36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38` and the retained result BLAKE3 was `e5a367f61bfaea2b6658384f9ece0e2af706d3bdb664ec112ab26d6a951c88c6`. The first invocation exited 1 before evaluation because this TLC version rejects the upstream script's `-noGenerateSpecTE` flag; the corrected command above changed only packaging/runtime options, not model bytes or declared bounds.

The result covers the invariants listed in `MCCount2.cfg` with pre-vote and snapshots disabled. It does not replay the TLAPS or Lean proofs, validate an implementation trace, or exercise Aspen. No checker dependency or default rail changed.

The smallest direct TLAPS candidate for the named safety theorems is a single-module check, run from `antithesis-fm/` in a disposable copy of the HashiCorp pin, with `TLAPM` pointing to an exact tool build:

```sh
(cd spec/proof && sha256sum -c spec.sha256) &&
  TLAPM=/absolute/path/to/bf665f4d162c1ae5010ca878578cd50a810ec205/bin/tlapm \
  THREADS=2 bash spec/proof/final.sh RaftSafety
```

The [upstream runner][runner] invokes `tlapm -I .. --threads 2 --cleanfp --strict RaftSafety.tla` from `spec/proof/` and retains `final-RaftSafety.log` and `final.log`. Require exit zero and `All N obligations proved` in the module log, not a successful parse. [`RaftSafety`][theorems] extends the earlier proof modules; a single-module result is not a fresh check of every imported lemma. Omit `RaftSafety` to run all 12 modules in order for that claim. Retain the exact tool and backend identities, source pin, command, exit status and logs; timeout, missing backend or unproved obligations are not a pass.

The exact [author TLAPM revision][tlapm] is not available on this host's `PATH`. The rolling `1.6.0-pre` [tag resolved to a different revision][rolling] when inspected, and the [CI run][tlapsci] at the author revision lists only JUnit artifacts, not a checker distribution. Pinning the rolling download's digest would freeze different bytes, not recover the authors' toolchain. Provisioning the author revision requires an exact prebuilt cohort or the [upstream source-build procedure][tlapsbuild] with its backends. The [Lean alternative][veil] pins Lean `4.35.0` and Veil `1287c31d0d62e7ae069993845fe7e5d9736b8fba`; even the upstream `ACTIONS=Restart bash scripts/veil-check.sh` subset only checks that action's preservation goals and first invokes `lake build Veil`. Neither Lean nor Lake was on `PATH`. A frozen proof replay was therefore not practical in this read-only, no-build assignment. No checker was invoked; there is no pass, failure, or inconclusive prover output to report.

## Aspen: what maps and what does not

- [ConsensusFastPath](consensus-fastpath-tla.md) is Aspen's completed bounded TLC/TLA-ts campaign for fast admission, promise/ack certificates, conflict ordering, and view recovery (three replicas, two commands, three views). Its progress claim assumes weak fairness for abstract recovery. The separate Quint lane compares the Rust API projection per step. Neither models Raft's term/vote persistence, lossy RPCs, snapshot transfer/compaction or restart I/O; neither is a live-Raft proof. The [verified node-replication pilot](verified-node-replication-pilot.md) is local multicore/NUMA work, not network consensus.
- Archived [`RaftReadIndex.tla`][readmodel] is closer to the live read path. `ReadLinearizable` says each completed read's `requiredIndex` covers `mustObserve`, the highest write index acknowledged before the read started. This is not full client-value linearizability: entries are abstract write/no-op tokens with a term, RPCs are atomic and lossless and append sends the whole log. `BecomeLeader` explicitly excludes another leader in the term, so this model does not independently establish election safety. The archive's [guard-exception run][readresult] reports 11,153,290 distinct states and an empty queue in its bounded instance, not live conformance. No live Raft spec is wired into the in-tree parity campaign.
- In [Aspen's live Raft at `f2f171a80c89f206caabbb43fc68821c947368c8`][live], `become_leader` initializes replication without appending a no-op. [`handle_read`][readcode] rejects an unready linearizable read as `Retryable`; exact [`is_commit_index_current`][guard] accepts either a current-term commit or a commit index at the end of the leader log. A pending read records the term, required index and acknowledgers; [a matching quorum][readack] produces `Current` at that required index. Ignoring the committed-log exception would produce a false divergence for legitimate live reads. Neither upstream model is Aspen's specification.

**Existing records can seed the projection, but are not a complete trace.** [Durable replay][durable] reconstructs a local hard state, log/suffix, commit and snapshot from the retained records; it resets volatile state and does not replay historical elections or the message schedule. The [live executor][executor] has a typed input and planned effects, emits ordered effect-kind/evidence refs and distinct send failures, and distinguishes applied, denied and failed transitions. An effect ref is not the input message or the state, and a send receipt is not delivery. The [Raft ledger][ledger] records selected commit and read-quorum evidence, snapshots and failures; it can suppress routine heartbeats. [Process receipts][receipts] expose checkpoint/terminal term, role, commit/applied indexes, quorum/snapshot refs and pending-read counts, not the pending-read entries or per-transition global states. The [native conformance record][native] projects committed command/apply identities and application state for observing matching applies, not elections, uncommitted log entries, or all participants' steps. Retained deterministic inputs and receipts help reproduce or cross-check a run; equal final hashes cannot rule out a transient violation.

## What a later complete-state check would need

Start with an explicit opt-in, fixed-membership three-voter live run (write, leader change, probe/read) and a **pinned Aspen model/config** based on a documented `RaftReadIndex` subset. Freeze the Aspen source/build, run profile and initial durable/volatile state, participant/generation and identity mapping, input or seed and fault/delivery schedule, projection schema, model/config and checker. Do not silently exclude a behavior the selected run actually exhibits.

1. Capture each admitted **and rejected** live event with a gap-detectable global order or an explicitly certified merge of per-node order and causal edges. Include typed timer, client and RPC inputs; correlate send, delivery, drop/duplicate and reply, rather than treating a send as a received RPC. Use actual effect results, including durable intent, finish/flush, failure or crash uncertainty, and recoveries when in scope. Never publish planned state as executed state.
2. Emit the initial state and a state after every mapped step. For a `RaftReadIndex`-style `vars`, assign every node's `currentTerm`, `votedFor`, `role`, ordered `log` (index, term, stable entry identity), `commitIndex`, `matchIndex` and `votes`, plus `acked` and `reads`. The last two need actual client write-response and read-invocation order, request identities, the required/observed index and term, each probe acknowledger, and completion or cancellation. In particular, local commit alone does not establish `acked`, and a pending-read count cannot determine `reads`. Any state the adopted model adds (e.g. message queue, `next_index`, fencing/rejoin, `last_applied`, snapshot boundary and retained suffix) also needs a complete projection, or an explicit out-of-scope restriction for the first run.
3. State exactly which live substeps map to `Init`, a permitted `Next` action or stutter, and how modeled history is derived from recorded events. Do not invent intermediate states to make an atomic abstract RPC fit; change the abstraction or select a run whose genuine boundaries fit it. Reject missing or undecodable state, unresolved refs, gaps and unsupported mappings; never fill them by guessing or report them as `valid`. Check a clean trace and a separately authorized negative control against the **same baseline** spec/config, retaining the first failing edge or invariant and the input evidence. A valid result is conformance of one recorded sequence, not exhaustive safety, liveness or a proof of Rust.

This note does not implement the later [live-capture slice of #63](https://github.com/brittonr/aspen/issues/63#issuecomment-7048725486), change [Aspen #62](https://github.com/brittonr/aspen/issues/62), or move schema/checker ownership from [trace-lab #6](https://github.com/OnixResearch/trace-lab/issues/6). No Aspen implementation trace or mutant was checked.

[hraft]: https://github.com/antithesishq/hashicorp-raft-formalized/tree/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm
[properties]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/spec/HashicorpRaft.tla#L455-L490
[theorems]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/spec/proof/RaftSafety.tla
[inv]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/spec/proof/RaftDefs.tla#L170
[hashes]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/spec/proof/spec.sha256
[limits]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/spec/README.md
[veil]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/veil/README.md
[count]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/spec/MCCount2.cfg
[binding]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/spec/RaftBinding.tla
[runner]: https://github.com/antithesishq/hashicorp-raft-formalized/blob/abcc6e40ec7a082a480be956981f345aa989664c/antithesis-fm/spec/proof/final.sh
[tlapm]: https://github.com/tlaplus/tlapm/commit/bf665f4d162c1ae5010ca878578cd50a810ec205
[rolling]: https://github.com/tlaplus/tlapm/commit/961e909fa3aca1fba412ee0908c5848b808e4b24
[tlapsci]: https://github.com/tlaplus/tlapm/actions/runs/43825070713
[tlapsbuild]: https://github.com/tlaplus/tlapm/blob/bf665f4d162c1ae5010ca878578cd50a810ec205/DEVELOPING.md#build--install-tlaps
[readmodel]: ../.cairn/archive/2026-11-16-guard-reads-after-leader-change/evidence/tla-2026-11-16/RaftReadIndex.tla
[readresult]: ../.cairn/archive/2026-11-16-guard-reads-after-leader-change/evidence/tla-2026-11-16/RaftReadIndex_GuardException.txt
[live]: https://github.com/brittonr/aspen/blob/f2f171a80c89f206caabbb43fc68821c947368c8/src/fabric_consistency/raft/transition/election.rs#L134-L153
[readcode]: https://github.com/brittonr/aspen/blob/f2f171a80c89f206caabbb43fc68821c947368c8/src/fabric_consistency/raft/transition/client.rs#L35-L107
[guard]: https://github.com/brittonr/aspen/blob/f2f171a80c89f206caabbb43fc68821c947368c8/src/fabric_consistency/raft/transition/support.rs#L136-L143
[readack]: https://github.com/brittonr/aspen/blob/f2f171a80c89f206caabbb43fc68821c947368c8/src/fabric_consistency/raft/transition/message/read.rs#L48-L83
[durable]: ../src/fabric_consistency/raft/parts/recovery/p000/body.rs#L46-L189
[executor]: ../src/fabric_consistency/raft/executor/mod.rs#L49-L85
[ledger]: ../src/fabric_consistency/raft/parts/evidence/p001/body.rs#L72-L187
[receipts]: ../src/fabric_consistency/raft/live_process/receipt.rs#L6-L40
[native]: ../.cairn/changes/add-chaoscontrol-consensus-conformance/evidence/2026-11-20-native-consumer-adapter.md
