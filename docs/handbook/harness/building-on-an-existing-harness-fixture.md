# Building on an existing harness fixture

Mode: How-to

## Goal and prerequisites

Extend a checked-in local deterministic suite so that the new scenario demonstrates one intended operation and its corresponding denial boundary. Start with [two-actor.preserves](../../../examples/two-actor.preserves), the current suite parser, and the behavioral tests linked below. Keep the original fixture unchanged while designing a separate candidate. This is a source-checked procedure; no candidate suite or command was executed for this document.

You need a clearly stated claim, a review workspace, and, for execution, a binary matching the source revision under review. Decide first whether the claim belongs to this harness. A claim about assertion visibility or suppressed clock requests fits its modeled boundaries. A claim about real sockets, service restart, or quorum behavior needs a different evidence layer. Changing an actor's kind to `remote-proxy` does not turn this recipe into a live multinode test.

## 1. Choose the smallest relevant behavior

Write the invariant before changing the fixture. For example: “The producer may send `hello` to the consumer only when its explicit capability context permits that request.” Use the existing send step and both native actor declarations. The positive case needs the producer's send grant; unrelated clock, random, assertion, and observation steps can be left out of a newly authored focused scenario.

Retain an explicit budget, explicit actor registry, and explicit capability fixture even when a sequence is empty. The parser has compatibility shapes that infer actors or use defaults, but the execution runner rejects those omissions for evidence-bearing suites. Do not interpret a parseable value as an executable scenario.

## 2. Change one admission input for the negative case

Build the negative case with the same send request and actors, but an explicitly empty capability grant sequence. This models missing authority, not malformed syntax. The checked-in `capability_missing_send_grant_denies_delivery` test follows that pattern and validates and replays the resulting denial report.

The acceptance condition is not a failing process. Expect a completed report containing denied admission and no `message-delivered` event for the request. Conversely, the positive test `capability_grant_allows_authorized_send` checks authorized admission and delivery. These tests are source evidence for how to formulate the pair, not proof your edited candidate has run.

Do not obtain a negative result by removing the whole capabilities record: that exercises suite preflight, not operation admission. Both tests are useful, but they answer different questions and should have different names and review expectations.

## 3. Decide whether policy is the variable instead

For policy behavior, retain the grant and add an explicit denial rule. Existing tests demonstrate a policy denying an otherwise granted send and clock request. The rule contains actor, action, target, value, and reason; `#f` in optional match positions means absence of that constraint, not an invented reference.

A useful failure case is an otherwise authorized readiness assertion blocked by policy. The existing assertion test checks rollback and absence of both assertion commit and observation. That makes the denial meaningful: a recorded deny marker alone would not demonstrate that consumers were protected from the forbidden state change.

Avoid removing the capability while adding the policy rule. Two simultaneous changes obscure whether the policy boundary was reached. If both inputs must change for a real use case, include a separate case that isolates each relevant boundary.

## 4. Keep executor and resource scope honest

For an admission-only change, retain native actors. Steel and Wasm require their own reviewed executor fixtures and preflight evidence. The executor registry distinguishes missing configuration from configured forms. Adapter and remote-proxy classifications are not, by themselves, evidence that an external service was contacted.

Keep budgets explicit and explain any change. Limits count steps, effects, events, and canonical report bytes. Evidence events and embedded suite data consume space too. The runner checks step count before trace collection, event/effect counts after each step, and report size during construction. A resource rejection therefore needs its own boundary interpretation; do not describe every resource error as an admission refusal before any modeled work.

## 5. Execute and retain the right artifacts

Use the [checked-in suite walkthrough](reading-and-running-a-checked-in-suite.md) for source-backed run and validation commands, substituting only your actual candidate path. Give every case a fresh output path. Record the exact input file and source revision, command exit status, output artifact type, and subsequent validation result. Never paste a made-up content reference into review evidence.

For negative operation scenarios, inspect the relevant observation and the absence of the prohibited effect or delivery. For preflight scenarios, retain the failure artifact and the discriminating cause. Stop if execution is unavailable; do not replace a denied or unexecuted case with a previous report whose suite differs.

## 6. Submit a bounded claim

Present the positive and negative cases together with their expected boundary, actual artifact, and evidence scope. A deterministic denial can be a successful test of protection. Neither that success nor its receipt authorizes the next operation. Follow the [distributed testing rules](../../distributed-testing.md) for broader evidence requirements and zero-retry release claims; exploratory reruns remain separately identified diagnostics.

## Sources

- [Handbook](../README.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Capability admission theory](../../technical/capabilities/capability-context-admission.md)
- [Two-actor fixture](../../../examples/two-actor.preserves)
- [Suite parser and explicit-fixture markers](../../../src/harness/parts/schema/p001/body.rs)
- [Policy rollback and missing-send tests](../../../src/harness/parts/mod/tests/m000/p001/body.rs)
- [Authorized-send and denied-effect tests](../../../src/harness/parts/mod/tests/m000/p002/body.rs)
- [Executor registry](../../../src/harness/executor.rs)
- [Resource accounting](../../../src/harness/parts/runner/p001/body.rs)
