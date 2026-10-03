# Inspecting a transcript and rewrite recipe

Mode: How-to

## Goal and prerequisites

Determine whether a proposed structural rewrite has an inspectable scope and relevant transcript evidence before allowing artifact installation or name movement. Have the candidate transcript file, rewrite plan or receipt, exact registry provenance, and the intended old/new artifact mapping. Use existing evidence first. This article is source-checked, not executed; command examples assume an available `molten` binary and operator-supplied file paths.

The [architecture](../../architecture.md) governs canonical identity and admitted effects. The [upgrade and migration companion](../../technical/extensions/upgrade-quarantine-and-migration.md) explains why preserving old bytes is not enough to establish reversible state migration. Here the practical decision is narrower: what can the local transcript and rewrite surfaces demonstrate?

## 1. Inspect without running or applying

The [root command declaration](../../../src/main/root/parts/command/p000/body.rs), [transcript declaration](../../../src/cli/core/transcript/command.rs), and [rewrite declaration](../../../src/cli/runtime/rewrite/command.rs) establish these spellings. The [show handlers](../../../src/cli/core/transcript/ops.rs) and [rewrite operations](../../../src/cli/runtime/rewrite/ops.rs) read input rather than apply it. Source-checked; not executed:

```sh
molten test transcript show "${TRANSCRIPT_FILE:?Set the reviewed transcript path}"
molten test rewrite show "${REWRITE_FILE:?Set the reviewed rewrite artifact path}"
```

The variables intentionally contain no example authority references. `transcript show` accepts a canonical transcript artifact or falls back to Markdown parsing with empty parse bindings. Successful display alone therefore does not establish admission bindings or successful execution. `rewrite show` produces a summary, not a complete semantic review; inspect the underlying canonical value as well.

If the input is Markdown, distinguish prose, executable stanzas, and expectations. Do not turn arbitrary fenced shell examples into transcript steps. The restricted interpreter requires `molten-cli` stanzas to start with `test` and supports only its implemented artifact, schema, storage, cache, and report subsets. It is not the general CLI or an ambient shell.

## 2. Decide what each expectation actually checks

Read stanza order and modifiers before accepting the aggregate decision. The [stanza runner](../../../src/transcripts/parts/mod/p002/body.rs) treats `skip` as skipped, records `bug` as known-bug, and makes `error` succeed when execution returns an error. A passing expected-error stanza demonstrates the tested denial, not success of the underlying operation. Binding denial occurs before ordinary execution handling.

Use the checked-in [transcript tests](../../../src/transcripts/parts/mod/p005/body.rs) as concrete examples. `fresh_runs_are_deterministic_across_temp_roots_and_render_hides_output` uses a hidden Preserves value followed by an exact-output expectation. `restricted_cli_installs_artifact_and_matches_receipt_expectations` tests a local artifact operation with admitted parse input. Neither is a live application upgrade.

Ask whether the proposed expectation binds the relevant output, decision, receipt kind, or reference. A rendered document is presentation, not a substitute for the run receipt. The CLI render path reconstructs a run with an empty stanza-outcome list when given a receipt, so do not assume rendering restores the original detailed observations.

## 3. Choose execution evidence deliberately

If new evidence is required, arrange an approved isolated run rather than executing from the inspection commands above. The runner offers fresh state and saved state; fork and in-place modes deny by default. Supplying a cache can return cached evidence before allocating fresh runner state. Consequently, the label `fresh` alone is insufficient to establish that work was recomputed when caching is enabled.

For a review record, retain the transcript identity, dependencies, handler profile, policy and capability bindings, mode, cache usage, and receipt. This article supplies no runnable mutation recipe because those admitted inputs belong to the operator's case. Missing bindings are a stop condition, not a reason to invent local content references.

## 4. Inspect structural scope before approving a recipe

The [rewrite implementation](../../../src/rewrites/parts/mod/p000/body.rs) supports several query patterns, but its replacement type is specifically `StringValue { from, to }`. Do not describe it as an arbitrary schema transformer or source-code refactoring engine.

The worked [rewrite fixture](../../../src/rewrites/parts/mod/p004/body.rs) starts with `<doc "old" ["old" "keep"]>`, selects string equality for `old`, previews changes, and applies a replacement with `new`. It asserts that the old payload remains available and the new artifact has a different reference. Review changed paths and old/new payload references, not just a textual preview containing the new word.

Verify roots, dependency inclusion, artifact-kind filters, and hidden references. A visibility-filtered result is not proof that no other dependent exists. Empty diffs produce a denied preview; broadening the scope merely to obtain a pass defeats the review.

## 5. Stop at the upgrade integration boundary

`apply` recomputes a preview and installs replacement artifacts; it does not consume a previously approved plan file as an immutable execution contract. Preserve the actual applied mapping and reconcile it against what was reviewed.

The [upgrade-plan hook](../../../src/rewrites/parts/mod/p001/body.rs) creates installation tasks and a transcript task, but uses synthetic source-gate evidence. The [CLI](../../../src/cli/runtime/rewrite/ops.rs) also derives local policy, capability, transcript, and migration references. These are source-review limitations, not observed failures. Do not present that path as release admission or as proof that a migration or replay happened. Require independently resolved evidence before a real cutover review.

## Sources

- [Handbook](../README.md)
- [Architecture](../../architecture.md)
- [Migration and rollback theory](../../technical/extensions/upgrade-quarantine-and-migration.md)
- [Transcript CLI input handling](../../../src/cli/core/transcript/io.rs)
- [Transcript execution](../../../src/transcripts/parts/mod/p001/body.rs)
- [Rewrite preview and types](../../../src/rewrites/parts/mod/p000/body.rs)
- [Rewrite application and upgrade hook](../../../src/rewrites/parts/mod/p001/body.rs)
