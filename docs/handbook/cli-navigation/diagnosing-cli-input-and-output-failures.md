# Diagnosing CLI input and output failures

Mode: Troubleshooting

Treat a failed invocation as a sequence of boundaries: parser, file access, decoding, typed admission, planning or execution, artifact publication, and presentation. The first discriminating fact is which boundary was reached. Do not infer that nothing happened from a nonzero exit or that all effects completed from a file's existence.

This guide is source-checked; its failure cases are source-review cases and existing test intentions, not newly reproduced failures. No commands or tests were executed. Return to the [Handbook](../README.md) for task routes; the [preview-first companion](../../technical/world-effects/preview-first-operator-composition.md) explains why planning and applying have separate obligations.

## Symptom: an expected flag or subcommand is rejected

**Discriminating evidence.** Compare the exact nesting with [root declarations](../../../src/main/root/parts/command/p000/body.rs), then resolve the alias through [main.rs](../../../src/main.rs). For example, the [world argument structs](../../../src/cli/runtime/world.rs) require `--out` for `world plan`, but `--plan-out` for single-operation variants. Fabric-time `show` takes a positional path, not a `--report` flag.

**Safe next action.** Correct only the command shape, retaining the same evidence input. Check the parser's variant and the called handler together. The root [parser tests](../../../src/main/root/parts/command/p001/body.rs) demonstrate acceptance and required arguments, but their made-up pathnames and repeated-character references are parsing fixtures, not runnable operational evidence.

**Stop condition.** If the checkout lacks the variant, do not invent an alias or substitute a more powerful verb. Establish the intended source cohort before continuing.

## Symptom: configuration or request parsing fails

**Discriminating evidence.** `runtime config` reads a file and calls `RuntimeStartupConfig::from_nickel_export_json` in the [root handler](../../../src/main/root.rs). Its input is exported JSON; the handler does not evaluate a raw `.ncl` source. World requests use [JSON deserialization](../../../src/cli/runtime/world/document.rs) with `deny_unknown_fields`, followed by typed-reference and closed-vocabulary conversion.

**Safe next action.** Inspect the input's format and producer before changing fields. Distinguish unreadable file, malformed JSON, unknown field, invalid reference, and unsupported operation kind. The [world tests](../../../src/cli/runtime/world/tests.rs) separate an unknown `raw_command` field from a shape-valid but unsupported operation kind.

**Stop condition.** Do not replace references with syntactically valid filler, add arbitrary fields, or loosen admission. If the producer/consumer contract cannot be established, preserve the original bytes and stop.

## Symptom: a valid request fails under a specific world verb

**Discriminating evidence.** `require_one_operation` requires exactly one operation of the matching kind. The [logical fixture](../../../tests/fixtures/world-operator/logical/request.json) is a thirteen-operation graph. Supplying it to inspect is therefore different from asking the planner to inspect its first operation.

**Safe next action.** Use graph planning when assessing the whole fixture. For a real single-operation request, obtain a properly scoped request from its owner; do not erase dependencies merely to satisfy cardinality. The existing checkpoint denial test deliberately constructs a reduced test request, but that manipulation is not a general operational recipe.

**Stop condition.** A missing dependency or mismatched operation is an input-contract problem, not grounds for bypassing the planner.

## Symptom: output files exist although the command failed

**Discriminating evidence.** [World output publication](../../../src/cli/runtime/world/output.rs) writes plan, optional receipt, and optional summary sequentially. The mutation handler publishes these before checking an apply request's required receipt destination. The denial writer can subsequently replace the receipt and return an error. [main.rs](../../../src/main.rs) prints returned errors and exits with status 1.

**Safe next action.** Preserve the complete partial artifact set, input, and terminal diagnostics. Identify the last completed write from the handler order; do not declare the set atomic. Missing parent directories are also significant: this world writer does not create them. Select a new isolated destination for a corrected planning attempt rather than overwriting the evidence under investigation.

**Stop condition.** Never infer rollback from exit status. For an uncertain live effect, use the owning subsystem's reconciliation boundary rather than rerunning a mutation.

## Symptom: apply produces a denial instead of live work

**Discriminating evidence.** The standalone world CLI has no live component handler registry. Its denial writer selects `HandlerUnavailable` when the submitted reference matches the plan and `StalePlan` otherwise. Both paths return an error without performing the world mutation.

**Safe next action.** Read the [workflow contract](../../world-operator-workflows.md) and distinguish this intentional composition limit from a transient transport failure. A matching preview does not supply handlers or fresh facts.

**Stop condition.** Do not retry with another fabricated reference, remove current-admission checks, or interpret a planning receipt as authority.

## Symptom: a time artifact cannot be shown

**Discriminating evidence.** The [time reader](../../../src/cli/runtime/fabric_time/ops.rs) checks metadata size, reads UTF-8 text, parses Preserves, and decodes a run report. An individual event is the wrong record type; the [fixture tests](../../../src/fabric_time/parts/tests/p000/body.rs) explicitly reject one as a report.

**Safe next action.** Locate the enclosing report and retain the event for its own consumer. Do not assume every `.preserves` file is text: world outputs write canonical bytes while the time fixture uses text rendering.

**Stop condition.** Do not strip fields, raise bounds, or convert unknown bytes until the producing schema and encoding are established. A successful readback still does not prove live clock health, remote deadlines, or release readiness.

## Sources

- [Handbook](../README.md)
- [Preview-first operator composition](../../technical/world-effects/preview-first-operator-composition.md)
- [World workflow contract](../../world-operator-workflows.md)
- [World argument handling](../../../src/cli/runtime/world.rs)
- [World output publication](../../../src/cli/runtime/world/output.rs)
- [Fabric-time readback](../../../src/cli/runtime/fabric_time/ops.rs)
- [Logical request fixture](../../../tests/fixtures/world-operator/logical/request.json)
