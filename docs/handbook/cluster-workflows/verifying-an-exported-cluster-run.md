# Verifying an exported cluster run

Mode: How-to

## Goal and prerequisites

Determine whether a received receipt-first cluster run is internally consistent and appropriate for a local integration review, without restarting its children or modifying its evidence. You need the complete exported directory, a compatible source-checked Molten binary or build environment, and the producer's source revision and claimed evidence scope. No running node or original mutable state root is required by the verifier.

This procedure is source-checked, not executed. It complements the [technical explanation of receipt-first diagnosis](../../technical/operations/receipt-first-cluster-diagnosis.md); it does not certify an archive's provenance or turn verification into authority.

## 1. Separate acquisition from assessment

Keep the producer's original export unchanged. If unpacking is necessary, use your organization's reviewed archive-handling process into a fresh isolated location. Do not unpack an untrusted archive over source, node state, or another run. This article does not prescribe an archive format: the cluster CLI declares run and verify operations, not a cluster archive-export command.

Before invoking the verifier, review the directory boundary and index as untrusted input. The core rejects unsafe relative paths, but the shell constructs file observations before the core assessment. That source ordering is not a demonstrated pre-read sandbox. Likewise, a leaf-file non-symlink check is not a general claim about every ancestor path. If safe containment is uncertain, stop and use an appropriately isolated review environment.

Obtain the original fixture separately if you need to compare it with the intended experiment. `fixture-metadata.preserves` binds a fixture ref and ordered nodes; the run writer does not copy the original `.cluster` file into the directory. A matching-looking filename is not evidence of matching content.

## 2. Check that this is the right artifact family

Look for `artifact-index.tsv`, whose header is `molten.cluster-run-index.v1`, and the verification companion `verification.preserves`. The [directory reference](cluster-run-directory-reference.md) explains the expected entries. Do not feed a VM output tree, simulation repro, or distinct-process transport directory to this procedure merely because it also contains receipts.

Keep review notes, screenshots, unpacking reports, and new command logs outside the run directory. The verifier detects unexpected files, with only specific companion exceptions. Adding a README inside a sealed directory is not harmless annotation from the verifier's perspective.

If the index is missing or malformed, retain that acquisition result. Ask the producer for a complete original export through the normal evidence channel; do not reconstruct the index from filenames.

## 3. Run the offline verifier once on the preserved input

Set `CLUSTER_RUN_DIR` yourself to the reviewed directory. The guard below prevents an empty variable becoming an unintended path. The command is source-checked, not executed; provenance is the [CLI declaration and handler](../../../src/cli/ops/parts/cluster/p000/body.rs), its [main alias](../../../src/main.rs), and the [offline CLI scenario](../../../tests/parts/cliharness/p018/body.rs).

```sh
cargo run -- cluster harness-verify \
  --run-dir "${CLUSTER_RUN_DIR:?Set CLUSTER_RUN_DIR to the reviewed export}"
```

The handler reports a decision, index ref, and verification ref, and returns an error for denial. It has no receipt-output option. The underlying verifier computes a receipt value but does not rewrite `verification.preserves`. Preserve the command's actual result outside the inspected directory; do not mistake its rendered summary for a newly exported canonical file.

A build or dependency failure is not an integrity verdict. Record it as verification unavailable and retain the input. Development dependency substitutions do not establish release provenance.

## 4. Branch on the evidence, not the desired answer

**Assessment passes:** confirm that the producer's claimed profile remains local multiprocess. Review the parent, child lifecycle receipts, and cleanup alongside the verifier result. A matching content graph cannot independently prove who produced it, current authorization, or successful live networking.

**Assessment denies:** use diagnostics to distinguish missing/unreadable artifacts, canonical-text differences, ref or kind mismatch, ineligible decisions, missing kinds, unexpected files, and companion mismatch. The first-divergence structure exists in the canonical verification representation, but this CLI handler does not render its full fields. Do not invent a show/export flag. The [partial-failure guide](diagnosing-partial-cluster-failure.md) identifies the next artifacts to inspect.

**Verifier cannot parse or read the input:** preserve the error and stop pass review. This is different from a complete assessed denial receipt; the API can return an error before constructing one.

## 5. Work through a packaging failure

The [checked-in tamper scenario](../../../tests/parts/cliharness/p018/body.rs) appends a newline to `drift-summary.preserves` after a successful fixture run and expects offline denial. It also replaces that artifact with a symlink on Unix and expects unreadable-artifact denial. These are inspected test cases, not results produced here.

The operator lesson is specific: a packaging tool that reformats Preserves can invalidate canonical text even when a parser accepts the value. Compare against the preserved producer export, not a hand-edited index. If transfer corruption is established, acquire a fresh complete copy under a new location and document its relationship to the original. Never change the companion or expected refs simply to obtain pass.

## 6. Record the bounded conclusion

Record source revision, acquisition identity, actual verifier result, unresolved diagnostics, claimed profile, and whether child/cleanup evidence supports that claim. Keep failure-bundle privacy review separate: named failure companions are allowed unindexed files and are not thereby independently validated by ordinary directory verification. Local process failure bundles remain non-replayable diagnostic evidence unless a separate recorded-effect basis exists.

## Sources

- [Handbook](../README.md)
- [Governing harness guide](../../receipt-first-cluster-harness.md)
- [Technical diagnosis companion](../../technical/operations/receipt-first-cluster-diagnosis.md)
- [Verifier and companion comparison](../../../src/cluster_harness/parts/runner/p002/body.rs)
- [Observation ordering and canonical text checks](../../../src/cluster_harness/parts/runner/p004/body.rs)
- [Allowed companion files and file reads](../../../src/cluster_harness/parts/runner/p005/body.rs)
- [CLI verification and tamper scenarios](../../../tests/parts/cliharness/p018/body.rs)
