# Reading and running a checked-in suite

Mode: Walkthrough

This walkthrough follows `examples/two-actor.preserves` from explicit fixture inputs to a local deterministic report. It is source-checked, not an execution record: no commands or tests below were run for this document. Have a `molten` binary built from the checkout under review available before trying the optional command stages. Build availability is a separate prerequisite, not evidence supplied by this page.

## 1. Establish the input boundary

Open the [checked-in fixture](../../../examples/two-actor.preserves). Its suite name is `two-actor`, and its seed is `7`. It declares two native actors, `consumer` and `producer`, an explicit budget, six capability grants, and six ordered steps. The budget limits mean at most 64 steps, 16 effects, 256 events, and 65,536 canonical report bytes; they are not measured usage.

The six steps observe `service.ready`, assert that value, send `hello`, request a clock value, request a random value with upper bound 100, and retract the readiness assertion. Their ordering matters. An observation established before an assertion supplies a different scenario from registering it afterward. The seed belongs to this local deterministic scenario; it is not host entropy or a timestamp.

Do not replace the suite's capabilities with a report from an earlier successful run. The grants are explicit fixture inputs. Reports describe what happened and do not confer permission on later runs.

## 2. Follow parsing into execution admission

The public path is `run_suite_value`, through [the runner](../../../src/harness/parts/runner/p000/body.rs), into `parse_suite`. Parsing retains the original Preserves value for the suite reference. Canonical Preserves plus BLAKE3 define content identity, not the Rust struct layout or the text file's whitespace.

The parser can represent omitted fixtures, but `prepare_suite_run` refuses evidence-bearing execution without an explicit actor registry, capabilities, and budget. For this example all three are present. The native actor declarations need no executor configuration. Actor IDs referenced by steps must occur in the registry. Policy is a separate optional fixture; the checked-in example does not declare policy denial rules.

This is the first useful observable boundary: malformed input or missing required execution evidence is a failure artifact, not a report proving an attempted operation was denied correctly.

## 3. Produce a fresh report, if the runtime is available

Choose `HARNESS_REPORT` as a new file in an isolated review directory. The CLI writer creates parent directories and otherwise uses ordinary file writing, so it can overwrite an existing destination. The guard below helps avoid an accidental overwrite in a single-user review workspace; it is not a concurrent filesystem admission mechanism.

Source-checked, not executed. Command spelling comes from the [root command declaration](../../../src/main/root/parts/command/p000/body.rs); output and failure behavior comes from the [harness CLI](../../../src/cli/test/harness.rs), reached through [main's path aliases](../../../src/main.rs).

```sh
test ! -e "${HARNESS_REPORT:?Set a fresh report file path}" &&
  molten test run examples/two-actor.preserves --report-out "$HARNESS_REPORT"
```

On success the file contains `harness-report-v1`. On an input or execution error, the same requested destination may instead contain `harness-failure-v1`. Preserve the exit status and inspect the artifact type before taking the next stage. Merely finding a file is not success evidence.

## 4. Read the turn boundaries

For each step, the runner hashes the before-state and step, computes admission, records actor and hostcall context, executes the allowed boundary, and records the after-state. It adds a `turn-journal-v1` binding the state transition, policy, capability, budget, and event references. Observations are ordered by step index.

For this fixture, useful evidence to inspect includes registration and assertion visibility, message delivery, the clock/random effect request-response pairs, and retraction behavior. These are expected categories from source, not fabricated terminal output or asserted values from a new run. Event counts include evidence records, not just user-visible messages. A six-step suite therefore does not have a six-event budget requirement.

A denied step is different from broken suite admission: the runner records rollback evidence and suppresses actor execution and runtime effects for that step. A completed report can consequently have status `pass` while containing a denied operation.

## 5. Validate the artifact before claiming reproducibility

Source-checked, not executed. The [report declaration](../../../src/cli/evidence/report/command.rs) defines this command, and its [handler](../../../src/cli/evidence/report/ops.rs) performs structural/evidence validation followed by replay.

```sh
molten test report validate "${HARNESS_REPORT:?Set the report produced above}"
```

Validation checks admission and executor evidence, effect-log agreement, and recorded budget usage. Replay consumes the embedded suite and effect log and compares observations and state boundaries. Matching only the final state is insufficient. Preserve the original report rather than editing its hashes or usage fields to make it validate.

## 6. State the result narrowly

If actually executed successfully, this path supplies local deterministic fixture evidence. It does not execute two operating-system nodes, demonstrate WAN delivery, establish current authority, or prove production readiness. The broader [distributed testing guide](../../distributed-testing.md) explains which claims require simulation, local process, VM, or live evidence. Record unavailable execution as unavailable; a source walkthrough cannot substitute for that run.

## Sources

- [Handbook](../README.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Deterministic playback theory](../../technical/foundations/deterministic-playback-contract.md)
- [Two-actor fixture](../../../examples/two-actor.preserves)
- [Suite parser](../../../src/harness/parts/schema/p001/body.rs)
- [Runner admission and trace](../../../src/harness/parts/runner/p000/body.rs)
- [Runner budgets and rollback](../../../src/harness/parts/runner/p001/body.rs)
- [Report validation and replay](../../../src/harness/replay.rs)
