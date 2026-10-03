# Bounded process execution

Molten's process-execution fabric is an application-owned effect port, not a transport, scheduler, or supervisor. This article follows admission, live execution, output publication, and uncertainty as distinct boundaries. It assumes familiarity with generation-scoped operations and content references. The [fabric execution contract](../../fabric-execution.md) governs this [Technical companion](../README.md).

## Admission precedes path resolution

A process request identifies substantially more than a program. The governing contract binds executable artifact and measurement, executable/process/workspace/effect authority, provenance and policy, extension and callback context, operation and generation, idempotency, explicit arguments and environment, and resource limits. A host path is a locator, not execution permission. Consequently the pure admission step operates on typed authority and resource facts before the shell resolves execution locations.

The [canonical boundary](../../../src/fabric_execution/canonical/mod.rs) makes this separation visible. `canonical_admit_execution_request` calls `admit_execution_request` with the admitted profile, request, authority facts, resource grant, and active generation; only an admitted plan is encoded and hashed into `CanonicalExecutionRequest`. Canonicalization identifies the admitted value. It does not independently measure a path or obtain current authority from the operating system.

Before spawning, [live mechanics](../../../src/fabric_execution/mechanics.rs) compare `ResolvedExecutionContext` with the plan's executable artifact reference, executable measurement reference, workspace reference, and stdin reference. Executable and workspace paths must be absolute. Stdin reference/byte presence must agree, and supplied bytes must fit the admitted input bound. These comparisons are a boundary check on the resolver's result, not a claim that this function itself rehashes executable or stdin bytes. Review the source of resolved facts rather than promoting equality of references into a fresh measurement.

## An explicit process envelope

`bounded_request` creates the `bounded_exec::RunRequest` from admitted fields. It sets `EnvironmentMode::Clear`, adds only the supplied entries, uses an explicit current directory, and passes arguments without introducing a shell. The governing live profile rejects inheritance, path search, implicit current directories, duplicate environment keys, secret environment values, and shell-mode requests. Those restrictions avoid treating the parent process's convenience state as application authority.

The process envelope includes timeout, input and retained-output byte bounds, polling interval, teardown timeout, exit-code policy, and truncation policy. Integer conversions into host-sized byte bounds can fail before start. Termination scope maps explicitly to child or process-group behavior; the first reviewed live profile uses Unix process groups. A direct-child termination observation would not justify a claim about all descendants.

Bounds are not equivalent to containment. They constrain the admitted execution and observation protocol. Neither an explicit environment nor process-group teardown establishes network isolation, sandboxing, hermetic filesystem access, or correctness of child behavior. Those non-claims are preserved by the governing contract.

## Completion and publication are different outcomes

The [live adapter](../../../src/fabric_execution/live.rs) distinguishes a `bounded-exec` spawn or request failure from errors after a process may have started. Known pre-start failures become `DefinitePreStartFailure`; other run errors become `Unknown`. Successful process observations are converted into lifecycle and disposition facts, including whether the observed exit policy accepted the result.

Each output stream has observed-byte count, retained-byte count, truncation, and a retained prefix. The prefix is published through `ExecutionOutputPublisher`; the canonical receipt carries publication references rather than raw stream bytes. Retention therefore answers what remains available, while the observed-byte count describes the broader stream observation. A truncated prefix is not the complete output simply because its own content identity verifies.

Publication failure occurs after a process observation may already be complete. `publish_and_record` attempts both streams, constructs the canonical receipt, records its terminal reference, and returns an `OutputPublication` failure when either publication failed. That failure retains the process observation and receipt. Treating it as “the command never ran” would discard exactly the evidence needed for safe application handling.

## Illustrative recovery scenario

Consider operation `report-export`, generation 7, whose child exits with a policy-accepted code after writing more stdout than the retained bound. This is an illustrative scenario. The request's truncation policy determines the observed disposition; an accepted exit code alone is insufficient to declare application success. Now suppose publishing the retained stdout prefix fails. The port reports a typed publication failure with process evidence, not a license to rerun the export.

If instead a post-start I/O error prevents a definitive terminal observation, the adapter records unknown status. `reconcile` requires the exact operation reference and generation; generation 8 does not retrieve generation 7's status. Unknown work does not automatically retry under the governing contract. An idempotency identity or an in-memory operation-status map does not establish exactly-once execution, durable crash recovery, or duplicate suppression across independent invocations.

## Review and verification guidance

Follow the full identity tuple from request admission through resolver output and completion admission. Review failures by the strongest observation actually available: definite pre-start refusal, terminal process observation, failed publication with terminal evidence, or unresolved post-start outcome. Do not collapse these into a generic retryable error.

The deterministic adapter described in the governing document consumes canonical requests and publication contracts without spawning processes. It is appropriate for reasoning about timeout, cancellation, truncation, and unknown completion transitions. It cannot validate actual Unix teardown or child I/O behavior. A live verification session should separately exercise those shell effects and inspect both retained stream counts and publication results. No runtime checks were executed for this documentation-only article.

## Limits and non-claims

The adapter's operation map is local runtime bookkeeping, not a durable distributed reconciliation service. A receipt describes one bounded observation, not executable trust, application success, platform equivalence, or release readiness. Current authority and resource facts remain application-owned inputs; content identity never supplies them implicitly.

## Sources

- [Bounded execution fabric contract](../../fabric-execution.md)
- [Canonical request and receipt boundary](../../../src/fabric_execution/canonical/mod.rs)
- [Resolved-context checks and request construction](../../../src/fabric_execution/mechanics.rs)
- [Live adapter failures and reconciliation](../../../src/fabric_execution/live.rs)
- [Technical companion](../README.md)
