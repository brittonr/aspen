# Bounded process execution

Molten's process-execution fabric is an application-owned effect port, not a transport, scheduler, or supervisor. This article follows admission, live execution, output publication, and uncertainty as distinct boundaries. It assumes familiarity with generation-scoped operations and content references. The [fabric execution contract](../../fabric-execution.md) governs this [Technical companion](../README.md).

## Admission precedes path resolution

A process request identifies substantially more than a program. The governing contract binds executable artifact and measurement, executable/process/workspace/effect authority, provenance and policy, extension and callback context, operation and generation, idempotency, explicit arguments and environment, and resource limits. A host path is a locator, not execution permission. Consequently the pure admission step operates on typed authority and resource facts before the shell resolves execution locations.

The [canonical boundary](../../../src/fabric_execution/canonical/mod.rs) makes this separation visible. `admit_request` calls `admit_execution_request` with the admitted profile, request, authority facts, resource grant, and active generation; only an admitted plan is encoded and hashed into `fabric_execution::Request`. Canonicalization identifies the admitted value. It does not independently measure a path or obtain current authority from the operating system.

Before spawning, [live mechanics](../../../src/fabric_execution/mechanics.rs) compare `ResolvedContext` with the plan's executable artifact reference, executable measurement reference, workspace reference, and stdin reference. Executable and workspace paths must be absolute. Stdin reference/byte presence must agree, supplied bytes must hash (BLAKE3 content reference) to the admitted stdin reference, and they must fit the admitted input bound. The [live adapter](../../../src/fabric_execution/live.rs) then opens the resolved executable once without blocking, requires a regular file, and spawns through the same handle (`/proc/<pid>/fd/<n>` on the admitted Linux profile), so replacing the file at the resolved path after measurement cannot substitute a different program. The adapter hashes exactly the length that handle reports against the plan's executable identity and remembers, per identity, the handle's `fstat` stamp (device, inode, size, modification and change times). A later execution whose handle has the same stamp reuses that measurement; a rename, replacement, size change, or metadata change alters the stamp and forces a full re-hash, and a stamp that changes during hashing is refused. A same-size write within one timestamp tick of a coarse-grained filesystem can leave the stamp unchanged; excluding writers is the resolver capability's job (see Limits). A measurement mismatch is a `ResolutionDenied` failure and an unopenable, non-regular, or concurrently changed executable is a definite pre-start failure; neither spawns anything.

### Resolved discrepancy: reference equality was not measurement

Earlier revisions only compared the resolver's *claimed* stdin and executable references with the plan: substituted stdin bytes or a different file at the executable path still ran. The live boundary now rehashes stdin bytes and measures the opened executable itself, and refuses to re-execute recorded work (see below). Regression coverage lives in the [binding tests](../../../src/fabric_execution/tests/binding.rs) and the unknown-retry case in the [stdin tests](../../../src/fabric_execution/tests/stdin.rs).

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

If instead a post-start I/O error prevents a definitive terminal observation, the adapter records unknown status. `reconcile` requires the exact operation reference and generation; generation 8 does not retrieve generation 7's status. Unknown work does not automatically retry under the governing contract, and the live adapter enforces this locally: a second `execute` for an `(operation_ref, generation)` already recorded as `Unknown` or `Terminal` fails with `PortFailureKind::DuplicateOperation` before resolution, measurement, or spawning, and the recorded status is kept. Only a definite pre-start failure may be executed again. An idempotency identity or an in-memory operation-status map does not establish exactly-once execution, durable crash recovery, or duplicate suppression across independent invocations or adapter instances.

## Review and verification guidance

Follow the full identity tuple from request admission through resolver output and completion admission. Review failures by the strongest observation actually available: definite pre-start refusal, terminal process observation, failed publication with terminal evidence, or unresolved post-start outcome. Do not collapse these into a generic retryable error.

The deterministic adapter described in the governing document consumes canonical requests and publication contracts without spawning processes. It is appropriate for reasoning about timeout, cancellation, truncation, and unknown completion transitions. It cannot validate actual Unix teardown or child I/O behavior. A live verification session should separately exercise those shell effects and inspect both retained stream counts and publication results. No runtime checks were executed for this documentation-only article.

## Limits and non-claims

The adapter's operation map, keyed by exact `(operation_ref, generation)`, and its measurement cache are local runtime bookkeeping, not a durable distributed reconciliation service. Measurement binds the bytes of the opened file at measurement time; an in-place write to that same file by a holder of write access after measurement and before exec is outside this check, and the resolver's capability is responsible for excluding writers. A receipt describes one bounded observation, not executable trust, application success, platform equivalence, or release readiness. Current authority and resource facts remain application-owned inputs; content identity never supplies them implicitly.

## Sources

- [Bounded execution fabric contract](../../fabric-execution.md)
- [Canonical request and receipt boundary](../../../src/fabric_execution/canonical/mod.rs)
- [Resolved-context checks and request construction](../../../src/fabric_execution/mechanics.rs)
- [Live adapter failures and reconciliation](../../../src/fabric_execution/live.rs)
- [Technical companion](../README.md)
