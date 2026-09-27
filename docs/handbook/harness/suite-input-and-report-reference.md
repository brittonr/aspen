# Suite input and report reference

Mode: Reference

This reference maps the local harness's supplied inputs, generated artifacts, and inspection surfaces to their owning source. It describes source-checked behavior, not an executed run. The wire values are Preserves records: Rust field names below help navigation but are not a second serialization format. Canonical Preserves encoding and BLAKE3 determine identity.

## Input records

The suite parser recognizes `harness-suite-v1`, with schema string `molten.harness.suite.v1`, a string name, a nonnegative integer seed, fixture records, and a final step sequence. It accepts four through eight fields, but execution has stricter explicit-evidence requirements. Duplicate budget, actor, capability, or policy fixtures are rejected.

| Input | Actual fields or shape | Owner and operational meaning |
| --- | --- | --- |
| Suite header | Schema, name, seed | `parse_suite` in schema p001; original source value is retained for suite identity |
| `budget-v1` input | Schema plus `<limits steps effects events report-bytes>` | Schema p029; required explicitly by runner preparation |
| `actor-registry-v1` | Schema plus actor sequence | Schema p022; duplicate actor IDs are rejected |
| `actor` | ID, kind, optional executor record | Schema p022; an actor entry has two or three fields |
| `capabilities-v1` | Schema plus grant sequence | Schema p034; explicit empty sequence represents no grants |
| `grant` | Actor, action, target, value | Schema p034; optional constraints, not live credentials |
| `policy-v1` | Policy fixture | Suite parser p001; independent from the capability fixture |
| Step sequence | Ordered modeled operations | Runner p000; determines observation order and step references |

The [two-actor example](../../../examples/two-actor.preserves) uses limits `64 16 256 65536`, native actors, six grants, and six steps. These are fixture values, not system-wide recommended limits. Its clock and random requests are deterministic local harness operations, not evidence of reads from the host clock or entropy source.

`Suite` also has `actors_explicit`, `capabilities_explicit`, and `budget_explicit` markers. They distinguish omitted data from explicitly supplied fixtures after parsing. Those markers explain why inferred actors or a default budget can exist in memory while evidence-bearing execution still refuses the suite. Do not erase that distinction when constructing a suite through an API.

## Current generated report

The current `report_value` builder emits a 17-field `harness-report-v1` record. This table groups adjacent positions; numbering is zero-based after the record label.

| Positions | Content | Review use |
| --- | --- | --- |
| 0–4 | Schema, `pass`, `deterministic`, `local-deterministic`, hash algorithm | Identify report kind and declared scope; not independent verification |
| 5–7 | Suite ref, initial-state hash, final-state hash | Bind input and state endpoints |
| 8 | Embedded original suite value | Reconstruct the intended scenario rather than guessing from its name |
| 9–11 | Policy, capability, budget gates | Inspect the supplied admission evidence |
| 12–13 | Actor registry and executor preflights | Bind actor/executor interpretation |
| 14 | Observation sequence | Inspect each step's state transition and events |
| 15 | Effect log | Supply recorded request-response pairs for replay |
| 16 | Budget limits and usage | Check actual counted usage against declared bounds |

The builder lives in [schema p004](../../../src/harness/parts/schema/p004/body.rs). A report's canonical reference is calculated from the report value; the `Report.report_ref` Rust field is not an extra serialized field in this table. The report parser recognizes historical arities too, but parser acceptance alone does not establish the evidence required by current validation.

Each observation binds an index, step reference, before-state hash, after-state hash, and events. Events include admission, actor input, hostcall request/decision, runtime outcomes, actor output, and a turn journal. The journal binds policy/capability/budget refs plus event, effect, and receipt refs. A receipt is review evidence, not fresh authorization.

## Artifact and command ownership

The spellings in this table are source-checked, not executed. They are argument forms, not shell recipes; supply actual files and use fresh destinations. [Root declarations](../../../src/main/root/parts/command/p000/body.rs), [harness CLI](../../../src/cli/test/harness.rs), and [report declarations](../../../src/cli/evidence/report/command.rs) own them.

| Surface | Artifact behavior | Important limit |
| --- | --- | --- |
| `molten test run SUITE --report-out FILE` | Writes report on success or failure artifact on handled input/run failure | Destination is not an append-only evidence store |
| `molten test replay REPORT --failure-out FILE` | Replays embedded suite/effects; can write failure evidence | Replay comparison is not the full report-validation entry point |
| `molten test report show REPORT` | Renders a summary; handler also recognizes several failure/receipt shapes | Successful display is not admission or validation |
| `molten test report validate REPORT --failure-out FILE` | Validates report evidence, then replays | Optional failure path records failure, not successful validation receipt |

`harness-failure-v1` carries schema, phase, kind, message, and diagnostics. Suite-associated failures may embed the suite and its ref; report-associated failures may embed the report and its ref. Treat these artifacts as potentially containing the original input, not automatically redacted logs.

## Worked interpretation

Suppose a candidate contains a send request and an explicitly empty capability fixture. A completed `pass` report can correctly contain denied admission and no delivered message. That is different from omitting the capability fixture, which fails runner preparation. Check the observation and invariant rather than converting the report header into “every operation was allowed.”

Budget usage also needs interpretation: events count evidence records, effects count recorded effect entries, and report bytes count canonical serialization, not text file size. Full validation recomputes those quantities and compares the effect log with observed request-response records. Local success does not establish distributed transport, current authority, or release readiness.

## Sources

- [Handbook](../README.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Deterministic playback theory](../../technical/foundations/deterministic-playback-contract.md)
- [Suite structures](../../../src/harness/parts/schema/p000/body.rs)
- [Suite parsing](../../../src/harness/parts/schema/p001/body.rs)
- [Report construction](../../../src/harness/parts/schema/p004/body.rs)
- [Budget and effect-log parsing](../../../src/harness/parts/schema/p029/body.rs)
- [Report validation and replay](../../../src/harness/replay.rs)
