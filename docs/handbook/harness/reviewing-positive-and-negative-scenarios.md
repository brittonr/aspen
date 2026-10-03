# Reviewing positive and negative scenarios

Mode: Review checklist

Use this checklist when a change claims that a harness fixture demonstrates both permitted behavior and protection against a forbidden operation. Ask for evidence at the consumer-visible boundary, not just a successful command or a report containing a denial word. This checklist was derived from source and existing tests; no suite, command, or runtime verification was performed for this document.

## Establish the claim and evidence layer

- [ ] Is the invariant stated in terms of an observable result, such as delivery, assertion visibility, rollback, or effect suppression? “The harness passes” is not a behavior claim.
- [ ] Does the claim identify its scope as local deterministic fixture evidence? Require a different evidence layer if the claim depends on a live service, operating-system process lifecycle, VM networking, or distributed quorum.
- [ ] Are the exact source revision and suite artifact supplied? A recognizable suite name does not prove the embedded input matches the reviewed file.
- [ ] Are unavailable execution and source-only review explicitly separated from observed runs? Do not accept a documented command as evidence that it was executed.

The [distributed testing guide](../../distributed-testing.md) makes these evidence layers explicit. A local report cannot replace platform evidence or establish production readiness, and a report reference is not current permission to perform an operation.

## Check the positive case's inputs

- [ ] Are actor registry, capability fixture, and budget explicitly present? Require actual fixture records, not inferred actors or constructor defaults.
- [ ] Does every actor named by a step exist exactly once in the registry? Inspect both send participants, not only the primary actor.
- [ ] Does the grant cover the exact request dimensions intended by the test: actor, action, target, and value? A broad grant may allow the example while failing to test the desired scope restriction.
- [ ] Is the executor appropriate to the claim? Native modeled behavior, configured Steel/Wasm execution, and adapter/remote-proxy fixture descriptions must not be conflated.
- [ ] Is the budget justified for the scenario, including evidence events and canonical report bytes? A large copied limit is not resource-boundary evidence.

For the checked-in two-actor fixture, review the sequence as well as the grants. The consumer observes readiness before the producer asserts it; later retraction exercises a different transition. Removing or reordering steps changes the scenario even if actor names and the seed remain unchanged.

## Check that the negative case discriminates

- [ ] Does it change the admission input needed to reach the claimed boundary, while holding the operation stable where possible?
- [ ] If testing missing authority, is the capability record explicitly empty or deliberately nonmatching rather than absent?
- [ ] If testing policy denial, does a matching capability remain present so that missing authority does not mask the policy boundary?
- [ ] If testing malformed input or missing required evidence, is it separately named as preflight rejection rather than operation denial?
- [ ] Does the acceptance condition require absence of the prohibited delivery, assertion visibility, or effect, not only presence of a deny decision?

**Worked review:** Pair the authorized-send test with the missing-send-grant test. The positive case grants producer-to-consumer send and checks authorized delivery. The negative case keeps the actors and send but supplies an empty grant list; it checks unauthorized admission and absence of `message-delivered`, then validates and replays the denial report. Those are complementary outcomes. A nonzero exit caused by a missing actor registry would not satisfy the negative operation claim.

A second useful pattern is denied clock access: the existing regression checks that neither effect request nor effect response appears. This guards the boundary before the modeled effect, instead of merely checking that the final output omitted a value.

## Inspect artifacts, not their filenames

- [ ] Is a successful run artifact actually `harness-report-v1`, and is a rejection artifact identified as `harness-failure-v1`? The requested report destination can hold either.
- [ ] Have reviewers separated report-level `pass` from each step's admission decision? A correct denied operation can appear inside a completed report.
- [ ] Does the embedded suite bind the intended input, and do the observation indices and step refs identify the claimed operation?
- [ ] Are before/after state refs, hostcall evidence, and turn journals retained, rather than reducing evidence to the final state hash?
- [ ] Has full report validation been distinguished from display and standalone replay? The report-validation CLI performs evidence validation and then replay; a rendered summary alone does neither.
- [ ] Are original artifacts preserved without hand-edited hashes, event removal, or rewritten budget usage?

Do not demand invented terminal transcripts. Ask for actual artifact references and observed results when execution exists; otherwise retain a clear source-only review outcome.

## Check failure timing and publication safety

- [ ] Does a resource failure claim identify the exceeded dimension and where it is checked? Step count is pre-trace, but event/effect counts follow a step and report bytes are checked during report construction.
- [ ] Are fresh isolated output paths used? The CLI's ordinary file writer can overwrite an existing artifact.
- [ ] Have embedded inputs been considered before sharing failure artifacts? Suite/report failure diagnostics may include the original value and are not automatically redacted.
- [ ] Are exploratory reruns separated from release evidence? The governing distributed-testing contract rejects retry-only deterministic pass claims without a separately accepted remediation boundary.
- [ ] Are source discrepancies recorded narrowly? For example, executor classification accepts configured adapter variants while some rejection wording says disabled; neither phrase alone proves a live adapter effect.

## Review disposition

Accept only the bounded claim supported by the actual evidence. Return missing behavior, mismatched suite identity, or an unexercised negative boundary as specific review gaps. Record unavailable execution honestly rather than promoting fixtures or source inspection into runtime proof. For theory behind this separation, use the technical companion instead of expanding the checklist into an architecture claim.

## Sources

- [Handbook](../README.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Deterministic playback theory](../../technical/foundations/deterministic-playback-contract.md)
- [Two-actor fixture](../../../examples/two-actor.preserves)
- [Policy rollback and missing-send tests](../../../src/harness/parts/mod/tests/m000/p001/body.rs)
- [Authorized-send and denied-clock tests](../../../src/harness/parts/mod/tests/m000/p002/body.rs)
- [Runner admission](../../../src/harness/parts/runner/p000/body.rs)
- [Runner resource timing](../../../src/harness/parts/runner/p001/body.rs)
- [Report validation CLI handler](../../../src/cli/evidence/report/ops.rs)
- [Executor classification](../../../src/harness/executor.rs)
