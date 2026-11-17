## Context

The #33 operator decision continues the opt-in integration despite the exact Noble producer's frozen performance NO-GO. The service must establish bounded deterministic execution and authoritative publication, not improved performance. The selected producer is Noble `f7af9e972aa2968d22978794a667d267cf2745e8`; the separately identified inets reference remains `01bd554a43504b58dc64c607f46dfe4acc7735cd` and grants neither language correspondence nor execution authority.

The current standalone batch composition already owns controller reservations, worker acceptance/begin/finish, cancellation tombstones, durable output copying and confirmed-process-exit restart. It currently admits only import-free in-process Wasm. A pure-net adapter therefore requires a separate grant, materialization contract, recovery justification and protocol version; injecting another ExecutionPort alone is insufficient.

## Decisions

### Decision: Install a separate workload in the existing authoritative composition

**Choice:** A versioned `pure-net-eval/v1` manifest is explicitly installed into standalone controller/worker roots. The task/request carrier selects a typed component or pure-net workload. The pure-net branch binds both manifest and program objects; content identity is never an execution grant. Existing policy keeps component grants separate from explicit manifest/program native grants. Bump the carrier/wire version and migrate every caller in one cutover.

**Rationale:** This is the smallest existing hosting path with an actual reopenable accepted-attempt ledger. The root node daemon/native loader and installed component executor have different persistence and callback contracts; adapting those would add unrelated service-host lifecycle work or falsely imply durable callback-value publication. Standalone deployment is explicit and is not node-daemon/system-tier activation or cross-host qualification.

### Decision: Preserve exact source and declare the consumer overlay

**Choice:** The repository-owned snapshot importer selects Onix-owned Rust and exact Cargo metadata for `noble-ngraph`, `noble-kernel` and `noble-contracts`, plus only the embedded `result.noble` asset covered by the subsequent exact-revision interactive grant. A versioned file filter enforces Rust extensions and explicit asset/metadata paths during both import and locked-tree checks; renamed originals cannot bypass it. Other non-Rust fixture programs remain excluded. Record both grants and third-party/proof exclusions. Preserve original upstream blobs and normalize manifests reproducibly. Any reducer observation/quantum overlay is Aspen-owned, declared in snapshot policy, and binds a separate backend revision rather than claiming unchanged upstream execution.

**Rationale:** The selected public tree lacks a project license declaration; the operator's asserted rights and explicit grant are a distinct prerequisite from the performance continuation decision. No upstream setting or license file is modified.

### Decision: Materialize and revalidate the producer's admitted subset

**Choice:** The pure producer bridge uses Noble's real parser/checker, corrected I64 graph lowerer, net validator, deterministic sequential reducer, independent source evaluator and complete ordered readback. Bound source/IR/net/input bytes and shape before work; reconstruct source-to-net correspondence before reduction rather than trusting a caller's checker record. Reject unsupported operations/effects, open or cyclic graphs, wrong slots/rules/arithmetic, malformed endpoints, incomplete outputs and mismatched identities. Inputs remain independently identified and ordered.

Bindings separately identify source/checked program, typed IR, compiled net, rules, wrapping-I64 semantics, ordered input, lowerer, reducer/overlay, readback schema, inets reference, canonicalization, executable bytes, dependency closure, manifest and resource limits. Recompute content/semantic identities from canonical Preserves, not JSON rendering, debug values or Rust layout.

### Decision: Retain one in-memory evaluation across cooperative quanta

**Choice:** One whole evaluation is the scheduling and retry unit. The sequential reducer retains its actual arena, queue and cumulative counters. Its declared observation hook offers a bounded handoff after each local quantum; the async adapter yields fairly before permitting the same calculation to continue. Setup, graph validation and readback have independent work budgets and observations. No per-rewrite durable record, RPC, consensus operation or restart-from-input quantum is introduced.

**Rationale:** Resetting the reducer on a yield would repeat work and reset limits. Parallel reduction and portable checkpoints remain distinct unadmitted profiles. Timing/deadline/cancellation are shell observations, not semantic interaction fuel.

### Decision: Bound native work explicitly; do not call it a sandbox

**Choice:** Reject oversized input before decoding; bound source nesting, graph/live-node/port/wire sizes, queue and arena allocation work, total interactions, per-turn work, output/readback and retained semantic observations. Bound active evaluations with a profile-specific permit; cap retry work conservatively through the existing attempt count and the manifest's checked per-attempt/aggregate budget. A stopped rejected/exhausted job releases its permit and cannot monopolize the runnable host.

Local turn work is bounded by `quantum_work + max_atomic_work`: indivisible frontend, decoder, sort and rewrite-plan validation/commit reservations are admitted before invocation. In particular, every selected rewrite reserves the size-dependent nested worklist/arena scans, bounded plan/check/apply/digest work and growth copies before entering its atomic mutation block. These conservative logical reservations are validation/total work, never semantic interactions or queue-pop counts.

Native execution is trusted in-process work with no application effect imports. Validate declared process-level memory/CPU limits at worker startup on supported Linux deployments, and retain the exclusive database owner in every blocking calculation closure. Logical arena/memory exhaustion and process interruption remain different observations; process-level limits do not become an instruction-metering or isolation claim.

### Decision: Keep fresh authority and publication distinct from pure calculation

**Choice:** Recheck policy identity, manifest/program revocation, namespace/generation, limits and cancellation before start and immediately before admitting successful output. An explicit publication action can deny success without preventing cancellation, stopped failure recording or reconciliation. The core's latched cancellation always wins over late success. Unknown persistence/publication retains ownership and is not a negative acknowledgement or a retry grant.

Typed outcomes distinguish completed ordered values; malformed/unsupported input; stuck/incomplete output; reduction/node/queue/memory/readback/observation/attempt exhaustion; cancellation; interruption; and publication uncertainty. Successful computation does not bypass the accepted worker record or controller output-copy/terminal transaction.

### Decision: Reuse exclusive-process recovery, with real boundary observations

**Choice:** Native evaluation is wholly owned by the worker process: no detached reducer/child and no external effect. Hold the database Arc through computation even if its async waiter is dropped. A new process may mark nonterminal records interrupted only after exclusive open; test runners kill and wait before reopening. New attempts use current grants and existing epoch/retry rules. Add bounded worker fault points around acceptance, begin, calculation completion, content storage, terminal commit and reply.

**Rationale:** The existing pure-Wasm recovery inference is not automatically valid for native execution. Keeping all pure calculation inside the same process and lifetime makes the same exclusive-lock/confirmed-exit argument applicable; a missing reply alone does not.

## Verification and retained evidence

- Preserve the existing component/controller/worker baselines and versioned carrier contracts.
- Keep positive/negative limits and authority controls together. Arithmetic expectations come from Noble's independent source evaluator, including subtraction operand order, complete output order, I64 min/max wrapping, sharing/fanout and erase/discard. Unsupported cycles are rejections, not manufactured nontermination.
- Compare repeated executions with identical complete bindings. Canonical semantic observations contain stable phase/work/outcome facts only; elapsed time, allocation addresses, process IDs and host diagnostics are excluded.
- Exercise the real CLI/service installation, exact materialization, result retrieval and receipt binding. Kill/reopen actual workers at each lifecycle boundary and inspect durable records/progress for one authoritative completion.
- Freeze representative net/reference cases under matching grants and durability. Record end-to-end latency; parsing/checking/lowering/decoding/validation/reduction/readback phase costs; interactions; peak logical live memory; externally observed process memory/allocations where available; and interrupted-attempt retry cost. Preserve slower/no-benefit measurements and distinguish logical allocation counters from whole-process allocation observations.
- Map requirement IDs to authored fixtures, deterministic executions and actual process-restart receipts. Run focused repository gates before sync/archive; lifecycle state is not behavioral evidence.

## Rejected Alternatives

- A second pure-net job/attempt store: duplicates the authoritative lifecycle and weakens reconciliation.
- Treating native bytes as a component, or allowing them through the component wildcard: silently expands authority and misstates recovery/resource contracts.
- A fresh reduction for each quantum: repeats work, changes trace and can evade cumulative limits.
- Full async/persistent net checkpointing or parallel reduction now: not authorized by sequential evidence and belongs behind #34/separate profile gates.
- Copying inets proofs/test suites or assuming GitHub visibility is a license: outside the exact source grant.

## Risks / Trade-offs

- The producer is a performance NO-GO. Integration may be slower; no optimization/adoption conclusion is predetermined.
- Native arithmetic is trusted consumer code, not guest isolation. Finite logical work and observed process caps do not prove universal termination or whole-host safety.
- The observation overlay must preserve the pinned reducer's sequential semantics and must remain separately attributable in provenance and differential checks.
- Conservative whole-attempt reservations and uncertainty handling sacrifice availability instead of authorizing duplicate work/publication.
- Local process tests do not qualify cross-host operation, production readiness, exact-once external effects or arbitrary Noble programs.
