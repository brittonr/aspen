# Deterministic Playback Contract

Deterministic playback is a conditional relationship between a fully specified run and its canonical observations, not a promise that arbitrary executions converge. This article assumes familiarity with canonical references and effect admission. It explains the architectural contract alongside the inspected replay validators, keeping fixture evidence distinct from a universal execution guarantee. See the [Technical companion](../README.md) for neighboring topics.

## The condition before the conclusion

The [architecture](../../architecture.md#core-envelope-spine) binds playback to artifacts, dependency closure, initial state, schemas, policies, handler profile, seed or recorded effects, and relevant runtime/tool versions. Holding only the executable constant is insufficient. A changed policy can legitimately deny a previously admitted action; a changed initial state can legitimately produce a different final hash. Neither observation, by itself, demonstrates nondeterminism.

The inspected `ReplayRunIdentity` makes that context explicit. It includes artifact, dependency-closure, and initial-state refs; schema and policy ref vectors; capability and revocation ref vectors; handler profile; seed-or-effect-log ref; runtime and tool ref vectors; and replay profile. `validate_replay_freshness` constructs canonical identity values for the expected and evidence contexts, hashes them, and records the first freshness diagnostic in a receipt. This is an identity comparison over supplied facts, not a mechanism that discovers every dependency automatically. See the [freshness implementation](../../../src/deterministic/parts/replay/p009/body.rs).

It helps to separate three questions: is this the intended run context, were external observations consumed under the intended contract, and did the resulting trace match? A passing answer to one does not eliminate the other two.

## Effects are replay inputs, not ambient permission

A replay cannot repair an absent recorded response by quietly reading today's clock or querying the network. That would introduce a different observation while retaining the appearance of the old execution context. The fabric therefore separates deterministic in-memory law from adapter effects; live and simulated substrates belong behind admitted boundaries, as the [fabric ownership rules](../../distributed-system-fabric.md#ownership-law) explain.

The concrete effect-log validator accepts `EffectLogEntry` and `ConsumedEffect` slices. Each recorded entry binds a sequence, effect kind, run identity, handler profile, turn, boundary, request, and response. Consumption facts bind the sequence, kind, request, response, boundary, and whether live fallback was used. The validator first checks supported shapes and bounds, then evaluates mismatch diagnostics. Its current entry bound is 1024; that is a bound of this validator, not a system-wide throughput or retention limit. See [effect-log validation](../../../src/deterministic/parts/replay/p006/body.rs).

The [matching helpers](../../../src/deterministic/parts/replay/p007/body.rs) require recorded sequences to start at zero and advance contiguously, reject duplicate recorded request refs, compare matching consumption bindings, reject unconsumed recorded entries and missing recorded effects, and reject reported live fallback. Importantly, these are checks over supplied observations. The validator does not monitor every operating-system effect or prove that a caller faithfully reported its behavior. Its consumed-sequence set checks also should not be restated as a general exactly-once execution theorem.

## Worked reasoning example: an equal final state can still fail

Consider an illustrative two-effect playback. Recorded entries have sequences zero and one, each with distinct request refs, and both belong to the same expected run and handler profile. Consumption at sequence one reports the right response but the wrong boundary ref. The final application state happens to match because the example operation ignores that response.

Matching final hashes does not repair the error. The binding comparison can report a boundary mismatch before any claim about application state is relevant. The contract concerns canonical traces and receipts as well as outputs and final state. Discarding intermediate evidence would erase exactly the distinction needed to diagnose this run.

Now consider a separate variant where sequence one has no recorded entry and a shell reports that it used a live observation. There are at least two violated conditions. The implementation reports the first applicable diagnostic, so a missing-recorded-effect diagnostic may appear before the live-fallback diagnostic. A reviewer should not infer that unreported later failures were accepted. Diagnostic precedence is a localization mechanism, not a complete enumeration of everything wrong.

## Reviewing playback evidence

Start with context identity before investigating trace differences. Check that evidence refers to the intended policy, capability, revocation, and handler profile, not just the same artifact. Then examine recorded-effect completeness and bindings. Finally compare the relevant trace, output, receipt, and state references at the supported boundary granularity.

The existing effect-log tests include ordered consumption, sequence gaps, duplicate or reordered entries, mismatched run/profile bindings, extra or missing effects, and live fallback. They are useful suggested verification targets in [the replay tests](../../../src/deterministic/parts/replay/tests/m000/p003/body.rs). No replay command or test suite was executed to write this article; inspected code and tests establish the description, not a new run receipt.

## Limits and non-claims

A fixture's replay success is evidence for that fixture and comparison profile. It does not establish complete instrumentation of arbitrary native code, live transport delivery, durable persistence, distributed liveness, or production readiness. The root replay module also includes storage-facing helpers; its name does not make the whole module a pure core. Deterministic re-execution, duplicate suppression, and successful retries are distinct properties, and none alone supplies an exactly-once distributed-effects claim.

## Sources

- [Architecture: deterministic run context](../../architecture.md)
- [Fabric ownership and non-claims](../../distributed-system-fabric.md)
- [Modularity and shell responsibilities](../../modularity-boundaries.md)
- [Replay run identity and freshness](../../../src/deterministic/parts/replay/p009/body.rs)
- [Effect-log validation](../../../src/deterministic/parts/replay/p006/body.rs)
- [Effect-log matching helpers](../../../src/deterministic/parts/replay/p007/body.rs)
- [Effect-log behavioral tests](../../../src/deterministic/parts/replay/tests/m000/p003/body.rs)
