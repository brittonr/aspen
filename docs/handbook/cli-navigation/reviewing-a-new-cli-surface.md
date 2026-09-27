# Reviewing a new CLI surface

Mode: Review checklist

Use this checklist to accept or reject a proposed CLI change with concrete evidence. It is not a demand that every command become a live adapter, nor permission to weaken an unavailable path until a demo works. A source-only planning surface is acceptable when its limits are explicit. The [Handbook](../README.md) gives practical navigation routes; the [technical companion](../../technical/world-effects/preview-first-operator-composition.md) supplies the planning and authority model.

This checklist was prepared by source inspection. No runtime or test execution is claimed. Reviewers applying it must distinguish source evidence, existing tests, and observed execution in their own review record.

## Parser and route acceptance

- [ ] Can the author name the complete user-facing route, including required parent families? Attach the exact enum and argument struct rather than an approximate help transcript.
- [ ] Does the trace resolve every alias and include file? Use [main.rs](../../../src/main.rs), [root dispatch](../../../src/main/root.rs), and the [command include module](../../../src/main/root/command.rs) as the pattern. A declared variant without a traced handler is incomplete evidence.
- [ ] Are required inputs genuinely required at the appropriate boundary? Inspect positional versus named arguments, enum values, and defaults. If a requirement depends on another flag, identify whether Clap or the handler checks it and whether output can already exist when it fails.
- [ ] Do existing parser tests establish only parsing claims? Reject operational examples that borrow fake references or nonexistent fixture pathnames from parsing-only tests.
- [ ] Have all exposed routes been considered? The existing `node` and `test node` variants share a handler, so changing that handler affects both namespaces.

Evidence sufficient for this section is a declaration-to-dispatch-to-handler chain plus explicit input shapes. A screenshot of help alone is insufficient.

## Input and authority acceptance

- [ ] Is each input's representation clear: raw Nickel, exported JSON, Preserves text, canonical binary, or an owned store entry? The suffix alone must not decide decoding.
- [ ] Is the owning schema documented, including unknown-field handling and closed vocabularies? [World document decoding](../../../src/cli/runtime/world/document.rs) offers a concrete boundary to inspect.
- [ ] Are identity checks separate from current authorization? Preserves plus BLAKE3 define canonical identity; neither Rust layout nor possession of a receipt confers authority.
- [ ] For stateful commands, does the public path become the reviewed capability root rather than an ambient descendant pathname? Compare the [node-state contract](../../node-state-filesystem-authority.md); do not infer its protections automatically apply to unrelated CLI writers.
- [ ] Can fixture facts be distinguished from live observations? Reject descriptions that relabel checked-in admitted flags as a deployment's current admission.

A reviewer should be able to point to the exact point where untrusted input becomes typed data and the later point, if any, where fresh authority is checked. Intended architecture is not proof every helper enforces it.

## Effects and artifact acceptance

- [ ] Has the author enumerated actual shell effects, including clock, entropy, file, process, and transport access? A profile called deterministic does not automatically suppress live effects.
- [ ] Is the command honestly classified as a fixture, planner, reader, or live operation? [Fabric-time execution](../../../src/fabric_time/parts/fixture/p000/body.rs) runs both adapters before selecting report data; [world CLI routing](../../../src/cli/runtime/world.rs) plans even for read-named verbs.
- [ ] Are output destination kind, encoding, ownership, overwrite behavior, and parent-directory requirements stated? Compare [world writes](../../../src/cli/runtime/world/output.rs) with [fabric-time artifact planning](../../../src/cli/runtime/fabric_time/ops.rs).
- [ ] Does a failure after partial publication preserve an intelligible evidence boundary? Do not claim transactional output unless the writer implements it.
- [ ] Are secret-bearing input values excluded from proposed summaries and review attachments? Stable references can support diagnosis without exposing credentials.
- [ ] Are uncertain effects terminal until component-owned reconciliation? Reject unconditional retries, deletion of state as repair, and admission bypasses.

## Worked rejection case: a misleading apply tutorial

Consider a proposed guide that previews a world checkpoint, then presents the matching plan reference as sufficient to execute it through the standalone CLI. The parser surface exists, so a shallow review might accept the guide.

Trace `plan_mutation` instead. It reads and plans the request, writes planning outputs, requires a receipt destination for apply, then calls `write_apply_denial`. That writer returns `HandlerUnavailable` for a matching reference. The [governing contract](../../world-operator-workflows.md) requires an embedding with reviewed handlers and fresh-facts adapters. The tutorial's execution claim must therefore be rejected, not “fixed” by changing references.

An acceptable revision describes preview and denial as the available CLI behavior, names the missing composition boundary, and links the [existing denial test](../../../src/cli/runtime/world/tests.rs). It does not claim that test ran during documentation review or that the embedding is provided by the example.

## Evidence and delivery acceptance

- [ ] Does any executable recipe link adjacent inspected declarations and explicitly state whether it ran? Inputs must be real fixtures or guarded operator-provided evidence, not invented authority references.
- [ ] Does exercised verification demonstrate consumer-visible behavior, including meaningful negative or boundary cases, rather than merely a parser accepting a string?
- [ ] Are existing tests cited with their actual scope? The retained world fixture test compares canonical record bytes; it does not execute every world component.
- [ ] Are non-claims explicit? Fixture results do not establish production readiness, exactly-once effects, or distributed agreement. Raft references remain control-plane scoped, with no OpenRaft inference.
- [ ] Can another maintainer identify what is available, what is intentionally denied, and what prerequisite blocks further use without guessing?

Accept the surface only when its user-visible promise matches the traced implementation and supplied evidence. Missing live composition is a documented limit, not permission to advertise a narrower planner as completed execution.

## Sources

- [Handbook](../README.md)
- [Preview-first operator composition](../../technical/world-effects/preview-first-operator-composition.md)
- [World workflow contract](../../world-operator-workflows.md)
- [Node-state filesystem authority](../../node-state-filesystem-authority.md)
- [Root parser and variants](../../../src/main/root/parts/command/p000/body.rs)
- [World CLI and apply boundary](../../../src/cli/runtime/world.rs)
- [World fixture and denial tests](../../../src/cli/runtime/world/tests.rs)
