# Review record: route explicitly admitted installed native effects

## Disposition

Eight read-only source reviews were performed against this worktree. None of them
ran the rail, a build, tests, a formatter, a lifecycle gate or a source gate. They
are source reviews, not runtime evidence, executed acceptance or release approval.
No human reviewer signed this record; it is the implementing worker's own record of
the source reviews it performed. Retained observations named below are quoted from
the change's `evidence/2026-11-01-rail-progress.md`, not re-observed here.

## 1. Retained control and ownership fences

- **Scope:** `tests/collaborationnative/driver/mod.rs` (`Node`, `Drop`, `finish`,
  `exited_child_cannot_supply_readiness`, `await_ready`), `driver/scenarios/replacement.rs`
  (`startup`, `fenced_preview`), `src/cli/ops/node/lifecycle/stdio.rs`,
  `src/cli/ops/node/parts/lifecycle/p001/body.rs`, `src/node/servicehost/installed.rs`.
- **Method:** read-only source review.
- **Findings:** `await_ready` samples the owned child before accepting any readiness
  text, so a log that outlives its process cannot supply readiness; the actual
  `exited_child_cannot_supply_readiness` test asserts that denial. `finish` closes the
  retained control (EOF), waits on an admitted `SupervisionDeadline`, requires a
  successful exit and then joins control; `Drop` closes control before killing and
  reaping its own child. `stdio::serve` treats `Input::End` as a shutdown signal,
  awaits the bound listener and always joins its reader; the reader uses cancellable
  forwarding so a full queue cannot block cancellation. Replacement startup accepts
  only an ordinary exit code 1 with no `node-service-host-ready-v1` line and joins
  its own control; its preview path rejects success, a signing request or any scope
  observation.
- **Resolution in this tree:** all of the above are present; no patch was required.
- **Residual risk:** readiness is still textual, mitigated only by the live-child
  check. Replacement evidence is single-voter and does not prove restored quorum,
  availability or future recovery.

## 2. Scope authority and currentness

- **Scope:** `src/collaborationscope/native/runtime/authority.rs` (`restage`,
  `observe_scope`, `observe_read`, `current_read`), `runtime/authority/verify.rs`,
  `runtime/live.rs` (`commit`, `wait_for_commit`), `runtime/authority/config.rs`,
  `runtime/worker.rs`.
- **Method:** read-only source review.
- **Findings:** `restage` first rejects a binding/effect whose port id, version,
  operation, schemas, target key or generation differs from the admitted runtime.
  It materializes the exact request bytes, checks the content identity and accounted
  byte count, parses the staged command and its proposal, and requires both refs to
  match. It then loads operator trust and the exact credential, performs a current
  linearizable read plus a separately reserved fresh probe, and requires equal
  service generation, consistency epoch, committed index, logical time, revocation
  reference and state. It re-reads the trust root after the round trip and stages
  through the actual Basalt/UCAN verifier with the revocation list and nonce ledger.
  A historical Decision, a source-trust fact or a previous staged snapshot never
  enters this path. `commit` waits for `proof::complete` and marks the worker
  uncertain on any failure. The observer path uses `observe_currentness`, constructs
  no `CurrentState` and consumes no read.
- **Resolution in this tree:** implemented as described; no patch was required.
- **Residual risk:** currentness is only as strong as the live quorum at read time.
  The fixture may freeze trust between its observation and the single protected
  delivery, and replacement proof is not a quorum-restoration proof.

## 3. Denial-classification safety

- **Scope:** `src/collaborationscope/native/runtime/authority/config.rs`
  (`read_required`, `classify`), `runtime/worker.rs` (`redacted_failure`),
  `collaborationscope/codec/canonical.rs`, `src/cli/ops/node/lifecycle/control.rs`,
  `tests/collaborationnative/driver/scenarios/admission.rs`.
- **Method:** read-only source review.
- **Findings:** the classified reader maps an absent value to `missing`, a
  non-regular leaf and canonical parse/schema failure to `domain`, and a failed
  entry observation or read to `uncertain`. `classify` passes an existing exact
  `collaboration-scope/<code>` failure through and maps every other `Err` to
  `domain`; no arm converts an `Err` into success. Worker redaction uses an exact
  `matches!` over the closed canonical vocabulary, so a string that merely begins
  with a canonical code, or carries backend text, is redacted. The CLI accepts only
  the closed set, and the admission fixture requires exact `missing`/`stale` codes
  for those modes while the malformed mode requires a non-success without a fixed
  code.
- **Resolution in this tree:** implemented; the two redaction regressions cover
  canonical preservation and suffix/backend redaction.
- **Residual risk:** `uncertain` is still an observable denial class, and the review
  cannot prove that every backend failure in every environment maps deterministically.

## 4. Diagnostic retry-loop safety

- **Scope:** `tests/collaborationnative/driver/scope.rs` (`select`),
  `src/collaborationscope/native/runtime/diagnostic.rs` (`Observer::observe`),
  `runtime/authority.rs` (`observe_scope`, `observe_read`), `runtime/worker.rs`
  (`Message::Observe` handling).
- **Method:** read-only source review.
- **Findings:** the metadata selector retries only the `scope-observe` operation and
  is bounded by 1800 seconds of admitted fixture-clock time, reporting its round
  count; it never delivers, reserves a nonce or supplies `CurrentState`. The
  observer uses a non-blocking `try_send` into the bounded worker queue and a
  bounded `recv_timeout` on a one-slot reply, mapping failure to `uncertain`.
  `observe_scope` performs a diagnostic read, derives currentness without
  constructing an authority value, and re-checks the trust reference; the worker
  refuses observation once the runtime is uncertain.
  An earlier fixed round cap was replaced by this elapsed bound, and the attempted
  `std::time::Instant` was rejected by the source gate and replaced with the
  admitted wall-clock helper.
- **Resolution in this tree:** implemented; the delivery, authority and currentness
  paths are unchanged by the diagnostic loop.
- **Residual risk:** diagnostic retries can consume up to the 30-minute bound and
  contend with real reads. They remain data, not mutation or completion authority.

## 5. Negative-case assertion audit

- **Scope:** `driver/scenarios/unconfigured.rs`, `admission.rs`, `uncommitted.rs`,
  `completion.rs`, `replacement.rs`, `driver/status/commit.rs`,
  `driver/scenarios/admission/readback.rs`, `src/node/servicehosting/work.rs`.
- **Method:** read-only source review.
- **Findings:**
  - *Receipt versus completion:* `Host::deliver` persists `Unknown` before dispatch
    and promotes work only for `NotRequested` with no approved effects or
    `Completed` covering every approved effect. The unconfigured case therefore
    requires exact, unique `Unknown` work plus its real Request/Ingress receipt; it
    no longer requires the invalid absence of a receipt.
  - *Terminal ref versus cache address:* `commit.rs::find_completion` selects a
    unique cached `system-extension-effect-completion-v2` whose request and output
    refs match the native terminal effect, whose native operation identity, binding,
    generation and terminal ref match, and which links the exact native Request
    ingress receipt. `inline` validates that the provider output bytes hash to the
    terminal ref. The Message callback receipt is the readiness/delivery boundary,
    not the wrapper's cache address.
  - *Opaque cache leaves:* the scan first validates the immutable address, skips
    non-Preserves bytes and non-regular or non-`.preserves` leaves, and denies more
    than 1024 entries.
  - *Store reopen ordering:* owners are closed before durable journals are read;
    the queued case retains the delivery result but calls `finish` before
    inspection; inventory compares only `raft-log-mutation-v1` and
    `raft-commit-boundary-v1` records; a killed-store readback is not claimed.
  - The earlier audit corrections remain: whole-log comparison was narrowed to
    protected records, malformed regular leaves are skipped, and predecessor
    retention accepts either an unresolved Unknown effect or a produced-but-
    unconsumed effect while forbidding its consumption.
- **Resolution in this tree:** all corrections are present; no further defect was
  found in these assertions.
- **Residual risk:** `is_retry_permitted` is hardcoded false product-wide, so that
  conjunct is consistent but cannot fail. The predicted blocked-leaf publication
  and replacement activation shape can only be settled by execution.

## 6. Completion-failure sequence

- **Scope:** `driver/scenarios/completion.rs`, `crates/molten-core/src/system_extension/dispatch.rs`
  (`plan_callback_dispatch`), `src/system_extension/parts/host/p001/body.rs`
  (`dispatch`), `src/system_extension/native_host/parts/service/p001/body.rs`
  (`deliver_effect_completion`).
- **Method:** read-only source review.
- **Findings:** an admitted dispatch increments the invocation sequence by exactly
  one; a resource decision with `invocation: None` returns `Deferred` and does not
  update it. Relative to the observed status base, the fixture's Request dispatch
  is the first invocation and the completion `Message` dispatch is the next, so
  the expected bridge sequence is `observed + 2`, with the request ref in the
  optional field. The admitted one-callback/4 MiB profile schedules that
  completion. The fixture creates a directory at the predicted future value leaf;
  publication then fails before writing it, and `has_failed_publication` requires
  the exact terminal `ValuePublication` rejection for that future identity.
  `require_post_effect_failure` additionally requires the terminal provider
  effect, the cached completion, the fresh proposal, `Unknown` work, unchanged
  state and an unconsumed effect. A wrong prediction leaves the obstruction unused
  and fails the case rather than passing.
- **Resolution in this tree:** implemented; all owners close cleanly because the
  obstruction is a definite pre-publication rejection, so the case claims no crash.
- **Residual risk:** the `+2` prediction depends on the admitted manifest and
  profile; any intervening admitted dispatch would fail closed. No process-loss
  evidence is claimed.

## 7. Queued-case framing and durability

- **Scope:** `tests/collaborationnative/producer/transport/sizing.rs`
  (`uncommitted_frame_budget`), `producer/mechanisms.rs`, `driver/scenarios/uncommitted.rs`,
  `src/fabric_consistency/raft/durability/mod.rs` (`persist_entries`, `flush_log`).
- **Method:** read-only source review.
- **Findings:** the separating budget is the largest encoded election, read or
  append-control frame with maximum-width numerics, and the first single-entry
  `AppendEntries` with minimum indices is strictly larger, so it cannot fit the
  admitted frame bound. The owning leader persists `raft-log-mutation-v1` at
  `Buffered` and then flushes at `MachineLoss` before any send; without a quorum no
  `raft-commit-boundary-v1` can form, and delivery returns a normal control error.
  The fixture closes the owner before reading durable state, keeps only protected
  records, invalidates an exact entry on a later truncation, rejects a commit
  boundary at that index, and requires no application scope state while the owning
  replica retains the proposal.
- **Resolution in this tree:** implemented; the token/message sizing is fixture
  resource-policy construction, not evidence of a live queue.
- **Residual risk:** the separation argument is analytic; channel timing, election
  races and actual append rejection can only be shown by execution. No owner is
  killed for this case.

## 8. Evidence-coverage mapping

- **Scope:** merged `.cairn/specs/system-extension/spec.md`,
  `tests/collaborationnative/driver/mod.rs`, the new
  `tests/collaborationnative/driver/installation.rs`,
  `driver/scenarios/`, `tests/evidence-matrix.ncl`, and the rail-progress record.
- **Method:** read-only source review plus comparison against the retained
  observation names in `evidence/2026-11-01-rail-progress.md`; the observations
  were not re-run.
- **Findings:** the six `native_effect_*` requirements now appear once each in the
  base specification. The ignored baseline test carries all six `r[verify]` tags and
  drives the positive baseline and the six negative cases. The new ignored
  `installed_native_effect_rejects_a_mismatched_installation` and
  `installed_native_effect_startup_failure_publishes_no_readiness` tests carry the
  installation and lifetime negative tags. The mismatched fixture keeps a canonically
  encoded configuration but replaces one binding's `installation_ref`, then requires
  no readiness, ordinary exit 1, and absent scope-application and Raft namespaces.
  The startup-failure fixture removes the operator scope trust, then requires the
  same refusal plus an absent native journal store. Neither fixture claims a route,
  commit or completion. The rail record names retained observations for the
  baseline and all six negative cases, produced from product bytes stated to be
  identical to the final admitted tree; those observations are runtime evidence in
  that record, not evidence supplied by this review.
- **Resolution in this tree:** the evidence matrix now maps each requirement to a
  positive and a negative integration entry pointing at the tagged test in
  `driver/mod.rs` or `driver/installation.rs`.
- **Residual risk:** the two new `installation.rs` cases are ignored and have not
  been executed; the mismatched-installation arm previously had no dedicated
  fixture. Matrix artifact and receipt refs are authoring placeholders regenerated
  by local verification receipts, not measured identities from this review.
