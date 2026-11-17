# Pure Interaction-Net Evaluation Service

An opt-in deterministic sequential workload over the existing standalone attempt lifecycle. This specification does not select the service by default, qualify a parallel backend, implement portable checkpoints, or claim production/cross-host readiness.

## ADDED Requirements

### Requirement: The workload is selected and installed explicitly

r[molten.pure_net.profile] Molten MUST expose a versioned `pure-net-eval/v1` workload manifest through the existing standalone controller/worker operator path, MUST keep native grants separate from pure-component grants, and MUST NOT silently select the workload or a parallel/checkpoint backend.

#### Scenario: An operator installs and submits an admitted workload
- GIVEN an explicitly installed manifest and matching native execution grants
- WHEN the supported service path receives a checked pure program
- THEN it MAY execute through the selected existing attempt composition and MUST retain the manifest and result receipt.

#### Scenario: Only component or unspecified grants exist
- GIVEN no matching pure-net manifest/program grant
- WHEN execution is requested
- THEN the service MUST deny native execution without falling back to another backend.

### Requirement: Source intake preserves the exact authorized boundary

r[molten.pure_net.source_intake] Molten MUST import the exact authorized Noble revision through its repository-owned mechanism, MUST preserve and identify upstream blobs and consumer-owned changes separately, and MUST retain the exact-revision permission/notice and reproducible normalized manifests. It MUST NOT redistribute excluded third-party proofs/test suites or claim an upstream-wide license grant.

#### Scenario: The approved source-only selection matches
- GIVEN Noble `f7af9e972aa2968d22978794a667d267cf2745e8` and the recorded operator grant
- WHEN the reviewed snapshot is imported or checked
- THEN all selected and derived files MUST match the generated provenance lock.

#### Scenario: Source or derived bytes drift
- GIVEN a changed upstream blob, undeclared file or stale derived manifest
- WHEN source admission is checked
- THEN the checker MUST fail closed.

### Requirement: Every semantic and executable identity is bound independently

r[molten.pure_net.identity] Molten MUST bind checked source/program, typed IR when present, compiled net, rules, wrapping-I64 arithmetic, ordered input, lowerer, reducer/consumer overlay, readback, inets reference, canonicalization, manifest, limits, executable bytes and selected dependency closure. It MUST recompute authoritative identities from canonical Preserves and MUST NOT treat identity/checker success as execution authority.

#### Scenario: Full materialization matches
- GIVEN exact admitted artifact/input bytes and all matching identities
- WHEN admission runs
- THEN the service MAY proceed and MUST retain the complete binding in the receipt.

#### Scenario: A bound identity differs
- GIVEN wrong program, IR, net, rules, arithmetic, input, executable or closure
- WHEN admission runs
- THEN calculation MUST be denied before semantic reduction.

### Requirement: The authoritative producer owns the admitted pure subset

r[molten.pure_net.producer] Molten MUST consume the corrected Noble restricted pure contract and real checker/lowerer/validator/readback. It MUST reject unsupported operations/effects, malformed or open boundaries, cycles outside the admitted subset, mismatched slots/rules and forged source correspondence. Arithmetic/output expectations MUST originate in the independent source evaluator.

#### Scenario: Corrected source and backend agree
- GIVEN frozen operand-order, complete-output-order, wrapping I64 boundary, sharing/fanout and erase/discard fixtures
- WHEN source reference and sequential backend execute
- THEN their complete ordered values MUST agree under the exact arithmetic contract.

#### Scenario: A caller supplies an unsupported or forged program
- GIVEN malformed/open/cyclic source/net or a claimed net not produced from the checked source
- WHEN admission runs
- THEN the service MUST reject it without widening the source language.

### Requirement: Quanta retain one actual calculation and cumulative work

r[molten.pure_net.quanta] Molten MUST start with deterministic sequential reduction, retain the same arena/queue/counters across bounded local quanta, and bound per-turn work independently of total work. One whole evaluation is the scheduling unit. It MUST NOT rerun from input, reset cumulative budgets, or perform a durable transaction/network/consensus operation per rewrite.

#### Scenario: A calculation yields repeatedly
- GIVEN a program needing more than one quantum
- WHEN the host schedules successive quanta
- THEN it MUST resume the retained calculation, preserve total consumption and allow an unrelated runnable task to progress.

#### Scenario: The cumulative limit is reached after yielding
- GIVEN work spread over multiple quanta
- WHEN the total budget is exhausted
- THEN the service MUST return the corresponding exhaustion instead of receiving fresh fuel.

### Requirement: Every admitted work and retention dimension is finite

r[molten.pure_net.bounds] Molten MUST enforce finite decoding/source nesting, checking/lowering/validation, live nodes/ports/wires/bytes, allocation work, active queue, reductions, per-turn work, output/readback, retained observations, concurrent evaluations and aggregate attempt-work limits. Setup/readback work MUST be accounted separately from semantic interactions. Trusted native execution MUST retain its declared process-limit observations and MUST NOT claim sandboxing or equate wall time with semantic fuel.

#### Scenario: Work reaches an admitted boundary
- GIVEN a workload within each manifest bound
- WHEN work reaches but does not exceed a boundary
- THEN it MAY continue under the remaining independently bounded resources.

#### Scenario: A dimension is exceeded
- GIVEN an oversized decoder/graph/queue/memory/readback/observation/concurrency/retry requirement
- WHEN the relevant bound is checked or reached
- THEN the service MUST reject or stop with its typed outcome and MUST release a stopped evaluation's permit without monopolizing the host.

### Requirement: Terminal outcomes distinguish semantic and shell failures

r[molten.pure_net.outcomes] Molten MUST distinguish completed ordered value, malformed/unsupported input, stuck/incomplete output, reduction/node/queue/memory/readback/observation/attempt exhaustion, cancellation, worker interruption and publication uncertainty. An empty active queue alone MUST NOT imply source-program completion.

#### Scenario: Readback proves complete ordered output
- GIVEN an irreducible net with complete admitted outputs
- WHEN bounded readback succeeds
- THEN the service MAY report completed value with its full binding.

#### Scenario: No active pair remains but output is incomplete
- GIVEN an irreducible net with missing or invalid outputs
- WHEN readback runs
- THEN the service MUST report stuck/incomplete rather than successful evaluation.

### Requirement: Existing accepted-attempt state remains authoritative

r[molten.pure_net.lifecycle] Molten MUST reuse the selected composition's acceptance/begin/finish, capacity reservation, cancellation tombstone, output-copy and terminal-publication rules. Duplicate submission/readback MUST reconcile the same logical operation. Retries MUST use its existing stopped-attempt/epoch policy and current grants. Unknown commit/publication MUST retain ownership and MUST NOT grant duplicate execution or authoritative completion.

#### Scenario: A duplicate or uncertain reply is observed
- GIVEN an existing logical operation or unknown commit/publication
- WHEN a caller resubmits or reconciles
- THEN the service MUST return/reconcile the same durable operation without minting a second launch or completion.

#### Scenario: A stopped failed attempt is eligible for retry
- GIVEN confirmed stopped failure, remaining bounded attempts and current authorization
- WHEN retry is selected
- THEN the existing policy MAY mint the next fenced attempt without replacing any previously accepted output.

### Requirement: Current authority and cancellation fence successful publication

r[molten.pure_net.authority] Molten MUST check current policy, manifest/program grants and revocations, namespace/generation, limits and cancellation before start and before admitting a successful result. Revocation MUST NOT prevent stop/reconciliation. Latched cancellation, stale generations and uncertain publication MUST NOT become current success.

#### Scenario: Authority is revoked or cancellation wins
- GIVEN revoked authority, stale generation, cancelled-before-start or a completion racing cancellation
- WHEN execution/publication is attempted
- THEN success MUST be denied or replaced by the stopped cancelled outcome without releasing ownership early.

#### Scenario: A stopped failure needs reconciliation after revocation
- GIVEN revoked execution authority and a genuine stopped failure
- WHEN reconciliation runs
- THEN the service MUST allow the bounded stopped record to be retained without authorizing new calculation.

### Requirement: Reopen follows actual exclusive worker-process exit

r[molten.pure_net.recovery] Molten MUST keep all pure-net calculation process-owned and retain the exclusive durable-store lifetime through blocking work. Recovery MAY terminalize interrupted work only after exclusive open proves the old owner cannot still execute. Tests MUST kill and wait/reopen actual workers around acceptance, begin, calculation completion, result storage, terminal commit and acknowledgement, and inspect authoritative durable records.

#### Scenario: A worker exits at a lifecycle boundary
- GIVEN an actual killed and reaped worker
- WHEN the store reopens exclusively and reconciliation runs
- THEN nonterminal work MUST become typed interruption, retained terminal/output records MUST survive, and at most one authoritative completion MAY be published.

#### Scenario: An old process still owns calculation or a reply is merely missing
- GIVEN a retained store owner or unproved exit
- WHEN another caller requests recovery/retry
- THEN the service MUST NOT infer stopped execution or acknowledge nonexistent durable success.

### Requirement: Declared semantic observations replay canonically

r[molten.pure_net.replay] Identical complete immutable bindings MUST reproduce the declared canonical semantic trace/outcome under the sequential profile. The contract MUST enumerate reproducible fields and MUST exclude elapsed time, allocation addresses, process IDs and host-specific diagnostics from semantic identity.

#### Scenario: Full bindings repeat
- GIVEN the same program/net/input/semantics/runtime/limits/manifest/closure binding
- WHEN independent deterministic executions run
- THEN their declared semantic trace and outcome identities MUST agree.

#### Scenario: Only diagnostics or an actual semantic binding changes
- GIVEN changed timing/process diagnostics or changed semantic input/limits
- WHEN receipts are projected
- THEN diagnostics MUST NOT change semantic identity while semantic-binding changes MUST remain distinguishable.

### Requirement: Frozen matched baselines preserve unfavorable results

r[molten.pure_net.baseline] Molten MUST retain representative frozen native/reference service measurements under matching semantics and durability: end-to-end and setup/lowering/decoding/validation/reduction/readback costs, interactions, peak live memory, allocations and retry cost. It MUST distinguish logical counters from whole-process instrumentation and MUST preserve slower/no-benefit results without promoting them to performance/adoption claims.

#### Scenario: A matched baseline runs
- GIVEN frozen exact workloads and matching grants/durability
- WHEN native and independent reference paths are measured
- THEN the retained record MUST include the cohort, measurements, retry observations and instrument limitations.

#### Scenario: Native execution is slower or observations are narrower
- GIVEN slower results or instrumentation measuring only a named memory/allocation domain
- WHEN results are recorded
- THEN they MUST remain visible and MUST NOT be renamed as speedup or whole-process evidence.

### Requirement: Requirement evidence retains its actual execution class

r[molten.pure_net.evidence] Molten MUST update applicable Cairn requirements, traceability, architecture/operator documentation and evidence mapping. Records MUST distinguish authored tests, deterministic simulations/executions, actual process restarts and later cross-host runs; must identify the maintenance owner and repeatable commands; and MUST NOT substitute lifecycle state or logs for canonical execution authority.

#### Scenario: Local implementation and process evidence is retained
- GIVEN actual focused service and restart execution
- WHEN review/sync/archive occurs
- THEN each claim MUST link the corresponding bounded evidence and repeatable command.

#### Scenario: A cross-host/parallel/checkpoint claim lacks its own qualification
- GIVEN only sequential local evidence
- WHEN a broader claim is requested
- THEN it MUST remain gated and MUST NOT inherit the local evidence class.
