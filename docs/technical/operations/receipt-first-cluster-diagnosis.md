# Receipt-first cluster diagnosis

The receipt-first cluster harness makes an isolated local multiprocess run reviewable after its children have stopped. This article assumes familiarity with canonical Preserves and the [harness contract](../../receipt-first-cluster-harness.md). It explains how to diagnose a run directory without replacing artifact verification with confidence in terminal output. Related material is listed in the [Technical companion](../README.md).

## The run directory is an evidence graph

A console transcript records what a process printed. A run directory additionally states which artifacts belong to the run, what kinds they have, how they are encoded, and which content refs are expected. The harness includes fixture metadata, command and process plans, child-process and node lifecycle receipts, a local executable-run receipt, the parent cluster receipt, drift and cleanup artifacts, diagnostic logs, and offline verification.

`artifact-index.tsv` is the entry point. The [index implementation](../../../src/cluster_harness/parts/runner/p004/body.rs) serializes each entry as four tab-separated fields: relative path, artifact kind, expected ref, and format. Its parser checks the header, field count, and entry bound. This means a path alone is not the identity of evidence. Substituting bytes at an unchanged path can change a content ref, canonicality, kind, or decision.

The shell observes files and constructs observations; deterministic assessment is delegated to the core. This separation makes offline review possible without pretending that file reads are pure or that process execution can be reconstructed from a receipt alone. The [governing contract](../../receipt-first-cluster-harness.md) specifies fail-closed handling of missing files, traversal, malformed or non-canonical values, kind drift, ref mismatches, denied children, missing required kinds, unexpected files, and stale verification companions.

## Canonicality is distinct from parseability

For indexed Preserves artifacts, the inspected shell checks that the path identifies a regular non-symlink file, reads the text, and parses it. It then computes the canonical hash, determines the actual artifact kind, and compares canonical rendering with the original text. A parseable alternative spelling can therefore fail the canonical-text check even if it denotes the same value.

Pass eligibility is another independent observation. If an artifact exposes a decision, a non-pass decision does not become acceptable simply because the value is canonical and its ref matches. Conversely, diagnostic text can be successfully read and content-bound without becoming canonical operational pass evidence. The classification of child logs as adjunct text is essential to this distinction.

The directory scan also detects unindexed files, with explicit exceptions for the index, verification companion, and failure-bundle companion files. Those named exceptions in the [runner helper](../../../src/cluster_harness/parts/runner/p005/body.rs) are not a general permission to add arbitrary explanatory files to a sealed run directory.

## The verification companion prevents stale summaries

`verify_cluster_run_directory` reads the index and computes an assessment from the current directory. It then derives the expected verification receipt. When the indexed assessment passes, the existing `verification.preserves` companion must parse and equal that expected value. A missing or mismatched companion changes the decision to deny and records a diagnostic-only first divergence. See the [offline verification implementation](../../../src/cluster_harness/parts/runner/p002/body.rs).

This avoids treating yesterday's verification summary as evidence about today's files. It also means diagnosis should preserve the original directory rather than editing a suspicious artifact until the display looks right. A successful regeneration is a new evidence event, not a retroactive correction of the historical run.

## Worked failure: a readable receipt has changed

Consider an illustrative two-node run whose parent receipt and child logs report success. During packaging, someone edits a child's receipt text for readability. The file still parses, but its spelling differs from canonical rendering. Verification can reject canonicality even before an operator considers whether its semantic content changed.

If the edit instead changes the decision or a bound ref, the canonical hash or pass eligibility can fail as well. Updating the TSV by hand is not a valid repair: it changes the index identity, and the old verification companion no longer describes the recomputed assessment. More importantly, arbitrary editing does not establish that the changed artifact was produced by the original child execution.

The productive diagnosis is to compare the earliest reported divergence with the indexed kind, expected ref, and observed file, then inspect the corresponding lifecycle or child-process evidence. The first-divergence field is a locator, not a proof of root cause. An early missing receipt may result from a timeout, a denied startup, or an interrupted copy; the supporting artifacts distinguish those possibilities.

## Cleanup, privacy, and review procedure

The harness attempts bounded cleanup after failures and records timeout, orphan, and ticket observations. A failure bundle can bind the fixture, plan, receipts, logs, redaction evidence, and non-replayable local observations. The [failure-bundle regression source](../../../src/cluster_harness/tests.rs) explicitly treats such a bundle as canonical diagnostic evidence rather than pass evidence. Private attachments still need the existing reveal and redaction policy before export.

Suggested verification follows the documented `cluster harness-run` and `cluster harness-verify` commands using isolated state and run roots. The implementation rejects zero or over-limit child timeouts and overlapping roots. Existing roots are rejected unless force is selected; force replaces those directories, so it is inappropriate for preserving a failing run under investigation. Retain the run first and choose separate roots for a new exercise.

These are review suggestions, not commands executed for this article. A passing local-process run is not VM, live-network, consensus, or production evidence. Offline integrity establishes the declared artifact relationships; it does not grant authority or prove that every external environmental claim is true. Production review additionally follows the [operator runbooks](../../production-operator-runbooks.md), including candidate-bound source-gate evidence.

## Sources

- [Receipt-first cluster harness](../../receipt-first-cluster-harness.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Offline verification and root admission](../../../src/cluster_harness/parts/runner/p002/body.rs)
- [Index and artifact observation implementation](../../../src/cluster_harness/parts/runner/p004/body.rs)
- [Verification companion and file helpers](../../../src/cluster_harness/parts/runner/p005/body.rs)
- [Canonical parent and failure-bundle tests](../../../src/cluster_harness/tests.rs)
- [Technical companion](../README.md)
