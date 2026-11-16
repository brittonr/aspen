# Durable batch progress v1

The standalone controller owns the accepted-output projection, not worker messages or UI counters. It exposes existing admitted Iroh RPC operations ProgressSnapshot, ProgressRead and ProgressArtifact. The first two are bounded observations. Disconnect has no cancellation effect; CancelRun remains a separate admitted request, and Cancelling never proves executor stop.

## Durable crash protocol

Output bytes are verified and persisted in the controller content table before successful task state. One immediate-durability Redb transaction commits canonical controller state and a small canonical progress owner; accepted-output/terminal transitions also append the journal and advance its position in that same transaction. State-only transitions keep journal bytes unchanged. A progress event cannot be committed independently of its state transition. Output persistence before that transaction can leave an unreferenced object, never an accepted completion. A failed/unknown commit requires ordinary load/reconciliation; it does not grant a second dispatch.

Before task/progress commit: the accepted worker observation is reconciled again. After task/progress commit but before notification: the same canonical event identity is replayed. There is no separate durable notification acknowledgement or deletion operation. A client advances its inclusive next-event cursor only after applying a whole received page. Lost replies may repeat records; the stable event_ref identifies the transition once. Keep the projection and last applied cursor together in the consumer's durable owner; a Rats view consumes that supplied projection and emits intents rather than doing I/O itself.

Existing databases bootstrap already accepted outputs and terminal runs in a single exclusive-owner transaction. A partial canonical/cache pair is corruption, not an empty history. Subsequent state revisions derive deltas from the exact previous state. Accepted records bind cluster, generation, run, job spec, task, assignment, attempt epoch, request, accepted result and content identity. Terminal success requires the entire authoritative job task set; failed/cancelled terminal records retain the accepted subset explicitly.

Public `event_ref` is `blake3:` plus the existing codec identity of `(cursor, payload_ref, committed_at_tick)` under `batch-progress.event.v1`; `payload_ref` uses `batch-progress.payload.v1`. These are canonical packed Preserves identities. The reusable core's internal index identity is reconstructed from the same position/payload and is not substituted for the public batch identity.

`controller-progress-owner.v1` binds the current state reference, the state reference at the last projected transition, and both exact journal canonical/reconstruction digests. State-only commits verify that binding and advance only its current-state reference; they do not deserialize, rehash event identities or rewrite unchanged retained history. New projection commits and recovery still perform full journal validation. Snapshots report the current state reference, not the older last-projection reference.

This is an internal forward storage migration, not a public cursor/event identity change. An existing monolithic `controller-progress.v1` pair is upgraded only after proving it binds the exact current state; existing state without any journal still uses accepted-state bootstrap. Partial/crossed owner or journal records fail closed. Older binaries must not write migrated stores; downgrade interoperability is not promised and rollback requires a matching pre-migration database snapshot.

## Snapshot and retention

A selected-run snapshot and watermark come from one Redb MVCC read transaction. It distinguishes pending, dispatching, running, uncertain, accepted, failed, blocked and cancelled tasks. Later pages are global to this cluster/generation service; consumers filter exact run/spec identities, never task names alone. The initial event contract covers accepted outputs and terminal runs, not every running/pending phase change.

The Aspen batch profile retains at most 256 events, below the shared committed-stream core ceiling; ProgressRead returns 1..64 events and rejects foreign services or stale epochs explicitly. Retention overflow returns ResnapshotRequired, never a silent cursor jump. Pages/snapshots are limited to 256KiB encoded replies. Polling has no server-side subscriber queue. Existing ingress permits at most eight in-flight bounded RPCs, one request per connection with a finite deadline. Artifact fetches are one object per request, at most 4MiB raw bytes, under the same admitted transport and ingress bounds. No automatic fetch/retry loop is hidden in the view.

Progress retention does not promise permanent content storage. The current content owner has no GC and retains at most 4096 objects/1GiB. Reaching bounds fails closed. Future GC must protect current accepted output roots or retain explicit artifact-gap semantics. ProgressArtifact returns ArtifactGap for missing/expired content, distinct from missing retained history. Resnapshot does not restore missing bytes. Other corruption/identity/authority failures remain errors rather than fabricated gaps.

## Consumer commands

Run from the repository checkout in the pinned Nix shell. BIN is the built molten-orchestrator and TARGET is the enrolled controller key@direct-IP:UDP-port.

    $BIN progress --root operator --target "$TARGET" --run RUN_ID > snapshot.json
    jq .watermark snapshot.json > cursor.json
    $BIN progress --root operator --target "$TARGET" --cursor cursor.json --limit 64 > page.json
    jq .Events.next page.json > next-cursor.json
    $BIN get --root operator --target "$TARGET" --id ACCEPTED_OUTPUT_ID --out tile.bin

Handle Events, ResnapshotRequired, StaleEpoch and ForeignService as distinct outcomes. Only apply a page and its cursor after validating the complete response. Never write a null cursor after a gap. Persist actual output references; full pixel buffers are not progress event records.

## Shared source and qualification

The adapter source-mounts crates/molten-core/src/committed_stream, including its cursor-retention fixes. It is the same repository-owned implementation, not a copied fork or a second ledger. The standalone package source closure therefore includes both pilots/orchestration and that owning core tree. A directory-only pilot source package is unsupported. default.nix declares the complete closure; it uses the standalone reviewed Cargo.lock and the repository-locked Nix providers.

    nix-build pilots/orchestration/default.nix
    nix-shell pilots/orchestration/render/shell.nix --run 'bash pilots/orchestration/scripts/progress-conformance.sh'

Conformance process faults are explicitly opt-in through serve --crash-at POINT and exit 86 abruptly without graceful drain or synthesized stop. Supported points: before-dispatch, after-dispatch, before-output-persist, after-output-persist, before-terminal-commit, after-terminal-commit, before-progress-commit, after-progress-commit and before-progress-reply. Task and progress commits are one transaction; no fictitious between-store atomic boundary is advertised. The lost-reply fixture exercises the notification-acknowledgement uncertainty.

`after-progress-ack` observes transport peer closure after publishing a nonempty progress reply. It is deliberately not proof that the consumer durably applied the page; the consumer still persists projection/cursor together. The acknowledgement fixture crashes there and replays unchanged event identities after restart. No receipt or closure deletes retained history.

The exact-source campaign was executed once with `bash pilots/orchestration/scripts/progress-conformance.sh /dev/shm/aspen-open-issues-20261113/target/source-gate-20261113/progress-exact-source`. Evidence is retained at `target/source-gate-20261113/progress-exact-source/run.4Myxlx/`: invocation, exit, environment, `source.tar`, `source.blake3`, storage/process/retention logs and bounded `public-observations/`; the parent directory also retains `execution-environment.json` with the pinned environment and exact command. Source archive BLAKE3: `b1003f5f6ff717314f9e67ec4a204c1ea83f878ea55b5d650d162b9dd3d66d43`. Storage passed 4/4 and the generic process selection passed 5/5 (three renderer cases ignored); the separately selected sixth generic case, retention overflow/resnapshot, failed its observation deadline after 316.03s with the fourth run still Running (56 Succeeded, 2 Active, 6 Pending). Overall exit was 101. The retained `public-observations/.tmpVvszxs/wait-*` diagnostics and failed retention log remain evidence, not a retention qualification; no retry or runtime change was made.

This run binds actual outcomes to the archived source, not to subsequent documentation edits, and does not replace prior failure history or the separate package closure build. It used the retained pinned environment, a warm same-host Cargo target, two build jobs, incremental disabled and dev/test debug disabled. Same-host process results never qualify a cold public build, two-host direct-LAN deployment, machine loss, hardware or release. Private keys and databases are not public evidence.

After moving selected-run status filtering into the RPC v2 controller reply, the unchanged exact-source campaign completed successfully. The new retained run is `target/source-gate-20261113/progress-scoped-status/run.hfieMi/`, with source archive BLAKE3 `fecf97428b93511ced1910be801cc3d1c954cace2d453a9169a15d4db34cedd7` and `exit=0`: four storage cases, five generic process cases, and the separately selected sixth retention/resnapshot case passed. The latter completed in 333.72 seconds overall across four 64-task runs; each original 120-second per-run observation deadline remained unchanged. Three component-dependent renderer cases were not selected by this campaign. No workload, task, fuel, memory, output, retention or admission bound was relaxed.

The original failed campaign and its diagnostics remain retained. This passing run establishes only the named cached same-host source/profile observations; it does not establish a performance improvement, package-closure build, cross-host durability or release qualification.

Actual scene component boundaries, active worker loss plus controller restart and pending/active render cancellation are selected explicitly by `scripts/render-faults.sh` after the real component build. Its ignored Cargo cases fail if the required component is not supplied; the script provides the exact artifact and records outcomes. The generic progress campaign independently exercises fuel/deadline/output exhaustion, stale attempt fencing, peer denial and the 256-event retention overflow.

The 2026-11-16 [retained ordinary capture](../../../evidence/progress-controls-2026-11-16/initial-observation.json) passed five controls but its 64-task retention run exceeded the existing observation deadline while still making progress. That failed source/log and selected-run/controller-state diagnostics remain unchanged. The [exact corrected gap case](../../../evidence/progress-controls-2026-11-16/corrected-gap-observation.json) passed separately: completed 16-task jobs fill the same 256-event window, the saved cursor requires resnapshot, and a fresh consistent snapshot preserves accepted outputs without replaying events after its watermark. Its finite number of runs derives from the retention bound; no event-capacity, per-run deadline, per-task limit or admission rule was relaxed. This is a retention-conformance fixture, not a maximum-size-job throughput promise. Batch/progress maintainers own repetition and the distinction between these two source-bound observations.
