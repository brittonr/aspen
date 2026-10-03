# Diagnosing partial cluster failure

Mode: Troubleshooting

Use this guide when one phase, node, or exported artifact is missing while other parts of a local cluster run look successful. Preserve the original run and state locations before investigating. Do not delete state, replace directories, edit expected refs, or rerun uncertain effects merely to obtain a cleaner result. This is source-checked troubleshooting; none of the scenarios below was executed for this article.

The [technical diagnosis companion](../../technical/operations/receipt-first-cluster-diagnosis.md) explains the evidence graph. Here the working distinction is between an invocation that could not finish, an exported denied run, and a complete run whose copy no longer verifies.

## Symptom: no parent receipt or index was produced

**Discriminating evidence:** identify the last available artifact and the actual command error. Input validation happens before root preparation; fixture planning happens after root preparation. A bad fixture can therefore leave newly created directories without a finished run. Capture and cleanup also use fallible filesystem operations before final artifact writing.

**Safe next action:** preserve the error, fixture identity, supplied paths, and any logs already written. Check whether the timeout was inside the admitted range and whether roots were distinct and previously unused. Check fixture readability and the manifest's header/node spelling through read-only inspection.

**Stop condition:** without an index and canonical parent, do not describe this as a verified run. Failure-bundle generation occurs in finalization, not in an unconditional error handler; absence of a bundle is not proof that nothing happened. Correct the identified prerequisite only after recording the incomplete attempt, then choose new isolated roots for any approved fresh experiment.

## Symptom: fixture-a succeeded but fixture-b failed

**Discriminating evidence:** compare per-phase process receipts, not the last stdout line. The phase loop attempts every node in order and accumulates success. The next phase starts only when the preceding aggregate phase passes. A failed init can therefore yield init observations for both nodes but no start observations.

The [phase implementation](../../../src/cluster_harness/parts/runner/p000/body.rs) records explicit skipped-phase diagnostics. The [node capture code](../../../src/cluster_harness/parts/runner/p003/body.rs) requires the complete per-node artifact set before constructing a successful lifecycle.

**Safe next action:** identify the earliest attempted failing phase and inspect its process receipt plus bound log. Keep success evidence from the other node as partial evidence, not as a cluster pass. Determine whether missing artifacts were never expected because a phase was skipped or were expected from an attempted child.

**Stop condition:** never manufacture missing receipts or treat a later diagnostic message as evidence that the skipped phase ran.

## Symptom: timeout, orphan, or shutdown uncertainty

**Discriminating evidence:** process receipts carry `timed-out`, `orphaned`, and `exit-code`; cleanup records orphan observations and remaining tickets. The shell polls children, attempts termination on timeout/status errors, and collects their output. These are shell observations, not an exactly-once or universal process-tree teardown guarantee.

**Safe next action:** retain the affected state root for authorized process/lifecycle investigation. Correlate actual stop process receipts with shutdown and stop-control artifacts. Ticket cleanup is a separate filesystem pass, not proof that all process effects were reversed.

**Important source-review limit:** normal reverse-order stop is attempted only when the entire start phase passes. Otherwise `is_stop_passed` is set true without stop child invocations; cleanup construction uses that flag when populating stopped node IDs. Consequently, a populated cleanup list alone is not adequate evidence that an individually successful node was stopped after partial startup. This is an inspected control-flow observation, not a reproduced runtime bug.

**Stop condition:** if startup effects or child ownership remain uncertain, withhold the cleanup/pass claim. Do not use an unconditional retry to discover what happened.

## Symptom: offline verification rejects an apparently successful export

**Discriminating evidence:** distinguish content/kind mismatch from noncanonical text, unreadable file, denied artifact, unexpected file, missing required kind, and companion mismatch. A process exit and a file's current integrity answer different questions.

**Safe next action:** use the [export verification procedure](verifying-an-exported-cluster-run.md) and keep its output outside the run. The index points to artifacts; child process evidence explains attempted execution. A first-divergence locator is diagnostic-only and is not a causal ordering of failures.

**Worked packaging case:** the checked-in CLI test adds a newline to `drift-summary.preserves` and expects verification failure. A subsequent Unix case uses a symlink and expects unreadable-artifact denial. Inspecting those tests establishes the intended boundary, not a result from this documentation session. Compare with the preserved original; do not reformat canonical files or regenerate the index by hand.

**Stop condition:** if the original is unavailable, report integrity unresolved. A complete new acquisition is a separate evidence event, not a repair of the original historical claim.

## Symptom: a sealed failure bundle is mistaken for recovery

**Discriminating evidence:** the finalizer labels local process failure bundles `non-replayable-local-process-observation`, sets diagnostic-only, and binds fixture, plan, child, lifecycle, and log refs. The bundle may be well formed while the run remains denied.

**Safe next action:** preserve the bundle with its run and apply the existing privacy/reveal policy before wider distribution. Use it to transfer diagnostic context, not to assert successful replay, cleanup, or remediation. Named failure companions are exempt from ordinary unexpected-file detection; that exemption is not independent bundle admission.

**Stop condition:** do not promote sealed diagnostics into pass evidence. The [distributed testing contract](../../distributed-testing.md) also rejects retry-only release success without a separately accepted remediation boundary.

## When the verifier itself is insufficient

The governing guide states fail-closed intent, but individual helpers have narrower observed behavior. Directory scan errors are ignored by an `if let Ok` branch, and artifact decision parse errors are converted to absence through `.ok().flatten()` in the [observer](../../../src/cluster_harness/parts/runner/p004/body.rs). Do not infer exhaustive semantic validation from a successful directory assessment. Escalate unreadable-tree or malformed-decision concerns with source locations and preserved evidence; these are source-review discrepancies, not demonstrated exploits or confirmed runtime failures.

## Sources

- [Handbook](../README.md)
- [Governing harness guide](../../receipt-first-cluster-harness.md)
- [Technical diagnosis companion](../../technical/operations/receipt-first-cluster-diagnosis.md)
- [Phase gates](../../../src/cluster_harness/parts/runner/p000/body.rs)
- [Cleanup inputs and failure finalization](../../../src/cluster_harness/parts/runner/p001/body.rs)
- [Child execution and deadlines](../../../src/cluster_harness/parts/runner/p002/body.rs)
- [CLI failure and tamper scenarios](../../../tests/parts/cliharness/p018/body.rs)
