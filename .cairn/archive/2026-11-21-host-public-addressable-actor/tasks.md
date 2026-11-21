# Tasks

- [x] [serial] Compose real current-placement observation and native lifecycle admission without another ledger. r[molten.addressable_actor.public_native_consumer]
- [x] [serial] Bind coordination-delivery claims/acknowledgements and actual durable semantic outcomes. r[molten.addressable_actor.public_native_consumer] r[molten.addressable_actor.public_result]
- [x] [serial] Reconcile native drained sleep, checkpoint identity, restart and bounded survival evidence. r[molten.addressable_actor.public_native_consumer]
- [x] [serial] Expose consumer outcomes that cannot mistake lifecycle progress for completion; author durable, buffered, stale, denied, duplicate and unknown coverage. r[molten.addressable_actor.public_result]
- [x] [serial] Execute the pinned init/install/request/result/checkpoint/sleep/process-exit/restore/wake flow and live negatives with exact source/config/evidence bindings. r[molten.addressable_actor.public_native_consumer] r[molten.addressable_actor.public_result]

Implementation and execution qualification remain separate. The positive and negative native cohorts below were executed under an explicitly experimental source authority; this does not grant production admission, machine-loss durability, cluster truth, or release eligibility.

Execution record (2026-11-20): the [experimental signed counter flow](evidence/2026-11-20-signed-counter-flow/run.json) binds the rebuilt `molten-node-source-binding-v1` output, trust selection, experimental operator policy, and four independently authored cohorts. The positive flow completed all 24 commands, including init, installs, placement activation, checkpoint/sleep, request/result with an early `unknown`, process exit/restore/wake, duplicate replay, and final semantic bytes. The [SIGKILL cohort](evidence/2026-11-20-signed-counter-flow/negative1.json) retained post-semantic `unknown` without retry; the [boundary cohort](evidence/2026-11-20-signed-counter-flow/boundary1.json) denied the wrong key, stale generation, substituted placement, cited revocation, and four unsupported survival classes; the [ordering cohort](evidence/2026-11-20-signed-counter-flow/ordering1.json) proved older-ready/new-request progress without an unrelated claim or effect. The signing key was explicitly experimental, not an independently authorized production operator key.

Parent observation (2026-11-13): `cargo test -p molten --lib addressable_actor`
passed all 47 selected tests. The retained-ready regressions use the actual
delivery Local CAS/read/reopen path at the same and advanced logical tick;
proof-reference controls accept reordering/additional evidence and reject each
missing exact native reference. Source review found no remaining blocking
defect in these repairs. The native attempted-effect classification compiled.
The signed public command flow and its negative cohorts are now executed in the
2026-11-20 record above; independently source-admitted Installed Owner/Session
failure injection remains unexecuted and is not claimed.

The broader native-host suite exposed a missing Drained process-loss transition.
After restoring that fenced lifecycle path, `cargo test --test nativesystemextension`
passed 26 tests with the two independently signed live-cohort tests still
ignored; `cargo test -p molten-core system_extension` passed all 57 selected
tests. Recovery still denies without a checkpoint, keeps missing-materialization
ingress closed, and persists its new incarnation before callback intent.
