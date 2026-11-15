# Verification handoff — issue #18

## Execution status

No build, test, formatter, linter, source gate, lifecycle gate, daemon, VM or restart fixture was executed by this slice. The parent owns verification after all source edits settle. Authored tests are executable assertions, not recorded passing evidence.

The original task worktree reached ENOSPC during an atomic edit. The affected `src/world_snapshot/local.rs` was reread and remained complete at its pre-edit snapshot. The parent copied the work to `/dev/shm/aspen-open-issues-20261113` and explicitly authorized resumed writes there. The original worktree was not mutated afterward. The current source location and parent-designated temporary workspace are tmpfs; neither is NVMe or machine-loss durability evidence.

## Parent commands

Run in the recovered integration root, using the parent's selected dependency/toolchain environment and temporary directory only when directed:

```sh
cargo test --lib world_snapshot
cargo test --lib engine::dataspace
cargo test --test worldsnapshot -- --nocapture
```

Run the repository's required formatting, Clippy, primary/library Octet and strict Cairn rails once after all slices settle, in the governing gate order. Do not treat this handoff as those results. `tests/worldsnapshot.rs` is an auto-discovered Cargo integration target; no root manifest/dependency/lock/build-plan edits are required by this slice.

## Expected real artifacts and assertions

- Public capture observes the actual executing binary and writes eleven closed canonical component payloads plus a canonical descriptor; the descriptor alone is not observed compatibility.
- A separate restore process stages/readbacks actual typed ledger/Redb data and imports the existing `RuntimeState` owner before activation-last publication.
- A third readback process reconstructs the same canonical image, including pending messages, assertions/observers/facets, logical clock, RNG, effect sequence, full retained history and complete unresolved requests. Subsequent deterministic clock/random operations compare with the uninterrupted owner; unresolved requests remain undispatched.
- Positive execution emits canonical success receipt bytes; missing/corrupt state, supplied fake destination cohort, unsupported logical profile and valid opaque descriptors deny before runtime handle creation.
- Actual current-document/grant changes during reconstruction keep the attempt inactive. Actual receipt-path failures after commit retain an observable completed activation. A different active world cannot be replaced through this profile.
- Commit errors are conservatively uncertain and never automatically retried. No fixture currently claims to have induced a physical storage failure, machine-loss event or opaque/VM activation.

## Authority prerequisites and exclusions

An operator must select an existing capability-rooted destination with canonical configuration/identity bindings, independently authorize the exact signer, and separately provision the exact current local policy/capability/revocation/resource/adapter/generation/expiry document plus its descriptor/destination-bound signature. Fixture keys authorize only disposable experimental roots. Bundle possession and planning flags cannot supply that authority.

The selected executable profile is `runtime-spine-committed-turn-v1`. It does not restore the node daemon, system-extension supervisors, Wasm instances, the separate `fabric_time::SchedulerRuntime`, live handles, credentials or arbitrary task models. Opaque machine restore and VM clone realization remain guarded by their missing separately admitted exact adapters. Current-document observations are local operator authority, not a distributed revocation oracle or production source admission.

## Required parent integration

Add the concise runtime-spine restore ownership/boundary sentence supplied to the parent to `docs/architecture.md`; public prose identifies Aspen, while existing `molten` CLI/crate/schema/requirement names remain unchanged. Then retain actual verification evidence, complete the validation task and perform native Cairn review/sync/archive only after its gates pass. No unmet execution criterion is marked verified here.

## Parent execution (2026-11-13)

The parent subsequently executed the selected runtime-spine tests: 18 library
tests passed, including currentness/revocation fencing and post-activation
output failure. The public CLI integration passed 13 tests, including separate
capture/restore/readback processes, deterministic continuation, denied malformed
or revoked input, and explicit opaque-profile refusal. The existing dataspace
owner passed 41 focused tests. These observations supersede the original
slice-authoring status above, not the declared profile exclusions. Later
source-policy extraction is subject to the parent's final integrated checks;
no daemon, live scheduler, VM or machine-loss result is inferred.

